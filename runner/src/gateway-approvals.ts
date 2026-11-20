// Gateway approvals in the thread (prds/gateway-approvals-in-threadbear.md).
//
// When the Connectors gateway (plugbear) holds a call for approval (a Slack
// post, an email), it used to be approved only on its own page in a
// browser. A gateway on this Mac with /api/approvals (local mode, its
// /health says `approvalsApi`) lets the runner put that approval on a card
// in the thread that made the call instead. The page still works: a call
// decided there, or expired, closes the card.
//
// Which thread: the gateway knows the agent (a login, shared by many
// threads), not the thread. So a held call is matched to a thread's own
// gateway tool call by tool name and arguments, while the call is still in
// flight (the gateway holds it for 45 seconds before answering), or by the
// approval id in a tool result (the "Waiting for ..." text, or
// connectors__await_approval's own call). Nothing the call's arguments say
// decides anything here: they are only compared, and shown on the card.
//
// The key is secrets/approvals-key in the gateway's data folder, read by
// the runner only (never an agent), and sent only to a gateway on this
// Mac's loopback address.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import type { Clock, TimerHandle } from './clock.ts';
import { GATEWAY_TOOL_PREFIX, gatewayIsLocal } from './connectors.ts';

export const GATEWAY_DATA = path.join(os.homedir(), 'Library', 'Application Support', 'Connectors');
export const APPROVALS_KEY_FILE = path.join(GATEWAY_DATA, 'secrets', 'approvals-key');

// A gateway call is held 45 seconds before the agent hears it's waiting;
// a little longer than that, a call that hasn't shown up isn't held.
export const WATCH_MS = 60_000;
// How often the waiting list is read while a call is being watched or a
// card is open.
export const POLL_MS = 1_500;
const TIMEOUT_MS = 5_000;
// tool.call cuts strings at 16 KB (docs/protocol.md); past this, only the
// start is compared.
const CUT_AT = 16 * 1024 - 64;

export type GatewayApproval = {
  id: string;
  status: 'pending' | 'approved' | 'declined' | 'expired' | 'ran' | string;
  agent: string;
  upstream: string;
  tool: string; // <prefix>__<tool>, as the agent called it
  upstreamTool: string;
  args: Record<string, unknown>;
  url?: string;
};

// The gateway tool a thread's call is, or undefined: mcp__connectors__slack__slack_send_message
// is slack__slack_send_message. The gateway's own tools (connectors__*) never hold.
export function gatewayTool(name: string): string | undefined {
  if (!name.startsWith(GATEWAY_TOOL_PREFIX)) return undefined;
  const rest = name.slice(GATEWAY_TOOL_PREFIX.length);
  const [prefix, ...tool] = rest.split('__');
  if (!prefix || !tool.length || prefix === 'connectors') return undefined;
  return rest;
}

// Whether a thread's call input is the held call's arguments. Deep equal,
// except a string the event cut short matches by its start.
export function sameArgs(input: unknown, args: unknown): boolean {
  if (typeof input === 'string' && typeof args === 'string') {
    if (input === args) return true;
    return input.length >= CUT_AT && args.startsWith(input.slice(0, CUT_AT));
  }
  if (Array.isArray(input) || Array.isArray(args)) {
    if (!Array.isArray(input) || !Array.isArray(args) || input.length !== args.length) return false;
    return input.every((v, i) => sameArgs(v, args[i]));
  }
  if (input && args && typeof input === 'object' && typeof args === 'object') {
    const a = input as Record<string, unknown>;
    const b = args as Record<string, unknown>;
    const keys = new Set([...Object.keys(a), ...Object.keys(b)].filter((k) => a[k] !== undefined || b[k] !== undefined));
    return [...keys].every((k) => sameArgs(a[k], b[k]));
  }
  return input === args;
}

// The approval ids a tool result names (the gateway's "Waiting for ... at
// <base>/approvals/ap_..." text), only on the gateway's own address.
export function approvalIdsIn(text: string, base: string): string[] {
  const origin = (() => {
    try {
      return new URL(base).origin;
    } catch {
      return undefined;
    }
  })();
  if (!origin) return [];
  const out = new Set<string>();
  for (const m of text.slice(0, 8000).matchAll(/https?:\/\/[^\s"'<>)\]]+\/approvals\/(ap_[A-Za-z0-9_-]{8,64})/g)) {
    try {
      if (new URL(m[0]).origin === origin) out.add(m[1]);
    } catch {
      // not a link
    }
  }
  return [...out];
}

// "Slack: send message", from the service's title and the tool's own name
// with the service's name taken off its front.
export function cardTitle(a: Pick<GatewayApproval, 'upstream' | 'upstreamTool'>, serviceTitle?: string): string {
  const service = serviceTitle?.trim() || a.upstream.charAt(0).toUpperCase() + a.upstream.slice(1);
  let tool = a.upstreamTool.replace(new RegExp(`^${a.upstream.replace(/[^A-Za-z0-9]/g, '')}[_-]`, 'i'), '');
  tool = tool.replace(/[_-]+/g, ' ').trim();
  return `${service}: ${tool || a.upstreamTool}`;
}

export type GatewayDecision = 'approve' | 'decline';

// A gateway from before /api/approvals: its page is the only way.
export class NoApprovalsApi extends Error {
  constructor() {
    super('This gateway has no /api/approvals.');
  }
}
// How long a gateway without it is left alone before asking again (it may
// be updated meanwhile).
export const NO_API_BACKOFF_MS = 10 * 60_000;

// The gateway's /api/approvals. Every call throws on a network failure or
// an answer it can't read; the watcher treats that as "try again later".
export class GatewayApprovalsClient {
  readonly base: string;
  private readonly keyFile: string;
  private readonly fetchImpl: typeof fetch;

  constructor(base: string, keyFile = APPROVALS_KEY_FILE, fetchImpl: typeof fetch = fetch) {
    this.base = base;
    this.keyFile = keyFile;
    this.fetchImpl = fetchImpl;
  }

  private key(): string | undefined {
    try {
      const k = fs.readFileSync(this.keyFile, 'utf8').trim();
      return /^[0-9a-f]{64,}$/.test(k) ? k : undefined;
    } catch {
      return undefined;
    }
  }

  // Whether it can be used at all: a gateway on this Mac, and its key.
  usable(): boolean {
    return gatewayIsLocal(this.base) && Boolean(this.key());
  }

  private async call(pathname: string, init: RequestInit = {}): Promise<{ status: number; body: unknown }> {
    const key = this.key();
    if (!key || !gatewayIsLocal(this.base)) throw new Error('No approvals key for a gateway on this Mac.');
    const res = await this.fetchImpl(new URL(pathname, this.base), {
      ...init,
      headers: { authorization: `Bearer ${key}`, 'content-type': 'application/json' },
      signal: AbortSignal.timeout(TIMEOUT_MS),
      redirect: 'error',
    });
    const body = await res.json().catch(() => undefined);
    return { status: res.status, body };
  }

  async list(): Promise<GatewayApproval[]> {
    const { status, body } = await this.call('/api/approvals');
    if (status === 404) throw new NoApprovalsApi();
    if (status !== 200) throw new Error(`The gateway answered ${status}.`);
    const list = (body as { approvals?: unknown })?.approvals;
    return Array.isArray(list) ? list.filter(isApproval) : [];
  }

  async get(id: string): Promise<GatewayApproval | undefined> {
    const { status, body } = await this.call(`/api/approvals/${encodeURIComponent(id)}`);
    if (status === 404) return undefined;
    if (status !== 200 || !isApproval(body)) throw new Error(`The gateway answered ${status}.`);
    return body;
  }

  // The approval as it stands after the decision: decided now, or already
  // decided (on the page, or it expired) when the answer is 409.
  async decide(id: string, decision: GatewayDecision): Promise<GatewayApproval | undefined> {
    const { status, body } = await this.call(`/api/approvals/${encodeURIComponent(id)}`, { method: 'POST', body: JSON.stringify({ decision }) });
    if (status === 200 && isApproval(body)) return body;
    if (status === 409) {
      const current = (body as { approval?: unknown })?.approval;
      return isApproval(current) ? current : this.get(id);
    }
    if (status === 404) return undefined;
    throw new Error(`The gateway answered ${status}.`);
  }
}

function isApproval(x: unknown): x is GatewayApproval {
  const a = x as GatewayApproval;
  return Boolean(a) && typeof a.id === 'string' && typeof a.status === 'string' && typeof a.tool === 'string' && typeof a.upstream === 'string' && typeof a.upstreamTool === 'string' && Boolean(a.args) && typeof a.args === 'object';
}

// What the runner does with a matched call: put a card up, take one down.
export type GatewayCardHost = {
  // Puts the card in the thread. `decide` is what Allow and Deny do.
  showGatewayCard(threadId: string, card: { cardId: string; approval: GatewayApproval; title: string; decide: (d: GatewayDecision) => Promise<GatewayApproval | undefined> }): boolean;
  // Closes a card the gateway decided some other way.
  closeGatewayCard(threadId: string, cardId: string, approval: GatewayApproval | undefined): void;
};

export type GatewayApprovalsOptions = {
  client: () => GatewayApprovalsClient | undefined;
  host: GatewayCardHost;
  clock: Clock;
  serviceTitle?: (upstream: string) => string | undefined;
  log?: { warn(msg: string): void };
};

type Watch = { threadId: string; tool: string; input: Record<string, unknown>; until: number };
type Card = { threadId: string; cardId: string };

export class GatewayApprovals {
  private watches: Watch[] = [];
  // Gateway approval id to its card.
  private cards = new Map<string, Card>();
  // Ids already handled (carded, or seen decided), so a held call that
  // was answered on the page isn't carded on a later poll.
  private seen = new Set<string>();
  private timer?: TimerHandle;
  private polling = false;
  private offUntil = 0;

  private readonly opts: GatewayApprovalsOptions;

  constructor(opts: GatewayApprovalsOptions) {
    this.opts = opts;
  }

  // Every event a thread's agent makes: its gateway tool calls are watched.
  observe(threadId: string, ev: { type: string; itemId?: string; name?: string; input?: Record<string, unknown>; text?: string }): void {
    if (ev.type === 'tool.call' && typeof ev.name === 'string') {
      const tool = gatewayTool(ev.name);
      if (!tool || this.opts.clock.now() < this.offUntil || !this.opts.client()?.usable()) return;
      this.watches.push({ threadId, tool, input: ev.input ?? {}, until: this.opts.clock.now() + WATCH_MS });
      this.kick();
    } else if (ev.type === 'tool.result' && typeof ev.text === 'string' && ev.text.includes('/approvals/ap_')) {
      const client = this.opts.client();
      if (!client?.usable()) return;
      for (const id of approvalIdsIn(ev.text, client.base)) {
        if (this.seen.has(id)) continue;
        void client.get(id).then(
          (a) => {
            if (a && a.status === 'pending' && !this.seen.has(a.id)) this.card(threadId, a);
          },
          (e) => this.opts.log?.warn(`gateway approval ${id}: ${(e as Error).message}`),
        );
      }
    }
  }

  // A card closed in the thread without an answer (the thread went, the
  // runner stopped): the gateway's call stays, for its page.
  forget(cardId: string): void {
    for (const [id, c] of this.cards) if (c.cardId === cardId) this.cards.delete(id);
  }

  stop(): void {
    this.timer?.cancel();
    this.timer = undefined;
    this.watches = [];
  }

  private kick(): void {
    if (this.timer || this.polling) return;
    this.timer = this.opts.clock.setTimeout(() => {
      this.timer = undefined;
      void this.poll();
    }, 0);
  }

  // One read of the waiting list: match new held calls to watches, and
  // close cards whose calls are no longer waiting.
  async poll(): Promise<void> {
    const client = this.opts.client();
    const now = this.opts.clock.now();
    this.watches = this.watches.filter((w) => w.until > now);
    if (!client?.usable() || (!this.watches.length && !this.cards.size)) return;
    this.polling = true;
    try {
      const waiting = await client.list();
      const ids = new Set(waiting.map((a) => a.id));
      for (const a of waiting) {
        if (this.seen.has(a.id)) continue;
        const i = this.watches.findIndex((w) => w.tool === a.tool && sameArgs(w.input, a.args));
        if (i < 0) continue;
        const [w] = this.watches.splice(i, 1);
        this.card(w.threadId, a);
      }
      for (const [id, c] of [...this.cards]) {
        if (ids.has(id)) continue;
        // Gone from the list: decided on the page, or expired.
        this.cards.delete(id);
        const current = await client.get(id).catch(() => undefined);
        this.opts.host.closeGatewayCard(c.threadId, c.cardId, current);
      }
    } catch (e) {
      if (e instanceof NoApprovalsApi) {
        this.offUntil = this.opts.clock.now() + NO_API_BACKOFF_MS;
        this.watches = [];
      } else this.opts.log?.warn(`gateway approvals: ${(e as Error).message}`);
    } finally {
      this.polling = false;
      if (this.watches.length || this.cards.size) {
        this.timer ??= this.opts.clock.setTimeout(() => {
          this.timer = undefined;
          void this.poll();
        }, POLL_MS);
      }
    }
  }

  private card(threadId: string, a: GatewayApproval): void {
    this.seen.add(a.id);
    const client = this.opts.client();
    if (!client) return;
    const cardId = `gw-${a.id}`;
    const shown = this.opts.host.showGatewayCard(threadId, {
      cardId,
      approval: a,
      title: cardTitle(a, this.opts.serviceTitle?.(a.upstream)),
      decide: async (d) => {
        const after = await client.decide(a.id, d);
        this.cards.delete(a.id);
        return after;
      },
    });
    if (shown) {
      this.cards.set(a.id, { threadId, cardId });
      this.kick();
    }
  }
}
