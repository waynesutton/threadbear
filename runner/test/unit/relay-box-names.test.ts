// A thread on a virtual runner, as the Mac app's chips show it (relay.ts):
// named after its virtual runner, not its machine's id, and its login is
// the one of this Mac's its cloud token is for, with that login's models.
// Before, the chip said "e82d4d91b721d8" and the login menu "No accounts
// yet", as a box lists no logins (Wayne's fork, 25 Nov 2026).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Relay } from '../../src/relay.ts';
import { FakeClock } from '../../src/clock.ts';
import { nullLogger } from '../../src/log.ts';
import type { RemoteThreads } from '../../src/thread-index.ts';

const at = new Date('2026-11-25T08:00:00Z').toISOString();
const MAC = 'mac000000001';
const BOX = 'box000000001';
const rows: Record<string, unknown[]> = {
  [MAC]: [
    { runnerId: MAC, id: 'claude-main', provider: 'claude', title: 'Main', email: 'wayne@example.com', models: [{ id: 'opus', title: 'Opus' }, { id: 'fable', title: 'Fable' }] },
    { runnerId: MAC, id: 'claude-two', provider: 'claude', title: 'Two', email: 'other@example.com' },
    { runnerId: MAC, id: 'codex-main', provider: 'codex', title: 'Codex', email: 'wayne@example.com' },
  ],
  [BOX]: [],
};

function relay(box: boolean) {
  const remote: RemoteThreads = {
    runners: [{ id: BOX, name: 'e82d4d91b721d8', online: true, lastSeenAt: at, ...(box ? { box: { state: 'awake', service: 's1', login: 'Wayne@example.com' } } : {}) }],
    threads: [{ runnerId: BOX, threadId: 'T', title: 'T', cwd: '/work/x', accountId: 'claude-main', provider: 'claude', status: 'idle', needsYou: false, unread: false, preview: '', createdAt: at, lastActivityAt: at }],
    services: [{ id: 's1', name: 'Cloud (Fly)', provider: 'fly' }],
  };
  const asked: string[] = [];
  const r = new Relay({
    source: () => ({ key: 'owner', apiVersion: 111, upload: async () => '', download: async () => new Uint8Array(), post: async (_k, fn, args) => { asked.push(`${fn}:${args.runnerId}`); return rows[String(args.runnerId)] ?? []; } }),
    remote: () => remote,
    publish: () => undefined,
    subscribed: () => false,
    ownRunnerId: () => MAC,
    clock: new FakeClock(),
    log: nullLogger,
  });
  return { r, asked };
}

test("a box's thread offers this Mac's login its cloud token is for, with its models", async () => {
  const { r, asked } = relay(true);
  const got = await r.logins(BOX);
  assert.deepEqual(asked, [`relay:remoteAccounts:${MAC}`]);
  assert.deepEqual((got.accounts as { id: string }[]).map((a) => a.id), ['claude-main']);
  assert.deepEqual((got.accounts as { models: { id: string }[] }[])[0]!.models.map((m) => m.id), ['opus', 'fable']);
});

test("another Mac's thread still offers that Mac's own logins", async () => {
  const { r, asked } = relay(false);
  await r.logins(BOX);
  assert.deepEqual(asked, [`relay:remoteAccounts:${BOX}`]);
});

test("a box's thread is named after its virtual runner where the relay names it", async () => {
  const { r } = relay(true);
  await assert.rejects(r.call('T', 'threads.update', { threadId: 'T', cwd: '/elsewhere' }), /Do it on Cloud \(Fly\)\./);
});
