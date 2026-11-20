import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Tasks, relatedFor } from '../../src/tasks.ts';
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
      () => ({ key: 'owner', apiVersion: 98, post: async () => { throw new Error('Must not reach the old backend'); } }),
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
  const tasks = new Tasks(() => ({ key: 'owner', apiVersion: 100, post: async () => { throw Error('must not reach backend'); } }), () => true, () => {});
  for (const [operation, args] of [['startThread', {id:'task'}], ['create', {title:'Task', columnId:'todo'}], ['setPreferences', {columns:[]}]] as const) {
    await assert.rejects(tasks.call(operation, args), /Update the Threadbear backend/);
  }
});

test('deleting a column gives an update message against a backend before 89', async () => {
  const tasks = new Tasks(() => ({ key: 'owner', apiVersion: 103, post: async () => { throw Error('must not reach backend'); } }), () => true, () => {});
  await assert.rejects(tasks.call('deleteColumn', { id: 'paused' }), /delete task columns/);
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


test('issue fields and comments give an update message against a backend before 87', async () => {
  const tasks = new Tasks(() => ({ key: 'owner', apiVersion: 101, post: async () => { throw Error('must not reach backend'); } }), () => true, () => {});
  for (const [operation, args] of [['update', { id: 't', priority: 'urgent' }], ['update', { id: 't', dueDate: '2026-11-20' }], ['create', { title: 'T', parentId: 'p' }], ['update', { id: 't', links: [] }], ['comment', { id: 't', body: 'Hi' }], ['activity', { id: 't' }], ['removeComment', { id: 't', commentId: 'c' }], ['imageData', { id: 't', base64: '' }]] as const) {
    await assert.rejects(tasks.call(operation, args), /Update the Threadbear backend/, operation);
  }
  // Plain edits still go to an 86 backend, without an author it would refuse.
  const sent: unknown[] = [];
  await new Tasks(() => ({ key: 'owner', apiVersion: 101, post: async (_k, _f, a) => void sent.push(a) }), () => true, () => {}).call('update', { id: 't', title: 'Plain', author: { kind: 'agent', name: 'x' } });
  assert.deepEqual(sent, [{ id: 't', title: 'Plain', auth: { key: 'owner' } }]);
});

test('writes carry who they are from, and a client cannot choose it', async () => {
  const sent: [string, Record<string, unknown>][] = [];
  const tasks = new Tasks(() => ({ key: 'owner', apiVersion: 102, post: async (_k, fn, a) => void sent.push([fn, a]) }), () => true, () => {});
  await tasks.call('comment', { id: 't', body: 'Hi', author: { kind: 'agent', name: 'Forged' } });
  await tasks.call('update', { id: 't', priority: 'high' }, { kind: 'agent', name: 'Fix login' });
  assert.equal((sent[0]![1].author as { kind: string }).kind, 'user');
  assert.notEqual((sent[0]![1].author as { name: string }).name, 'Forged');
  assert.deepEqual(sent[1]![1].author, { kind: 'agent', name: 'Fix login' });
  // Reads and deletes have no author.
  await tasks.call('activity', { id: 't' });
  await tasks.call('remove', { id: 't' });
  assert.equal('author' in sent[2]![1], false);
  assert.equal('author' in sent[3]![1], false);
});

test('a task image uploads checked bytes and appends a link', async () => {
  const base64 = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aD1sAAAAASUVORK5CYII=';
  const calls: [string, unknown][] = [];
  const tasks = new Tasks(() => ({ key: 'owner', apiVersion: 102, post: async (_k, fn, a) => { calls.push([fn, a]); return fn === 'tasks:uploadPhoto' ? 'https://example.test/upload' : { id: 'image-s' }; },
    upload: async (_url, _bytes, mime) => { assert.equal(mime, 'image/png'); return 'storage-1'; } }), () => true, () => {});
  assert.deepEqual(await tasks.call('imageData', { id: 't', base64, name: 'shot.png' }), { id: 'image-s' });
  assert.deepEqual(calls.at(-1), ['tasks:addImage', { id: 't', storageId: 'storage-1', title: 'shot.png', auth: { key: 'owner' } }]);
  await assert.rejects(tasks.call('imageData', { id: 't', base64: Buffer.from('not an image').toString('base64') }), /PNG/);
  const dir = mkdtempSync(join(tmpdir(), 'tasks-image-'));
  try {
    writeFileSync(join(dir, 'pic.png'), Buffer.from(base64, 'base64'));
    await tasks.call('imageFile', { id: 't', path: join(dir, 'pic.png') });
    assert.deepEqual(calls.at(-1), ['tasks:addImage', { id: 't', storageId: 'storage-1', title: 'pic.png', auth: { key: 'owner' } }]);
  } finally { rmSync(dir, { recursive: true, force: true }); }
});

test('related lists this runner\'s threads and prints for a task in one of its projects', async () => {
  const related = relatedFor({
    threadIds: (p) => (p === 'p1' ? ['old', 'gone', 'new', ...Array.from({ length: 12 }, (_, i) => `t${i}`)] : undefined),
    thread: (id) => (id === 'gone' ? undefined : { id, title: id, status: 'idle', lastActivityAt: id === 'new' ? '2026-11-18T00:00:00Z' : id === 'old' ? '2026-01-01T00:00:00Z' : '2026-06-01T00:00:00Z', cwd: '/x' }),
    prints: () => [{ id: 'a', title: 'A', updatedAt: '2026-01-01' }, { id: 'b', title: 'B', kind: 'doc', updatedAt: '2026-11-01' }],
  }, 'p1');
  assert.equal(related.threads.length, 10);
  assert.equal(related.threads[0]!.id, 'new');
  assert.equal(related.threads.some((t) => t.id === 'gone' || t.id === 'old'), false);
  assert.deepEqual(related.prints, [{ id: 'b', title: 'B', kind: 'doc' }, { id: 'a', title: 'A', kind: 'page' }]);
  assert.equal('cwd' in related.threads[0]!, false);

  const asked: string[] = [];
  const project = (runnerId: string) => ({ id: 't', project: { runnerId, id: 'p1' } });
  let row = project('mac');
  const tasks = new Tasks(() => ({ key: 'owner', apiVersion: 102, post: async (_k, fn) => (fn === 'tasks:get' ? row : null) }), () => true, () => {},
    () => ({ runnerId: 'mac', related: (id) => (asked.push(id), { threads: [{ id: 'x', title: 'X', status: 'idle', lastActivityAt: '' }], prints: [] }) }));
  assert.equal((await tasks.call('related', { id: 't' }) as { threads: unknown[] }).threads.length, 1);
  row = project('other-mac');
  assert.deepEqual(await tasks.call('related', { id: 't' }), { threads: [], prints: [] });
  assert.deepEqual(await tasks.call('related', { id: 't', project: null }), { threads: [], prints: [] });
  assert.deepEqual(asked, ['p1']);
});
