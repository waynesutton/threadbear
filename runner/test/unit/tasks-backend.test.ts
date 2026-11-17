import { test } from 'node:test';
import assert from 'node:assert/strict';
import { call, load, memoryBackend, outcome, THREAD_BASICS } from '../helpers/convex-memory.ts';
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
  assert.equal((await b.run('preferences')).defaultView, 'list');
  await b.run('setPreferences', { defaultView: 'board' });
  await b.run('setPreferences', { defaultView: 'list' });
  await b.run('setPreferences', { backgroundColor: '#112233', fade: 0.6 });
  const prefs = await b.run('preferences');
  assert.equal(prefs.defaultView, 'list');
  assert.equal(prefs.backgroundColor, '#112233');
  assert.equal(prefs.fade, 0.6);
});
test('Doing is a status, and orders saved before it get it back', async () => {
  const b = await setup();
  await b.run('create', { id: 'w', title: 'Write it', status: 'doing', position: 5 });
  const row = (await b.run('list')).items.find((t: { id: string }) => t.id === 'w');
  assert.equal(row.status, 'doing');
  assert.equal(row.position, 5);
  const fresh = await b.run('preferences');
  assert.deepEqual(fresh.columnOrder, ['backlog', 'todo', 'doing', 'done']);
  assert.deepEqual(fresh.listOrder, ['doing', 'todo', 'backlog', 'done']);
  // An older client still sends three columns; Doing goes back after Todo.
  await b.run('setPreferences', { columnOrder: ['done', 'todo', 'backlog'] });
  assert.deepEqual((await b.run('preferences')).columnOrder, ['done', 'todo', 'doing', 'backlog']);
  await b.run('setPreferences', { listOrder: ['backlog', 'doing', 'todo', 'done'], projectOrder: ['mac/b', 'none', 'mac/a'] });
  const prefs = await b.run('preferences');
  assert.deepEqual(prefs.listOrder, ['backlog', 'doing', 'todo', 'done']);
  assert.deepEqual(prefs.projectOrder, ['mac/b', 'none', 'mac/a']);
  await assert.rejects(b.run('setPreferences', { columnOrder: ['todo', 'todo', 'done'] }), /each status once/);
  await assert.rejects(b.run('setPreferences', { projectOrder: ['mac/a', 'mac/a'] }), /each project once/);
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
    startThread: { id: 'never' },
    setPreferences: { defaultView: 'list' },
  };
  assert.deepEqual(Object.keys(tasks).sort(), Object.keys(cases).sort(), 'new operations need an auth case');
  for (const [operation, args] of Object.entries(cases)) {
    for (const auth of [{ token: 'invalid' }, { key: 'invalid' }]) {
      assert.equal(await outcome(b.run(operation, { ...args, auth })), 'auth', operation);
    }
  }
});

test('column names and custom membership survive edits without changing completion semantics', async () => {
  const b = await setup();
  const columns = [
    { id: 'backlog', name: 'Later', status: 'backlog' },
    { id: 'todo', name: 'Next', status: 'todo' },
    { id: 'doing', name: 'In progress', status: 'doing' },
    { id: 'done', name: 'Finished', status: 'done' },
    { id: 'review', name: 'Review', status: 'todo' },
  ];
  await b.run('setPreferences', { columns });
  await b.run('create', { id: 'custom-column-task', title: 'Review draft', columnId: 'review' });
  let task = (await b.run('list')).items[0];
  assert.equal(task.columnId, 'review');
  assert.equal(task.status, 'todo');
  await b.run('setPreferences', { columns: columns.map(c => c.id === 'review' ? { ...c, name: 'Ready for review' } : c) });
  await b.run('update', { id: task.id, notes: 'Keep my column' });
  task = (await b.run('list')).items[0];
  assert.equal(task.columnId, 'review');
  await b.run('update', { id: task.id, status: 'done' });
  task = (await b.run('list')).items[0];
  assert.equal(task.status, 'done');
  assert.notEqual(task.columnId, 'review');
});

test('columns reject missing built-ins, duplicate ids and invalid membership', async () => {
  const b = await setup();
  const columns = ['backlog', 'todo', 'doing', 'done'].map(status => ({ id: status, name: status, status }));
  assert.equal(await outcome(b.run('setPreferences', { columns: columns.slice(1) })), 'badArgs');
  assert.equal(await outcome(b.run('setPreferences', { columns: [...columns, columns[0]] })), 'badArgs');
  assert.equal(await outcome(b.run('create', { id: 'bad-column', title: 'Invalid', columnId: 'missing' })), 'badArgs');
  assert.equal(await outcome(b.run('create', { id: 'bad-status', title: 'Invalid', columnId: 'done', status: 'todo' })), 'badArgs');
});

test('sending a task queues one thread even after a repeated click', async () => {
  const b = await setup();
  await b.run('create', { id: 'thread-task', title: 'Build a page', notes: 'Use the existing theme' });
  const first = await b.run('startThread', { id: 'thread-task' });
  assert.equal(await b.run('startThread', { id: 'thread-task' }), first);
  const commands = await b.ctx.db.query('commands').collect();
  assert.equal(commands.length, 1);
  assert.equal(commands[0].kind, 'create');
  assert.match(commands[0].args.text, /Build a page/);
  assert.match(commands[0].args.text, /Use the existing theme/);
});


test('task launch carries project context and records the created thread across devices', async () => {
  const b = await setup();
  const sync = load('sync');
  await b.ctx.db.insert('projects', { runnerId: 'mac', id: 'project', title: 'Project', folders: ['/work/project'], goals: '', state: '', notes: '', createdBy: 'user', archived: false, createdAt: 'now', updatedAt: 'now', threadCount: 0 });
  await b.run('create', { id: 'launch', title: 'Build', project: { runnerId: 'mac', id: 'project' } });
  const commandId = await b.run('startThread', { id: 'launch', auth: { token: 'phone' } });
  const command = b.rows('commands').find(c => c._id === commandId)!;
  assert.equal(command.runnerId, 'mac');
  assert.deepEqual(command.args.projectIds, ['project']);
  assert.equal(command.args.cwd, '/work/project');
  await call(b.ctx, sync.upsertThreads, { key: 'task-owner', threads: [{ ...THREAD_BASICS, threadId: 'made', runnerId: 'mac', title: 'Build', view: 'inbox', needsYou: false }] });
  await call(b.ctx, sync.finishCommand, { key: 'task-owner', commandId, ok: true, resultThreadId: 'made' });
  const row = (await b.run('list', { auth: { token: 'phone' } })).items[0];
  assert.deepEqual(row.thread, { runnerId: 'mac', id: 'made' });
  assert.equal(await b.run('startThread', { id: 'launch' }), commandId);
});

test('task launch exposes failure and refuses deleted tasks', async () => {
  const b = await setup();
  await b.run('create', { id: 'launch', title: 'Build' });
  const commandId = await b.run('startThread', { id: 'launch' });
  await call(b.ctx, load('sync').finishCommand, { key: 'task-owner', commandId, ok: false, note: 'No available account' });
  assert.equal((await b.run('list')).items[0].threadError, 'No available account');
  await b.run('remove', { id: 'launch' });
  assert.equal(await outcome(b.run('startThread', { id: 'launch' })), 'badArgs');
});

test('a scoped runner may edit shared tasks but cannot queue agent work on another machine', async () => {
  const b = await setup();
  const key = 'tbr_tasks-secondary';
  await b.ctx.db.insert('runnerKeys', { runnerId: 'secondary', keyHash: await sha256(key), name: 'Secondary', createdAt: 1 });
  await b.run('create', { id: 'scoped', title: 'Shared task', auth: { key } });
  assert.equal(await outcome(b.run('startThread', { id: 'scoped', auth: { key } })), 'forbidden');
  assert.equal(b.rows('commands').length, 0);
});
