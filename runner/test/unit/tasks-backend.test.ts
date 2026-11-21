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
    get: { id: 'never' },
    activity: { id: 'never' },
    comment: { id: 'never', body: 'Never said' },
    removeComment: { id: 'never', commentId: 'never' },
    addImage: { id: 'never', storageId: 'stored1' },
    claimReminder: { id: 'never', key: '2026-11-20T09:30' },
    deleteColumn: { id: 'paused' },
  };
  // moveColumnTasks is internal: only deleteColumn schedules it.
  // Registered functions only: tasks.ts also exports helpers tasksLinear.ts shares.
  const registered = Object.entries(tasks).filter(([, f]) => typeof (f as { handler?: unknown } | undefined)?.handler === 'function').map(([k]) => k);
  assert.deepEqual(registered.filter((k) => k !== 'moveColumnTasks').sort(), Object.keys(cases).sort(), 'new operations need an auth case');
  for (const [operation, args] of Object.entries(cases)) {
    for (const auth of [{ token: 'invalid' }, { key: 'invalid' }]) {
      assert.equal(await outcome(b.run(operation, { ...args, auth })), 'auth', operation);
    }
  }
});

test('column names and custom membership survive edits without changing completion semantics', async () => {
  const b = await setup();
  const columns = [...(await b.run('preferences')).columns, { id: 'review', name: 'Review', status: 'todo' }];
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

test('columns reject dropped columns, duplicate ids and invalid membership', async () => {
  const b = await setup();
  const columns = (await b.run('preferences')).columns;
  assert.equal(await outcome(b.run('setPreferences', { columns: columns.slice(1) })), 'badArgs');
  assert.equal(await outcome(b.run('setPreferences', { columns: columns.filter((c: { id: string }) => c.id !== 'todo') })), 'badArgs');
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


test('sending a task carries its pictures without handing their files to the command', async () => {
  const b = await setup();
  const shot = b.file('shot1', { size: 1000, contentType: 'image/png' });
  await b.run('create', { id: 'pics', title: 'Fix the header', notes: 'See the screenshot', links: [
    { id: 'l1', kind: 'link', title: 'Docs', url: 'https://example.com' },
    { id: 'l2', kind: 'image', title: 'Shot', storageId: shot },
  ] });
  const commandId = await b.run('startThread', { id: 'pics' });
  const command = b.rows('commands').find(c => c._id === commandId)!;
  assert.deepEqual(command.args.taskImages, [{ url: 'https://files.example/shot1' }]);
  assert.equal(command.args.images, undefined);
  assert.match(command.args.text, /See the screenshot\n\nPictures attached to this task:\n\[Picture 1\]$/);
  // Finishing the thread leaves the task's picture alone.
  // Taken first: Mike's command machine finishes only a taken command.
  await call(b.ctx, load('sync').takeCommand, { key: 'task-owner', commandId });
  await call(b.ctx, load('sync').finishCommand, { key: 'task-owner', commandId, ok: false, note: 'x' });
  assert.deepEqual(b.deleted, []);
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
  // Taken first: Mike's command machine finishes only a taken command.
  await call(b.ctx, sync.takeCommand, { key: 'task-owner', commandId });
  await call(b.ctx, sync.finishCommand, { key: 'task-owner', commandId, ok: true, resultThreadId: 'made' });
  const row = (await b.run('list', { auth: { token: 'phone' } })).items[0];
  assert.deepEqual(row.thread, { runnerId: 'mac', id: 'made' });
  assert.equal(await b.run('startThread', { id: 'launch' }), commandId);
});

test('task launch exposes failure and refuses deleted tasks', async () => {
  const b = await setup();
  await b.run('create', { id: 'launch', title: 'Build' });
  const commandId = await b.run('startThread', { id: 'launch' });
  // Taken first: Mike's command machine finishes only a taken command.
  await call(b.ctx, load('sync').takeCommand, { key: 'task-owner', commandId });
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


// ---- API 87: issue fields, sub-tasks, links, comments and activity.
const user = { kind: 'user', name: 'Wayne' };
const agent = { kind: 'agent', name: 'Fix the login' };
const bodies = async (b: Awaited<ReturnType<typeof setup>>, id: string, auth?: unknown) =>
  (await b.run('activity', { id, ...(auth ? { auth } : {}) })).items.map((i: { author: { name: string }; body: string }) => `${i.author.name} ${i.body}`);

test('priority and due date are kept, cleared with null, and old rows read without them', async () => {
  const b = await setup();
  await b.run('create', { id: 'old', title: 'From before' });
  await b.run('create', { id: 'p', title: 'Urgent one', priority: 'urgent', dueDate: '2026-11-20', author: user });
  let row = (await b.run('list')).items.find((t: { id: string }) => t.id === 'p');
  assert.equal(row.priority, 'urgent');
  assert.equal(row.dueDate, '2026-11-20');
  assert.deepEqual(row.createdBy, user);
  const old = (await b.run('list')).items.find((t: { id: string }) => t.id === 'old');
  assert.equal(old.priority, undefined);
  await b.run('update', { id: 'p', priority: null, dueDate: null });
  row = (await b.run('list')).items.find((t: { id: string }) => t.id === 'p');
  assert.equal(row.priority, null);
  assert.equal(row.dueDate, null);
  for (const dueDate of ['2026-02-30', '20-11-2026', 'tomorrow']) assert.equal(await outcome(b.run('update', { id: 'p', dueDate })), 'badArgs', dueDate);
});

test('a due time rides on a due date, and one Mac claims each reminder', async () => {
  const b = await setup();
  await b.run('create', { id: 't', title: 'Call Sam', dueDate: '2026-11-20', dueTime: '09:30' });
  let row = (await b.run('list')).items.find((t: { id: string }) => t.id === 't');
  assert.equal(row.dueTime, '09:30');
  for (const dueTime of ['9:30', '24:00', '09:60', 'noon']) assert.equal(await outcome(b.run('update', { id: 't', dueTime })), 'badArgs', dueTime);
  assert.equal(await b.run('claimReminder', { id: 't', key: '2026-11-20T09:30' }), true);
  assert.equal(await b.run('claimReminder', { id: 't', key: '2026-11-20T09:30' }), false);
  assert.equal(await b.run('claimReminder', { id: 't', key: '2026-11-20T08:00' }), false);
  // A new time reminds again.
  await b.run('update', { id: 't', dueTime: '10:00' });
  assert.equal(await b.run('claimReminder', { id: 't', key: '2026-11-20T10:00' }), true);
  // Clearing the date clears the time; a time needs a date.
  await b.run('update', { id: 't', dueDate: null });
  row = (await b.run('list')).items.find((t: { id: string }) => t.id === 't');
  assert.equal(row.dueTime, null);
  assert.equal(await outcome(b.run('update', { id: 't', dueTime: '10:00' })), 'badArgs');
  await b.run('setPreferences', { reminders: { mac: true, channel: 'slack', to: ' #ops ', lead: 15 } });
  assert.deepEqual((await b.run('preferences')).reminders, { mac: true, channel: 'slack', to: '#ops', lead: 15 });
  assert.equal(await outcome(b.run('setPreferences', { reminders: { mac: true, channel: 'none', to: '', lead: -1 } })), 'badArgs');
});

test('the backend writes events for status, priority, due date, project and parent, by who did them', async () => {
  const b = await setup();
  await b.ctx.db.insert('projects', { runnerId: 'mac', id: 'launch', title: 'Launch', archived: false });
  await b.run('create', { id: 'parent', title: 'Ship it', author: user });
  await b.run('create', { id: 'child', title: 'Write notes', author: agent });
  await b.run('update', { id: 'child', status: 'doing', priority: 'high', dueDate: '2026-11-20', project: { runnerId: 'mac', id: 'launch' }, parentId: 'parent', author: user });
  // A title-only edit, and a repeat of the same values, say nothing.
  await b.run('update', { id: 'child', title: 'Write the notes', priority: 'high', author: user });
  assert.deepEqual(await bodies(b, 'child'), [
    'Fix the login created the task',
    'Wayne moved to Shaping',
    'Wayne set priority to High',
    'Wayne set the due date to Nov 20, 2026',
    'Wayne moved to project Launch',
    'Wayne made this a sub-task of Ship it',
  ]);
  assert.deepEqual(await bodies(b, 'parent'), ['Wayne created the task', 'Wayne added sub-task Write notes']);
  // A browser is the user, by the name the runner last gave; it can't claim to be an agent.
  await b.run('update', { id: 'child', priority: 'urgent', auth: { token: 'phone' }, author: agent });
  assert.equal((await bodies(b, 'child')).at(-1), 'Wayne set priority to Urgent');
  const item = (await b.run('activity', { id: 'child' })).items.at(-1);
  assert.deepEqual(item.author, { kind: 'user', name: 'Wayne' });
  assert.equal(item.kind, 'event');
});

test('sub-tasks are one level deep', async () => {
  const b = await setup();
  await b.run('create', { id: 'a', title: 'A' });
  await b.run('create', { id: 'b', title: 'B', parentId: 'a' });
  await b.run('create', { id: 'c', title: 'C' });
  assert.equal(await outcome(b.run('create', { id: 'd', title: 'D', parentId: 'b' })), 'badArgs');
  assert.equal(await outcome(b.run('update', { id: 'a', parentId: 'c' })), 'badArgs');
  assert.equal(await outcome(b.run('update', { id: 'c', parentId: 'c' })), 'badArgs');
  assert.equal(await outcome(b.run('update', { id: 'c', parentId: 'missing' })), 'badArgs');
  await b.run('update', { id: 'b', parentId: null });
  assert.equal((await b.run('list')).items.find((t: { id: string }) => t.id === 'b').parentId, null);
  assert.equal((await bodies(b, 'a')).at(-1), 'You removed sub-task B');
});

test('deleting a parent deletes its sub-tasks and restoring brings back only those', async () => {
  const b = await setup();
  await b.run('create', { id: 'p', title: 'Parent' });
  await b.run('create', { id: 'k1', title: 'Kid one', parentId: 'p' });
  await b.run('create', { id: 'k2', title: 'Kid two', parentId: 'p' });
  await b.run('remove', { id: 'k2' });
  await b.run('remove', { id: 'p' });
  assert.deepEqual((await b.run('list')).items, []);
  await b.run('restore', { id: 'p' });
  const items = (await b.run('list')).items;
  assert.deepEqual(items.map((t: { id: string }) => t.id), ['k1', 'p']);
  assert.equal('deletedWithParent' in items[0], false);
  // A sub-task restored while its parent is gone stands alone.
  await b.run('remove', { id: 'p' });
  await b.run('restore', { id: 'k1' });
  assert.equal((await b.run('list')).items.find((t: { id: string }) => t.id === 'k1').parentId, null);
});

test('comments count, list oldest first with events, and remove', async () => {
  const b = await setup();
  await b.run('create', { id: 't', title: 'Talk about it' });
  const first = await b.run('comment', { id: 't', body: '  First thought  ', author: agent });
  assert.equal(first.body, 'First thought');
  assert.deepEqual(first.author, agent);
  await b.run('comment', { id: 't', body: 'From the web', auth: { token: 'phone' } });
  let row = (await b.run('list')).items[0];
  assert.equal(row.commentCount, 2);
  const items = (await b.run('activity', { id: 't' })).items;
  assert.deepEqual(items.map((i: { kind: string }) => i.kind), ['event', 'comment', 'comment']);
  assert.equal(await outcome(b.run('comment', { id: 't', body: '   ' })), 'badArgs');
  await b.run('removeComment', { id: 't', commentId: first.id });
  await b.run('removeComment', { id: 't', commentId: first.id });
  // An event isn't a comment to remove.
  await b.run('removeComment', { id: 't', commentId: items[0].id });
  row = (await b.run('list')).items[0];
  assert.equal(row.commentCount, 1);
  assert.equal((await b.run('activity', { id: 't' })).items.length, 2);
});

test('links are checked, capped at 50, replaced whole, and image links carry their URL', async () => {
  const b = await setup();
  const png = b.file('img1', { size: 1000, contentType: 'image/png' });
  const text = b.file('txt1', { size: 10, contentType: 'text/plain' });
  await b.run('create', { id: 't', title: 'Linked' });
  const links = [
    { id: 'l1', kind: 'link', title: '', url: 'https://example.com/a' },
    { id: 'l2', kind: 'print', title: 'Report', printId: 'report-1' },
    { id: 'l3', kind: 'thread', title: 'The thread', thread: { runnerId: 'mac', id: 'th1' } },
    { id: 'l4', kind: 'image', title: 'Shot', storageId: png },
  ];
  await b.run('update', { id: 't', links });
  let row = (await b.run('list')).items[0];
  assert.equal(row.links[0].title, 'https://example.com/a');
  assert.equal(row.links[3].imageUrl, 'https://files.example/img1');
  for (const bad of [
    [{ id: 'x', kind: 'link', title: 'No url' }],
    [{ id: 'x', kind: 'link', title: 'Script', url: 'javascript:alert(1)' }],
    [{ id: 'x', kind: 'image', title: 'Not a picture', storageId: text }],
    [{ id: 'x', kind: 'print', title: 'No print' }],
    [links[0], links[0]],
    Array.from({ length: 51 }, (_, i) => ({ id: `l${i}`, kind: 'link', title: 'x', url: 'https://example.com' })),
  ])
    assert.equal(await outcome(b.run('update', { id: 't', links: bad })), 'badArgs');
  // Dropping an image link deletes its file.
  await b.run('update', { id: 't', links: links.slice(0, 1) });
  assert.deepEqual(b.deleted, ['img1']);
  // addImage appends once per file.
  const pic = b.file('img2', { size: 2000, contentType: 'image/jpeg' });
  const link = await b.run('addImage', { id: 't', storageId: pic, title: 'photo.jpg' });
  assert.equal((await b.run('addImage', { id: 't', storageId: pic })).id, link.id);
  row = (await b.run('list')).items[0];
  assert.deepEqual(row.links.map((l: { kind: string }) => l.kind), ['link', 'image']);
  assert.equal(row.links[1].title, 'photo.jpg');
  assert.equal(await outcome(b.run('addImage', { id: 't', storageId: text })), 'badArgs');
});

test('get finds one task, and a deleted one too', async () => {
  const b = await setup();
  assert.equal(await b.run('get', { id: 'nope' }), null);
  await b.run('create', { id: 'g', title: 'Get me', priority: 'low' });
  await b.run('remove', { id: 'g' });
  const row = await b.run('get', { id: 'g' });
  assert.equal(row.priority, 'low');
  assert.equal(row.deleted, true);
});

test('a new board starts on the eight product columns, Building first in the list', async () => {
  const b = await setup();
  const prefs = await b.run('preferences');
  assert.deepEqual(prefs.columns.map((c: { name: string }) => c.name), ['Backlog', 'On Deck', 'Shaping', 'Building', 'Observing', 'Done', 'Paused', 'Canceled']);
  assert.deepEqual(prefs.boardColumnOrder, ['backlog', 'on-deck', 'doing', 'todo', 'observing', 'done', 'paused', 'canceled']);
  assert.equal(prefs.sectionOrder[0], 'todo');
  await b.run('create', { id: 'new', title: 'Quick add' });
  await b.run('create', { id: 'stop', title: 'Not now', columnId: 'canceled' });
  await b.run('create', { id: 'named', title: 'By name', columnId: 'observing' });
  await b.run('update', { id: 'named', columnId: 'on deck' });
  const byId = Object.fromEntries((await b.run('list')).items.map((t: { id: string }) => [t.id, t]));
  assert.equal(byId.new.columnId, 'todo');
  assert.equal(byId.stop.status, 'done');
  assert.equal(byId.named.columnId, 'on-deck');
  // A status alone keeps a task in its column when the status already matches.
  await b.run('update', { id: 'named', status: 'todo' });
  assert.equal((await b.run('get', { id: 'named' })).columnId, 'on-deck');
});
test('deleting a column moves its tasks to one with the same status, and Building stays', async () => {
  const b = await setup();
  await b.ctx.db.insert('tasks', { id: 'old', title: 'From before columns', notes: '', status: 'doing', project: null, position: 1, createdAt: 1, updatedAt: 1, deleted: false });
  await b.run('create', { id: 'c', title: 'Dropped idea', columnId: 'canceled' });
  await b.run('create', { id: 'gone', title: 'In the bin', columnId: 'canceled' });
  await b.run('remove', { id: 'gone' });
  assert.deepEqual(await b.run('deleteColumn', { id: 'canceled' }), { movedTo: 'done' });
  assert.equal(await b.run('deleteColumn', { id: 'canceled' }), null);
  await b.run('restore', { id: 'gone' });
  // Shaping goes to Observing, the other doing column; then Observing to Building.
  assert.deepEqual(await b.run('deleteColumn', { id: 'doing' }), { movedTo: 'observing' });
  assert.deepEqual(await b.run('deleteColumn', { id: 'observing' }), { movedTo: 'todo' });
  const byId = Object.fromEntries((await b.run('list')).items.map((t: { id: string }) => [t.id, t]));
  assert.equal(byId.c.columnId, 'done');
  assert.equal(byId.gone.columnId, 'done');
  assert.equal(byId.old.columnId, 'todo');
  assert.equal(byId.old.status, 'todo');
  const prefs = await b.run('preferences');
  assert.ok(!prefs.columns.some((c: { id: string }) => ['canceled', 'doing', 'observing'].includes(c.id)));
  assert.ok(!prefs.boardColumnOrder.includes('doing'));
  assert.equal(await outcome(b.run('deleteColumn', { id: 'todo' })), 'badArgs');
  // Older clients still name deleted built-in columns and doing.
  await b.run('update', { id: 'c', columnId: 'doing' });
  assert.equal((await b.run('get', { id: 'c' })).columnId, 'todo');
  await b.run('setPreferences', { boardColumnOrder: ['doing', 'todo', 'backlog'] });
  assert.deepEqual((await b.run('preferences')).boardColumnOrder.slice(0, 2), ['todo', 'backlog']);
});
test('a deleted column with many tasks keeps moving them in batches', async () => {
  const b = await setup();
  for (let i = 0; i < 205; i++) await b.ctx.db.insert('tasks', { id: `p${i}`, title: 'Wait', notes: '', status: 'backlog', columnId: 'paused', project: null, position: i, createdAt: 1, updatedAt: 1, deleted: false });
  await b.run('deleteColumn', { id: 'paused' });
  assert.equal(b.scheduled.length, 1);
  const { moveColumnTasks } = tasks;
  await call(b.ctx, moveColumnTasks, (b.scheduled[0] as { args: Record<string, unknown> }).args);
  const items = [];
  for (let after: string | null = ''; after !== null; ) {
    const page = await b.run('list', { after, limit: 250 });
    items.push(...page.items);
    after = page.next;
  }
  assert.ok(items.every((t: { columnId: string }) => t.columnId === 'backlog'));
  const raw = await b.ctx.db.query('tasks').withIndex('by_column_status', (q: any) => q.eq('columnId', 'paused')).take(10);
  assert.equal(raw.length, 0);
});
