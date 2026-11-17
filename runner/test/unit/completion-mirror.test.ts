import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, load, memoryBackend, THREAD_BASICS } from '../helpers/convex-memory.ts';
import { COMPLETION_API_VERSION, forBackend, toThreadDoc } from '../../src/sync.ts';
import type { ThreadSummary } from '../../src/protocol.ts';

const sync = load('sync');
const phone = load('phone');
const relay = load('relay');
const { sha256 } = load('lib') as { sha256: (text: string) => Promise<string> };
const completion = { tasks: [
  { id: 'build', title: 'Build the card', status: 'completed' as const },
  { id: 'try', title: 'Try it in the app', status: 'blocked' as const },
] };

test('completion mirror is additive, scrubbed, and absent on older backends or busy threads', () => {
  const thread: ThreadSummary = {
    id: 'T', title: 'Completion', cwd: '/w', accountId: 'a', provider: 'claude', permissionMode: 'full',
    status: 'idle', live: false, needsYou: true, needs: 'you', lastSeq: 5, lastSeenSeq: 0,
    createdAt: '2026-11-16T00:00:00Z', updatedAt: '2026-11-16T00:00:00Z', lastActivityAt: '2026-11-16T00:00:00Z', preview: '', completion,
  };
  const doc = toThreadDoc(thread, 'r', Date.now(), undefined, undefined, undefined, (text) => text.replace('app', '[redacted]'));
  assert.equal(doc.completion?.tasks[1]?.title, 'Try it in the [redacted]');
  assert.equal(forBackend(doc, COMPLETION_API_VERSION - 1, Date.now()).completion, undefined);
  assert.deepEqual(forBackend(doc, COMPLETION_API_VERSION, Date.now()).completion, doc.completion);
  assert.equal(toThreadDoc({ ...thread, status: 'running' }, 'r', Date.now()).completion, undefined);
  assert.equal(toThreadDoc({ ...thread, archivedAt: '2026-11-16T00:00:00Z' }, 'r', Date.now()).completion, undefined);
});

test('the real backend schema passes task status to the phone, website and other Mac', async () => {
  process.env.RUNNER_KEY = 'completion-owner';
  delete process.env.RUNNER_KEY_NEXT;
  const backend = memoryBackend();
  await backend.ctx.db.insert('tokens', { hash: await sha256('completion-phone'), name: 'phone', kind: 'phone', expiresAt: Date.now() + 1e9, createdAt: Date.now() });
  await call(backend.ctx, sync.upsertThreads, { key: 'completion-owner', threads: [{
    ...THREAD_BASICS, threadId: 'T', runnerId: 'mac', title: 'Completion', view: 'inbox', needsYou: true, completion,
  }] });
  const row = await call(backend.ctx, phone.threadRow, { token: 'completion-phone', threadId: 'T' });
  assert.deepEqual(row.completion, completion);
  const tail = await call(backend.ctx, relay.remoteThread, { key: 'completion-owner', threadId: 'T' });
  assert.deepEqual(tail.completion, completion);
  await call(backend.ctx, sync.upsertThreads, { key: 'completion-owner', threads: [{
    ...THREAD_BASICS, threadId: 'T', runnerId: 'mac', title: 'Completion', view: 'inbox', needsYou: false,
  }] });
  assert.equal((await call(backend.ctx, phone.threadRow, { token: 'completion-phone', threadId: 'T' })).completion, undefined, 'turning the experiment off clears the old card');
});
