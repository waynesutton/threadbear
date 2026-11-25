// An expired box's rows go in the end (convex/boxService.ts purgeExpiredBoxes).
// Before, nothing ever purged a box, so every virtual-runner thread left a
// `runners` row for good, read by every full read of runners: the held-push
// cron every 2 minutes, each keyed heartbeat's name check, the runners lists
// (code review, 20 Nov 2026). An expired box (its save gone, so it can't come
// back) keeps showing, and saying so, for its virtual runner's
// deleteAfterDays again; then it goes with its thread and runner.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { call, load, memoryBackend, THREAD_BASICS } from '../helpers/convex-memory.ts';

const boxService = load('boxService');
const sync = load('sync');
const DAY = 24 * 60 * 60_000;
const DAYS = 14;

async function world() {
  const b = memoryBackend();
  const serviceId = await b.ctx.db.insert('runnerServices', { name: 'Cloud', provider: 'fly', box: { cpus: 1, memoryMb: 1024 }, image: 'registry.fly.io/img@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', limits: { maxBoxes: 5, idleSleepMinutes: 10, maxAwakeHoursPerDay: 8, deleteAfterDays: DAYS }, createdAt: 1 });
  // The main Mac, which stays whatever happens.
  await b.ctx.db.insert('runners', { runnerId: 'mac000000001', name: 'Laptop', version: '1', lastSeenAt: Date.now(), startedAt: 'x', draining: false, threadCount: 1, byOwner: true });
  const box = async (n: number, state: Record<string, unknown>, extra: Record<string, unknown> = {}) => {
    const runnerId = `box00000000${n}`;
    const threadId = `thread-${n}`;
    const id = await b.ctx.db.insert('boxes', { serviceId, threadId, runnerId, provider: 'fly', boxState: state, boxStateTimers: [], sealedThreadKey: 's', repo: { origin: 'o', subfolder: '', baseBranch: 'main' }, login: 'claude-main', createdAt: 1, ...extra });
    await b.ctx.db.insert('runners', { runnerId, name: `box ${n}`, version: '1', lastSeenAt: 1, startedAt: 'x', draining: false, threadCount: 1, box: { serviceId } });
    await b.ctx.db.insert('runnerStats', { runnerId, at: 1 } as never);
    await b.ctx.db.insert('threads', { threadId, runnerId, ...THREAD_BASICS, title: `T${n}`, view: 'den', needsYou: false });
    await b.ctx.db.insert('threadIndex', { threadId, runnerId, ...THREAD_BASICS, title: `T${n}`, archivedAt: 'x', updatedMs: 1 } as never);
    for (let seq = 0; seq < 3; seq++) await b.ctx.db.insert('messages', { threadId, runnerId, seq, role: 'assistant', text: 'hi', at: 'x' } as never);
    return { id, runnerId, threadId };
  };
  return { ...b, serviceId, box, destroys: [] as string[] };
}

const RUNS = /purgeExpiredBoxes|purgeBox|forgetBox|dropThreadRows|failExpired/;
const boxes = load('boxes');
const fnFor = (name: string) => (/dropThreadRows/.test(name) ? sync.dropThreadRows : /failExpired/.test(name) ? boxes.failExpired : boxService[name.split('.').pop()!]);

async function purgeAll(w: Awaited<ReturnType<typeof world>>) {
  w.scheduled.length = 0;
  w.destroys = [];
  await call(w.ctx, boxService.purgeExpiredBoxes, {});
  for (let runs = 0; runs < 200; runs++) {
    for (const x of w.scheduled as { fn: string; args: Record<string, unknown> }[]) if (/destroyBox/.test(x.fn)) w.destroys.push(x.args.boxId as string);
    for (let j = w.scheduled.length - 1; j >= 0; j--) if (/destroyBox/.test((w.scheduled[j] as any).fn)) w.scheduled.splice(j, 1);
    const i = w.scheduled.findIndex((x: any) => RUNS.test(x.fn));
    if (i < 0) return runs;
    const [next] = w.scheduled.splice(i, 1) as { fn: string; args: Record<string, unknown> }[];
    await call(w.ctx, fnFor(next!.fn), next!.args);
  }
  throw new Error('the purge never finished');
}

test('an expired box past its days again goes, with its thread and runner, and nothing else does', async () => {
  const w = await world();
  const now = Date.now();
  const old = await w.box(1, { kind: 'expired', at: now - (DAYS + 1) * DAY });
  const recent = await w.box(2, { kind: 'expired', at: now - (DAYS - 1) * DAY });
  const gone = await w.box(3, { kind: 'gone', at: now - 100 * DAY });
  const kept = await w.box(4, { kind: 'expired', at: now - 100 * DAY }, { providerBoxId: 'm-still-there' });
  const running = await w.box(6, { kind: 'expired', at: now - 100 * DAY }, { seenRunningAt: now - 60_000 });
  const rekeyed = await w.box(7, { kind: 'expired', at: now - 100 * DAY });
  await w.ctx.db.insert('runnerKeys', { runnerId: rekeyed.runnerId, keyHash: 'h', name: 'a Mac', createdAt: 1 });
  const leased = await w.box(5, { kind: 'expired', at: now - 100 * DAY }, { op: { id: 'op', name: 'destroyBox', until: now + 60_000 } });
  await w.ctx.db.insert('boxCondemned', { boxId: old.id, providerBoxId: 'm-old', attempt: 1, at: 1 });
  await purgeAll(w);
  const runners = w.rows('runners').map((r) => r.runnerId).sort();
  const stay = [recent, gone, kept, leased, running, rekeyed];
  assert.deepEqual(runners, ['mac000000001', ...stay.map((b) => b.runnerId)].sort(), "only the old expired box's runner went");
  assert.deepEqual(w.rows('boxes').map((r) => r._id).sort(), stay.map((b) => b.id).sort());
  assert.equal(w.rows('boxCondemned').length, 0);
  // The one the provider may still have is asked to be deleted again; its DESTROYED clears the way.
  assert.ok([...w.destroys, ...(w.scheduled as any[]).filter((x) => /destroyBox/.test(x.fn)).map((x) => x.args.boxId)].includes(kept.id));
  for (const table of ['threads', 'threadIndex', 'messages', 'runnerStats'] as const) {
    assert.ok(!w.rows(table).some((r) => r.runnerId === old.runnerId), `its ${table} went`);
    assert.ok(w.rows(table).some((r) => r.runnerId === recent.runnerId), `the recent one's ${table} stayed`);
  }
  assert.ok(w.history.some((h) => h.machine === 'box' && h.event === 'PURGE'), 'the step is in its history');
});

test('many expired boxes go a box a run, each run under its batch', async () => {
  const w = await world();
  for (let n = 0; n < 7; n++) await w.box(n, { kind: 'expired', at: Date.now() - 30 * DAY });
  const runs = await purgeAll(w);
  assert.equal(w.rows('boxes').length, 0);
  assert.deepEqual(w.rows('runners').map((r) => r.runnerId), ['mac000000001']);
  assert.ok(runs >= 7, `${runs} runs`);
});

test("a thread's messages go a share of a run at a time, however big", async () => {
  const w = await world();
  const b = await w.box(1, { kind: 'expired', at: Date.now() - 30 * DAY });
  for (let seq = 3; seq < 40; seq++) await w.ctx.db.insert('messages', { threadId: b.threadId, runnerId: b.runnerId, seq, role: 'user', text: 'x'.repeat(1024 * 1024), at: 'x' } as never);
  w.scheduled.length = 0;
  await call(w.ctx, boxService.purgeExpiredBoxes, {});
  for (let runs = 0; runs < 200; runs++) {
    const i = w.scheduled.findIndex((x: any) => RUNS.test(x.fn));
    if (i < 0) break;
    const [next] = w.scheduled.splice(i, 1) as { fn: string; args: Record<string, unknown> }[];
    w.resetReads();
    await call(w.ctx, fnFor(next!.fn), next!.args);
    assert.ok(w.bytesRead() < 16 * 1024 * 1024, `a run read ${w.bytesRead()}`);
  }
  assert.equal(w.rows('boxes').length, 0);
  assert.equal(w.rows('messages').length, 0);
});

// Codex's review: the runner's rows include runner requests, another machine's,
// which a catalog-built mutation may not delete; and a box that can't be purged
// must hold up no other.
test("a box's runner requests go too, and each box is purged in runs of its own", async () => {
  const w = await world();
  const a = await w.box(1, { kind: 'expired', at: Date.now() - 30 * DAY });
  const b = await w.box(2, { kind: 'expired', at: Date.now() - 30 * DAY });
  await w.ctx.db.insert('runnerStorage', { runnerId: a.runnerId, bytes: 0 } as never);
  // A message sent to it while its rows go: failed, saying it can't come back.
  const late = await w.ctx.db.insert('commands', { runnerId: a.runnerId, kind: 'send', args: { threadId: a.threadId, text: 'x' }, state: 'pending', createdAt: Date.now(), commandState: { kind: 'pending' }, commandStateTimers: [] } as never);
  await w.ctx.db.insert('runnerRequests', { runnerId: a.runnerId, kind: 'start', state: 'pending', requestState: { kind: 'pending' }, createdAt: 1 } as never);
  await purgeAll(w);
  assert.equal(w.rows('runnerRequests').length, 0);
  assert.ok(!w.rows('runnerStorage').some((r) => r.runnerId === a.runnerId), 'its storage count went');
  assert.deepEqual(w.rows('boxes'), []);
  assert.equal((await w.ctx.db.get(late))?.state, 'failed');
  assert.ok(b);
});

test('nothing expired, nothing scheduled', async () => {
  const w = await world();
  await w.box(1, { kind: 'expired', at: Date.now() - DAY });
  await purgeAll(w);
  assert.equal(w.scheduled.length, 0);
  assert.equal(w.rows('boxes').length, 1);
});

// Deleting an archived thread on a virtual runner from the app (boxes.ts
// deleteThread): before, Delete was greyed out for it ("Delete it on the Mac
// it runs on", and there's no such Mac), so it stayed in Archived for its
// virtual runner's deleteAfterDays twice over (Wayne's fork, 24 Nov 2026).
// world()'s sidebar rows carry a thread row's fields; one that's patched must fit the schema.
async function fitIndex(w: Awaited<ReturnType<typeof world>>) {
  for (const r of w.rows('threadIndex')) await w.ctx.db.patch(r._id, { lastActivityMs: undefined, needsYou: false, unread: false } as never);
}

const DELETE_RUNS = /deleteNow|purgeDeleted|purgeBox|forgetBox|dropThreadRows|failExpired/;

async function deleteAll(w: Awaited<ReturnType<typeof world>>) {
  w.destroys = [];
  for (let runs = 0; runs < 200; runs++) {
    for (const x of w.scheduled as { fn: string; args: Record<string, unknown> }[]) if (/destroyBox/.test(x.fn)) w.destroys.push(x.args.boxId as string);
    const queued = w.scheduled as { fn: string }[];
    for (let j = queued.length - 1; j >= 0; j--) if (/destroyBox/.test(queued[j]!.fn)) queued.splice(j, 1);
    const i = queued.findIndex((x) => DELETE_RUNS.test(x.fn));
    if (i < 0) return runs;
    const [next] = w.scheduled.splice(i, 1) as { fn: string; args: Record<string, unknown> }[];
    await call(w.ctx, fnFor(next!.fn), next!.args);
  }
  throw new Error('the delete never finished');
}

test('deleting an archived thread on a virtual runner takes its rows and runner at once, and no other', async () => {
  process.env.RUNNER_KEY = 'owner-key';
  const w = await world();
  const now = Date.now();
  const mine = await w.box(1, { kind: 'gone', at: now - 60_000 });
  const other = await w.box(2, { kind: 'gone', at: now - 60_000 });
  await fitIndex(w);
  w.scheduled.length = 0;
  await call(w.ctx, boxes.deleteThread, { key: 'owner-key', threadId: mine.threadId });
  // Its sidebar row is a tombstone at once, so the app drops it before the rest goes.
  assert.equal(w.rows('threadIndex').find((r) => r.threadId === mine.threadId)?.deleted, true);
  await deleteAll(w);
  assert.deepEqual(w.rows('boxes').map((r) => r._id), [other.id]);
  assert.deepEqual(w.rows('runners').map((r) => r.runnerId).sort(), ['mac000000001', other.runnerId].sort());
  for (const table of ['threads', 'threadIndex', 'messages'] as const) {
    assert.ok(!w.rows(table).some((r) => r.runnerId === mine.runnerId), `its ${table} went`);
    assert.ok(w.rows(table).some((r) => r.runnerId === other.runnerId), `the other thread's ${table} stayed`);
  }
});

test('a deleted thread whose box was kept has that box deleted first', async () => {
  process.env.RUNNER_KEY = 'owner-key';
  const w = await world();
  const kept = await w.box(1, { kind: 'shelved', at: Date.now() - 60_000 }, { providerBoxId: 'm-kept' });
  await fitIndex(w);
  w.scheduled.length = 0;
  await call(w.ctx, boxes.deleteThread, { key: 'owner-key', threadId: kept.threadId });
  await deleteAll(w);
  assert.ok(w.destroys.includes(kept.id), 'the provider is asked to delete its box');
  assert.equal(w.rows('boxes')[0]?.boxState.kind, 'expired');
  // The provider says it's gone (DESTROYED clears providerBoxId): the next look purges it.
  await w.ctx.db.patch(kept.id, { providerBoxId: undefined });
  await call(w.ctx, boxService.purgeDeleted, { boxId: kept.id, tries: 1 });
  await deleteAll(w);
  assert.equal(w.rows('boxes').length, 0);
});

test("a thread that isn't archived, or another key's ask, deletes nothing", async () => {
  process.env.RUNNER_KEY = 'owner-key';
  const w = await world();
  const asleep = await w.box(1, { kind: 'asleep', since: Date.now() });
  const gone = await w.box(2, { kind: 'gone', at: Date.now() });
  w.scheduled.length = 0;
  await assert.rejects(call(w.ctx, boxes.deleteThread, { key: 'owner-key', threadId: asleep.threadId }), (e: { data?: { message?: string } }) => /Archive it first/.test(e?.data?.message ?? ''));
  await assert.rejects(call(w.ctx, boxes.deleteThread, { key: 'not-the-owner', threadId: gone.threadId }));
  assert.equal(w.scheduled.length, 0);
  assert.equal(w.rows('boxes').length, 2);
});

test("a thread whose unarchive failed is archived, then deleted, and what waited for its box fails", async () => {
  process.env.RUNNER_KEY = 'owner-key';
  const w = await world();
  const failed = await w.box(1, { kind: 'failed', at: Date.now(), error: 'No box image has been promoted' });
  await fitIndex(w);
  w.scheduled.length = 0;
  await call(w.ctx, boxes.deleteThread, { key: 'owner-key', threadId: failed.threadId });
  assert.equal(w.rows('boxes')[0]?.boxState.kind, 'destroying');
  assert.equal(w.rows('threadIndex').find((r) => r.threadId === failed.threadId)?.deleted, true);
  // It waits while the box is put away, then carries on once it's gone.
  await deleteAll(w);
  assert.equal(w.rows('boxes')[0]?.boxState.kind, 'destroying');
  await w.ctx.db.patch(failed.id, { boxState: { kind: 'gone', at: Date.now() } });
  await call(w.ctx, boxService.deleteNow, { boxId: failed.id, tries: 1 });
  await deleteAll(w);
  assert.equal(w.rows('boxes').length, 0);
  assert.ok(!w.rows('threads').some((r) => r.runnerId === failed.runnerId));
});

test("a failed box whose thread isn't archived is left alone", async () => {
  process.env.RUNNER_KEY = 'owner-key';
  const w = await world();
  const failed = await w.box(1, { kind: 'failed', at: Date.now(), error: 'x' });
  const t = w.rows('threads').find((r) => r.threadId === failed.threadId)!;
  await w.ctx.db.patch(t._id, { view: 'inbox' });
  w.scheduled.length = 0;
  await assert.rejects(call(w.ctx, boxes.deleteThread, { key: 'owner-key', threadId: failed.threadId }), (e: { data?: { message?: string } }) => /Archive it first/.test(e?.data?.message ?? ''));
  assert.equal(w.rows('boxes')[0]?.boxState.kind, 'failed');
});
