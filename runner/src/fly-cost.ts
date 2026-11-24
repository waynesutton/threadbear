// What a cloud runner's boxes have cost this month, worked out from Fly's own
// record of each Machine's starts and stops. Fly has no billing API (its
// docs point at the dashboard; the old GraphQL `billables` query was removed
// in 2023), but the Machines API keeps every Machine's events, deleted ones
// too with include_deleted, and the guest size to price them. No agent and no
// tokens: the runner reads it when Settings asks (cloud.cost).

// Fly's list prices, USD, for its cheapest regions (iad, ewr), from
// docs.fly.io/about/pricing (24 Nov 2026). A shared vCPU comes with 256 MB;
// more RAM is per GB. A month is 30 days. Other regions cost a little more.
export const FLY_PRICES = {
  sharedCpuMonth: 2.19,
  ramGbMonth: 6.0,
};
const MONTH_S = 30 * 24 * 3600;

export type FlyGuest = { cpu_kind?: string; cpus?: number; memory_mb?: number };

export type FlyMachine = {
  id: string;
  state?: string;
  config?: { guest?: FlyGuest };
  events?: { type?: string; status?: string; timestamp?: number }[];
};

export type CloudCost = {
  app: string;
  // UTC, the first of this month.
  monthStart: number;
  asOf: number;
  usd: number;
  // Hours the boxes ran this month, all of them together.
  hours: number;
  // Machines that ran this month (test boxes included).
  boxes: number;
};

/** A started Machine's price per second at list price, or 0 for a kind it doesn't know. */
export function perSecond(guest: FlyGuest | undefined): number {
  const cpus = guest?.cpus ?? 1;
  const mb = guest?.memory_mb ?? 256 * cpus;
  if ((guest?.cpu_kind ?? 'shared') !== 'shared') return 0;
  const month = cpus * FLY_PRICES.sharedCpuMonth + (Math.max(0, mb - 256 * cpus) / 1024) * FLY_PRICES.ramGbMonth;
  return month / MONTH_S;
}

const STOPS = new Set(['stopped', 'suspended', 'destroying', 'destroyed', 'failed', 'replacing']);

/** Seconds a Machine spent started between from and to, from its events. */
export function startedSeconds(m: FlyMachine, from: number, to: number): number {
  const events = [...(m.events ?? [])].filter((e) => typeof e.timestamp === 'number').sort((a, b) => a.timestamp! - b.timestamp!);
  let total = 0;
  let since: number | undefined;
  for (const e of events) {
    if (e.status === 'started' && since === undefined) since = e.timestamp!;
    else if (since !== undefined && STOPS.has(e.status ?? '')) {
      total += overlap(since, e.timestamp!, from, to);
      since = undefined;
    }
  }
  // Still going: to now, if Fly says it's started.
  if (since !== undefined && m.state === 'started') total += overlap(since, to, from, to);
  return total / 1000;
}

function overlap(a: number, b: number, from: number, to: number): number {
  return Math.max(0, Math.min(b, to) - Math.max(a, from));
}

export function monthStart(now: number): number {
  const d = new Date(now);
  return Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), 1);
}

export function estimate(app: string, machines: FlyMachine[], now: number): CloudCost {
  const from = monthStart(now);
  let usd = 0;
  let secs = 0;
  let boxes = 0;
  for (const m of machines) {
    const s = startedSeconds(m, from, now);
    if (s <= 0) continue;
    boxes++;
    secs += s;
    usd += s * perSecond(m.config?.guest);
  }
  return { app, monthStart: from, asOf: now, usd: Math.round(usd * 100) / 100, hours: Math.round((secs / 3600) * 10) / 10, boxes };
}
