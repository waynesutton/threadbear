import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Tasks } from '../../src/tasks.ts';
test('Tasks bridge supplies its own credentials and preserves retry ids', async () => {
  const calls: unknown[] = [];
  let changed = 0;
  const tasks = new Tasks(
    () => ({
      key: 'private',
      post: async (...a) => {
        calls.push(a);
        return 'ok';
      },
    }),
    () => true,
    () => changed++,
  );
  await tasks.call('create', {
    id: 'same-id',
    title: 'Task',
    auth: { key: 'injected' },
  });
  assert.deepEqual(calls, [['mutation', 'tasks:create', { id: 'same-id', title: 'Task', auth: { key: 'private' } }]]);
  assert.equal(changed, 1);
});
test('Tasks bridge rejects disabled, offline and unknown operations', async () => {
  await assert.rejects(
    new Tasks(
      () => ({ key: 'owner', apiVersion: 83, post: async () => { throw new Error('Must not reach the old backend'); } }),
      () => true,
      () => {},
    ).call('create', { title: 'Work in progress', status: 'doing' }),
    /Update the Threadbear backend/,
  );
  await assert.rejects(
    new Tasks(
      () => undefined,
      () => false,
      () => {},
    ).call('list'),
    /Turn on Tasks/,
  );
  await assert.rejects(
    new Tasks(
      () => undefined,
      () => true,
      () => {},
    ).call('create'),
    /has not been saved/,
  );
  await assert.rejects(
    new Tasks(
      () => undefined,
      () => true,
      () => {},
    ).call('arbitrary'),
    /Unknown/,
  );
});

test('Tasks resolve project names and refuse ambiguous matches', async () => {
  const calls: unknown[] = [];
  const tasks = new Tasks(
    () => ({
      key: 'owner',
      post: async (_kind, fn, args) => {
        if (fn === 'tasks:projects')
          return [
            { id: 'p1', runnerId: 'mac', title: 'Build' },
            { id: 'p2', runnerId: 'other', title: 'Build' },
          ];
        calls.push(args);
        return null;
      },
    }),
    () => true,
    () => {},
  );
  await assert.rejects(tasks.call('create', { title: 'Task', project: 'Build' }), /Several projects/);
  await assert.rejects(tasks.call('create', { title: 'Task', project: 'Missing' }), /No matching project/);
  await tasks.call('create', { id: 'retry', title: 'Task', project: 'p1' });
  assert.deepEqual(calls, [
    {
      id: 'retry',
      title: 'Task',
      project: { id: 'p1', runnerId: 'mac' },
      auth: { key: 'owner' },
    },
  ]);
});
test('Tasks subscriptions follow connection replacement and stop when disabled', async () => {
  let connection = {},
    enabled = true,
    starts = 0,
    stops = 0;
  const tasks = new Tasks(
    () => ({
      key: 'owner',
      connection,
      post: async () => null,
      subscribe: () => {
        starts++;
        return () => {
          stops++;
        };
      },
    }),
    () => enabled,
    () => {},
  );
  await tasks.call('list');
  await tasks.call('list');
  assert.equal(starts, 3);
  connection = {};
  await tasks.call('list');
  assert.equal(starts, 6);
  assert.equal(stops, 3);
  enabled = false;
  await assert.rejects(tasks.call('list'), /Turn on Tasks/);
  assert.equal(stops, 6);
});
