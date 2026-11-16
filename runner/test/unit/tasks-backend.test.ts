import { test } from 'node:test';
import assert from 'node:assert/strict';
import { call, load, memoryBackend, outcome } from '../helpers/convex-memory.ts';
const tasks = load('tasks');
const { sha256 } = load('lib');
async function setup() {
  process.env.RUNNER_KEY = 'task-owner';
  delete process.env.RUNNER_KEY_NEXT;
  const b = memoryBackend();
  await b.ctx.db.insert('tokens', {
    hash: await sha256('phone'),
    name: 'browser',
    expiresAt: Date.now() + 60000,
  });
  const auth = { key: 'task-owner' };
  const run = (name: string, args: Record<string, unknown> = {}) => call(b.ctx, tasks[name], { auth, ...args });
  return { ...b, run };
}
test('tasks require authentication and accept paired browsers', async () => {
  const b = await setup();
  assert.equal(await outcome(b.run('list', { auth: { key: 'bad' } })), 'auth');
  assert.equal(await outcome(b.run('list', { auth: { token: 'bad' } })), 'auth');
  assert.deepEqual((await b.run('list', { auth: { token: 'phone' } })).items, []);
});
test('create retries do not duplicate or overwrite and updates patch only given fields', async () => {
  const b = await setup();
  await b.run('create', { id: 'a', title: 'First', notes: 'Keep this' });
  await b.run('create', { id: 'a', title: 'Retry' });
  await b.run('update', { id: 'a', title: 'Edited', status: 'done' });
  const { items } = await b.run('list');
  assert.equal(items.length, 1);
  assert.equal(items[0].title, 'Edited');
  assert.equal(items[0].notes, 'Keep this');
  assert.equal(items[0].status, 'done');
  assert.ok(items[0].position > 0);
});
test('delete and restore are idempotent and preserve the task', async () => {
  const b = await setup();
  await b.run('create', { id: 'a', title: 'Recover me' });
  await b.run('remove', { id: 'a' });
  await b.run('remove', { id: 'a' });
  assert.equal((await b.run('list')).items.length, 0);
  await b.run('restore', { id: 'a' });
  await b.run('restore', { id: 'a' });
  assert.equal((await b.run('list')).items[0].title, 'Recover me');
});
test('invalid projects, empty titles and invalid column order are refused', async () => {
  const b = await setup();
  assert.equal(await outcome(b.run('create', { id: 'a', title: ' ' })), 'badArgs');
  assert.equal(
    await outcome(
      b.run('create', {
        id: 'a',
        title: 'Task',
        project: { runnerId: 'mac', id: 'missing' },
      }),
    ),
    'badArgs',
  );
  assert.equal(await outcome(b.run('setPreferences', { columnOrder: ['todo', 'todo', 'done'] })), 'badArgs');
});
test('default view is explicit and preference patches preserve other choices', async () => {
  const b = await setup();
  assert.equal((await b.run('preferences')).defaultView, 'board');
  await b.run('setPreferences', { defaultView: 'list' });
  await b.run('setPreferences', { backgroundColor: '#112233', fade: 0.6 });
  const prefs = await b.run('preferences');
  assert.equal(prefs.defaultView, 'list');
  assert.equal(prefs.backgroundColor, '#112233');
  assert.equal(prefs.fade, 0.6);
});
test('rapid updates always invalidate subscribers and pages do not repeat tasks', async () => {
  const b = await setup();
  await b.run('create', { id: 'a', title: 'A' });
  const first = await b.run('revision');
  await b.run('update', { id: 'a', title: 'B' });
  assert.ok((await b.run('revision')) > first);
  await b.run('create', { id: 'b', title: 'B' });
  const page = await b.run('list', { limit: 1 });
  assert.equal(page.next, 'a');
  assert.deepEqual(
    (await b.run('list', { after: page.next })).items.map((t: { id: string }) => t.id),
    ['b'],
  );
});
test('project task counts exclude deleted tasks and deleting a project leaves tasks unassigned', async () => {
  const b = await setup();
  const project = { runnerId: 'mac', id: 'p1' };
  const dbId = await b.ctx.db.insert('projects', {
    ...project,
    title: 'Launch',
    archived: false,
  });
  await b.run('create', { id: 'assigned', title: 'Ship it', project });
  await b.run('create', { id: 'loose', title: 'Read a book' });
  assert.deepEqual(await b.run('count', { project }), {
    count: 1,
    more: false,
  });
  await b.run('remove', { id: 'assigned' });
  assert.equal((await b.run('count', { project })).count, 0);
  await b.run('restore', { id: 'assigned' });
  await b.ctx.db.delete(dbId);
  const items = (await b.run('list')).items;
  assert.equal(items.find((t: { id: string }) => t.id === 'assigned').project, null);
  assert.equal(items.length, 2);
});
test('revoked browser tokens cannot read or write Tasks', async () => {
  const b = await setup();
  const token = await b.ctx.db.query('tokens').first();
  await b.ctx.db.delete(token._id);
  for (const operation of ['list', 'preferences', 'uploadPhoto', 'revision']) {
    assert.equal(await outcome(b.run(operation, { auth: { token: 'phone' } })), 'auth');
  }
  assert.equal(
    await outcome(
      b.run('create', {
        auth: { token: 'phone' },
        id: 'blocked',
        title: 'Blocked',
      }),
    ),
    'auth',
  );
});

test('every public task operation rejects an invalid account credential', async () => {
  const b = await setup();
  const cases: Record<string, Record<string, unknown>> = {
    list: {},
    preferences: {},
    projects: {},
    photo: {},
    revision: {},
    uploadPhoto: {},
    count: { project: { runnerId: 'mac', id: 'p1' } },
    create: { id: 'never', title: 'Never created' },
    update: { id: 'never', title: 'Never edited' },
    remove: { id: 'never' },
    restore: { id: 'never' },
    setPreferences: { defaultView: 'list' },
  };
  assert.deepEqual(Object.keys(tasks).sort(), Object.keys(cases).sort(), 'new operations need an auth case');
  for (const [operation, args] of Object.entries(cases)) {
    for (const auth of [{ token: 'invalid' }, { key: 'invalid' }]) {
      assert.equal(await outcome(b.run(operation, { ...args, auth })), 'auth', operation);
    }
  }
});
