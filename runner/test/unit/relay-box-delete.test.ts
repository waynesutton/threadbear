// Deleting an archived thread of a virtual runner from the Mac app
// (relay.ts threads.delete): the backend does it (boxes:deleteThread), as
// no Mac has its files. Another Mac's thread is still deleted there.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Relay } from '../../src/relay.ts';
import { FakeClock } from '../../src/clock.ts';
import { nullLogger } from '../../src/log.ts';
import type { RemoteThreads } from '../../src/thread-index.ts';

const T = 'thread-on-a-box';
const at = new Date('2026-11-25T05:00:00Z').toISOString();

function relayFor(box: boolean, del?: (threadId: string) => Promise<unknown>) {
  const remote: RemoteThreads = {
    runners: [{ id: 'r1', name: box ? 'Cloud (Fly)' : 'mac-mini', online: false, lastSeenAt: at, ...(box ? { box: { state: 'gone' } } : {}) }],
    threads: [{ runnerId: 'r1', threadId: T, title: 'A thread', cwd: '/work/x', accountId: 'claude-main', provider: 'claude', status: 'idle', needsYou: false, unread: false, preview: '', createdAt: at, lastActivityAt: at, archivedAt: at }],
  };
  return new Relay({
    source: () => undefined,
    remote: () => remote,
    publish: () => undefined,
    subscribed: () => false,
    ...(del ? { deleteBoxThread: del } : {}),
    clock: new FakeClock(),
    log: nullLogger,
  });
}

test("a virtual runner's archived thread is deleted by the backend", async () => {
  const asked: string[] = [];
  const relay = relayFor(true, async (id) => void asked.push(id));
  assert.deepEqual(await relay.call(T, 'threads.delete', { threadId: T }), {});
  assert.deepEqual(asked, [T]);
});

test('what the backend says about a delete it refused reaches the app', async () => {
  const relay = relayFor(true, async () => {
    throw Object.assign(new Error('ConvexError'), { data: { kind: 'conflict', message: 'Its machine is still being put away. Try again in a minute.' } });
  });
  await assert.rejects(relay.call(T, 'threads.delete', { threadId: T }), /still being put away/);
});

test("another Mac's thread is still deleted on that Mac", async () => {
  const asked: string[] = [];
  const relay = relayFor(false, async (id) => void asked.push(id));
  await assert.rejects(relay.call(T, 'threads.delete', { threadId: T }), /isn't possible through Convex yet/);
  assert.deepEqual(asked, []);
});
