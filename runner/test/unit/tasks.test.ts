import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
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


test('native photo upload preserves image MIME and selects the uploaded storage id', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'tasks-photo-'));
  try {
    const path = join(dir, 'backdrop.png');
    const bytes = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aD1sAAAAASUVORK5CYII=', 'base64');
    writeFileSync(path, bytes);
    const writes: unknown[] = [];
    const tasks = new Tasks(() => ({ key: 'owner', post: async (_kind, fn, args) => {
      if (fn === 'tasks:uploadPhoto') return 'https://example.test/upload';
      writes.push(args);
      return null;
    }, upload: async (url, body, mime) => {
      assert.equal(url, 'https://example.test/upload');
      assert.equal(mime, 'image/png');
      assert.deepEqual(Buffer.from(body), bytes);
      return 'photo-storage';
    } }), () => true, () => {});
    await tasks.call('photoFile', { path });
    assert.deepEqual(writes, [{ auth: { key: 'owner' }, backgroundPhoto: 'photo-storage' }]);
  } finally { rmSync(dir, { recursive: true, force: true }); }
});


test('new task controls give an update message against an older backend', async () => {
  const tasks = new Tasks(() => ({ key: 'owner', apiVersion: 85, post: async () => { throw Error('must not reach backend'); } }), () => true, () => {});
  for (const [operation, args] of [['startThread', {id:'task'}], ['create', {title:'Task', columnId:'todo'}], ['setPreferences', {columns:[]}]] as const) {
    await assert.rejects(tasks.call(operation, args), /Update the Threadbear backend/);
  }
});


test('native picker can upload bytes without granting the runner access to its source path', async () => {
  const base64 = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aD1sAAAAASUVORK5CYII=';
  let uploaded = 0;
  const tasks = new Tasks(() => ({ key: 'owner', post: async (_kind, fn) => fn === 'tasks:uploadPhoto' ? 'https://example.test/upload' : null,
    upload: async (_url, bytes, mime) => { uploaded++; assert.equal(mime, 'image/png'); assert.deepEqual(Buffer.from(bytes), Buffer.from(base64, 'base64')); return 'storage'; }
  }), () => true, () => {});
  await tasks.call('photoData', {base64});
  assert.equal(uploaded, 1);
  await assert.rejects(tasks.call('photoData', {base64: Buffer.from('not an image').toString('base64')}), /PNG/);
  await assert.rejects(tasks.call('photoData', {base64: '!invalid'}), /PNG/);
  assert.equal(uploaded, 1);
});
