import { test } from 'node:test';
import assert from 'node:assert/strict';
import { call, load, memoryBackend, outcome } from '../helpers/convex-memory.ts';

// convex/tasksLinear.ts (API 105) on the in-memory backend, its writes
// checked against convex/schema.ts.
const tasks = load('tasks');
const linear = load('tasksLinear');
const { sha256 } = load('lib');
async function setup() {
  process.env.RUNNER_KEY = 'task-owner';
  delete process.env.RUNNER_KEY_NEXT;
  const b = memoryBackend();
  await b.ctx.db.insert('tokens', { hash: await sha256('phone'), name: 'browser', expiresAt: Date.now() + 60000 });
  const auth = { key: 'task-owner' };
  const t = (name: string, args: Record<string, unknown> = {}) => call(b.ctx, tasks[name], { auth, ...args });
  const l = (name: string, args: Record<string, unknown> = {}) => call(b.ctx, linear[name], { auth, ...args });
  const get = async (id: string) => t('get', { id });
  return { ...b, t, l, get };
}
const snap = (over: Record<string, unknown> = {}) => ({
  issueId: 'uuid-7',
  identifier: 'ENG-7',
  url: 'https://linear.app/acme/issue/ENG-7/x',
  title: 'Fix it',
  priority: 2,
  dueDate: '2026-12-01',
  status: 'Todo',
  statusType: 'unstarted',
  updatedAt: '2026-11-20T10:00:00.000Z',
  ...over,
});
const claim = (requestId: string) => ({ id: 'a', requestId, teamId: 'team-1', teamName: 'Engineering', projectId: 'proj-1', projectName: 'Launch', sync: true });
async function linked(b: Awaited<ReturnType<typeof setup>>, sync = true) {
  await b.t('create', { id: 'a', title: 'Fix it', notes: 'Mine', priority: 'high', dueDate: '2026-12-01' });
  await b.l('claim', { ...claim('req-11111111'), sync });
  await b.l('settle', { id: 'a', requestId: 'req-11111111', issue: snap() });
}

test('every Linear operation needs a key, and the Mac-only ones refuse a browser', async () => {
  const b = await setup();
  await b.t('create', { id: 'a', title: 'Task' });
  const cases: Record<string, Record<string, unknown>> = {
    claim: claim('req-11111111'),
    uploaded: { id: 'a', requestId: 'r', storageId: 's', assetUrl: 'https://x' },
    waiting: { id: 'a', requestId: 'r', url: 'https://x' },
    settle: { id: 'a', requestId: 'r' },
    link: { id: 'a', issue: snap(), teamId: 't', teamName: 'T', sync: true },
    importIssues: { project: null, sync: true, issues: [] },
    apply: { id: 'a', issue: snap() },
    missing: { id: 'a', issueId: 'x' },
    setSync: { id: 'a', sync: false },
    unlink: { id: 'a' },
    synced: {},
    linkedIssues: { issueIds: [] },
    claimSweep: {},
  };
  assert.deepEqual(Object.keys(linear).filter((k) => typeof linear[k]?.handler === 'function').sort(), Object.keys(cases).sort(), 'new operations need an auth case');
  for (const [name, args] of Object.entries(cases)) {
    for (const auth of [{ token: 'invalid' }, { key: 'invalid' }]) assert.equal(await outcome(call(b.ctx, linear[name], { ...args, auth })), 'auth', name);
  }
  for (const name of ['claim', 'settle', 'link', 'importIssues', 'apply', 'claimSweep']) assert.equal(await outcome(call(b.ctx, linear[name], { ...cases[name], auth: { token: 'phone' } })), 'badArgs', name);
  // A browser can still see the link, switch sync and unlink.
  assert.deepEqual(await call(b.ctx, linear.synced, { auth: { token: 'phone' } }), []);
});

test('a claim is a lock, and settle only takes its own answer', async () => {
  const b = await setup();
  await b.t('create', { id: 'a', title: 'Fix it' });
  await b.l('claim', claim('req-11111111'));
  assert.equal(await outcome(b.l('claim', claim('req-22222222'))), 'badArgs');
  await b.l('settle', { id: 'a', requestId: 'req-22222222', issue: snap() });
  assert.equal((await b.get('a')).linear.state, 'creating');
  await b.l('uploaded', { id: 'a', requestId: 'req-11111111', storageId: 'st1', assetUrl: 'https://uploads.linear.app/1' });
  await b.l('settle', { id: 'a', requestId: 'req-11111111', error: 'Team archived' });
  let row = await b.get('a');
  assert.equal(row.linear.state, 'failed');
  assert.equal(row.linear.error, 'Team archived');
  // A retry keeps the uploads and makes the issue.
  const again = await b.l('claim', claim('req-33333333'));
  assert.deepEqual(again.uploads, [{ storageId: 'st1', assetUrl: 'https://uploads.linear.app/1' }]);
  await b.l('settle', { id: 'a', requestId: 'req-33333333', issue: snap() });
  row = await b.get('a');
  assert.equal(row.linear.state, 'linked');
  assert.equal(row.linear.identifier, 'ENG-7');
  assert.ok(row.links.some((x: { id: string; url: string }) => x.id === 'linear-issue' && x.url === snap().url));
  assert.equal(await outcome(b.l('claim', claim('req-44444444'))), 'badArgs');
  const events = (await b.t('activity', { id: 'a' })).items.map((i: { body: string }) => i.body);
  assert.ok(events.includes('sent to Linear as ENG-7'));
});

test('done tasks are not claimed, and one issue belongs to one task', async () => {
  const b = await setup();
  await b.t('create', { id: 'done', title: 'Done', status: 'done' });
  assert.equal(await outcome(b.l('claim', { ...claim('req-11111111'), id: 'done' })), 'badArgs');
  await linked(b);
  await b.t('create', { id: 'b', title: 'Other' });
  assert.equal(await outcome(b.l('link', { id: 'b', issue: snap(), teamId: 't', teamName: 'T', sync: true })), 'badArgs');
  await b.l('claim', { ...claim('req-55555555'), id: 'b' });
  await b.l('settle', { id: 'b', requestId: 'req-55555555', issue: snap() });
  assert.match((await b.get('b')).linear.error, /already belongs to another task/);
});

test('apply copies only what changed in Linear and keeps local edits', async () => {
  const b = await setup();
  await linked(b);
  await b.t('update', { id: 'a', title: 'My title' });
  let r = await b.l('apply', { id: 'a', issue: snap({ priority: 1, updatedAt: '2026-11-20T11:00:00.000Z' }) });
  assert.deepEqual(r.changed, ['set priority to Urgent']);
  let row = await b.get('a');
  assert.equal(row.title, 'My title');
  assert.equal(row.priority, 'urgent');
  assert.equal(row.notes, 'Mine');
  // Older or same answers change nothing.
  r = await b.l('apply', { id: 'a', issue: snap({ title: 'Stale', updatedAt: '2026-11-20T09:00:00.000Z' }) });
  assert.deepEqual(r.changed, []);
  r = await b.l('apply', { id: 'a', issue: snap({ title: 'Renamed', dueDate: null, priority: 0, statusType: 'completed', status: 'Done', updatedAt: '2026-11-20T12:00:00.000Z' }) });
  row = await b.get('a');
  assert.equal(row.title, 'Renamed');
  assert.equal(row.dueDate, null);
  assert.equal(row.priority, null);
  assert.equal(row.status, 'done');
  assert.equal(row.linear.status, 'Done');
  await b.l('apply', { id: 'a', issue: snap({ title: 'Renamed', dueDate: null, priority: 0, statusType: 'started', status: 'In Progress', updatedAt: '2026-11-20T13:00:00.000Z' }) });
  assert.equal((await b.get('a')).status, 'doing');
  await b.l('apply', { id: 'a', issue: snap({ title: 'Renamed', dueDate: null, priority: 0, statusType: 'canceled', status: 'Canceled', updatedAt: '2026-11-20T14:00:00.000Z' }) });
  row = await b.get('a');
  assert.equal(row.columnId, 'canceled');
  assert.equal(row.status, 'done');
  const events = (await b.t('activity', { id: 'a' })).items.filter((i: { author: { name: string } }) => i.author.name === 'Linear').map((i: { body: string }) => i.body);
  assert.ok(events.includes('canceled this in Linear'));
});

test('comments come in once, read-only, and only with sync on', async () => {
  const b = await setup();
  await linked(b);
  const c = { id: 'c1', body: 'Ship it', author: 'Sam', createdAt: '2026-11-20T11:00:00Z' };
  await b.l('apply', { id: 'a', issue: snap(), comments: [c] });
  await b.l('apply', { id: 'a', issue: snap(), comments: [c] });
  const items = (await b.t('activity', { id: 'a' })).items.filter((i: { source?: string }) => i.source === 'linear');
  assert.equal(items.length, 1);
  assert.equal(items[0].author.name, 'Sam in Linear');
  assert.equal((await b.get('a')).commentCount, 1);
  assert.equal(await outcome(b.t('removeComment', { id: 'a', commentId: items[0].id })), 'badArgs');
  await b.l('setSync', { id: 'a', sync: false });
  await b.l('apply', { id: 'a', issue: snap({ title: 'Ignored', status: 'Backlog', statusType: 'backlog', updatedAt: '2026-11-21T00:00:00.000Z' }), comments: [{ ...c, id: 'c2' }] });
  const row = await b.get('a');
  assert.equal(row.title, 'Fix it');
  assert.equal(row.linear.status, 'Backlog');
  assert.equal(row.status, 'todo');
  assert.equal((await b.t('activity', { id: 'a' })).items.filter((i: { source?: string }) => i.source === 'linear').length, 1);
  assert.deepEqual(await b.l('synced'), []);
});

test('import makes linked tasks once, sub-issues under their parent', async () => {
  const b = await setup();
  const item = (s: Record<string, unknown>, extra: Record<string, unknown> = {}) => ({ issue: snap(s), description: 'From Linear', teamId: 'team-1', teamName: 'Engineering', comments: [], ...extra });
  const r = await b.l('importIssues', {
    project: null,
    sync: true,
    issues: [item({ statusType: 'started', status: 'In Progress', priority: 3 }), item({ issueId: 'uuid-8', identifier: 'ENG-8', url: 'https://linear.app/acme/issue/ENG-8/y', title: 'Child' }, { parentIssueId: 'uuid-7', comments: [{ id: 'k1', body: 'hi', author: 'Ana', createdAt: '2026-11-20T11:00:00Z' }] })],
  });
  assert.deepEqual(r, { created: ['ENG-7', 'ENG-8'], skipped: [] });
  const parent = await b.get('linear-uuid-7');
  const child = await b.get('linear-uuid-8');
  assert.equal(parent.status, 'doing');
  assert.equal(parent.priority, 'normal');
  assert.equal(parent.notes, 'From Linear');
  assert.equal(child.parentId, 'linear-uuid-7');
  assert.equal(child.commentCount, 1);
  const again = await b.l('importIssues', { project: null, sync: true, issues: [item({})] });
  assert.deepEqual(again, { created: [], skipped: ['ENG-7'] });
  assert.deepEqual((await b.l('linkedIssues', { issueIds: ['uuid-7', 'nope'] })).map((x: { taskId: string }) => x.taskId), ['linear-uuid-7']);
  assert.equal((await b.l('synced')).length, 2);
});

test('missing stops sync, unlink forgets the issue, and the sweep runs once per interval', async () => {
  const b = await setup();
  await linked(b);
  await b.l('missing', { id: 'a', issueId: 'uuid-7' });
  let row = await b.get('a');
  assert.equal(row.linear.missing, true);
  assert.equal(row.linear.sync, false);
  await b.l('setSync', { id: 'a', sync: true });
  assert.equal((await b.get('a')).linear.missing, undefined);
  await b.l('unlink', { id: 'a' });
  row = await b.get('a');
  assert.equal(row.linear, undefined);
  assert.ok(!(row.links ?? []).some((x: { id: string }) => x.id === 'linear-issue'));
  assert.deepEqual(await b.l('claimSweep'), { since: null });
  assert.equal(await b.l('claimSweep'), null);
  assert.equal(await b.l('claimSweep', { force: true }), null);
  // Saving settings keeps the sweep's time.
  const before = (await b.t('preferences')).linear.lastSweepAt;
  await b.t('setPreferences', { linear: { syncEvery: '6h', defaults: [{ projectKey: 'none', teamId: 'team-1' }] } });
  const prefs = (await b.t('preferences')).linear;
  assert.equal(prefs.syncEvery, '6h');
  assert.equal(prefs.lastSweepAt, before);
});
