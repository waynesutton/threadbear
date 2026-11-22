import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildThreadbearTools, type ThreadControl, NEVER_ASK_TOOLS } from '../../src/threadbear-mcp.ts';
import { Tasks } from '../../src/tasks.ts';
const control = new Proxy({}, { get: () => () => undefined }) as ThreadControl;
test('task tools are absent while disabled and use the shared task bridge when enabled', async () => {
  const off = buildThreadbearTools(control, 'self');
  assert.equal(
    off.some((t) => t.name === 'create_task'),
    false,
  );
  const calls: [string, Record<string, unknown>][] = [];
  const tasks = new Tasks(
    () => ({
      key: 'owner',
      post: async (_kind, fn, args) => {
        calls.push([fn, args]);
        return fn === 'tasks:list' ? { items: [], next: null } : fn === 'tasks:projects' ? [] : 'created';
      },
    }),
    () => true,
    () => {},
  );
  const on = buildThreadbearTools(control, 'self', Date.now, undefined, undefined, undefined, { tasks: () => tasks });
  for (const name of ['list_tasks', 'create_task', 'update_task', 'delete_task']) assert.ok(on.some((t) => t.name === name));
  const invoke = async (name: string, args: Record<string, unknown>) => on.find((t) => t.name === name)!.handler(args as never, {});
  await invoke('list_tasks', {});
  await invoke('create_task', { id: 'retry', title: 'Review launch' });
  await invoke('update_task', { id: 'retry', status: 'done', project: null });
  await invoke('delete_task', { id: 'retry', undo: true });
  assert.deepEqual(
    calls.map(([fn]) => fn),
    ['tasks:list', 'tasks:projects', 'tasks:preferences', 'tasks:create', 'tasks:update', 'tasks:restore'],
  );
  assert.equal(NEVER_ASK_TOOLS.has('mcp__threadbear__list_tasks'), true);
  assert.equal(NEVER_ASK_TOOLS.has('mcp__threadbear__create_task'), false);
});

test('task tools take priority and due dates, and update_task comments as the agent', async () => {
  const calls: [string, Record<string, unknown>][] = [];
  const tasks = new Tasks(() => ({ key: 'owner', apiVersion: 104, post: async (_kind, fn, args) => { calls.push([fn, args]); return null; } }), () => true, () => {});
  const named = new Proxy({}, { get: (_, p) => (p === 'get' ? () => ({ title: '  Fix  the login ' }) : () => undefined) }) as ThreadControl;
  const on = buildThreadbearTools(named, 'self', Date.now, undefined, undefined, undefined, { tasks: () => tasks });
  const invoke = async (name: string, args: Record<string, unknown>) => on.find((t) => t.name === name)!.handler(args as never, {});
  const agent = { kind: 'agent', name: 'Fix the login' };
  await invoke('create_task', { id: 'a', title: 'Ship', priority: 'urgent', dueDate: '2026-11-20' });
  await invoke('update_task', { id: 'a', priority: null, comment: 'Done with the first pass' });
  await invoke('update_task', { id: 'a', comment: 'Only a comment' });
  assert.deepEqual(calls.map(([fn]) => fn), ['tasks:create', 'tasks:update', 'tasks:comment', 'tasks:comment']);
  assert.equal(calls[0]![1].priority, 'urgent');
  assert.equal(calls[0]![1].dueDate, '2026-11-20');
  for (const [, args] of calls) assert.deepEqual(args.author, agent);
  assert.equal(calls[1]![1].priority, null);
  assert.equal('comment' in calls[1]![1], false);
  assert.equal(calls[2]![1].body, 'Done with the first pass');
});
