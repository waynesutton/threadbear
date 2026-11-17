import { z } from 'zod';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { mergeCompletion, completionOpen } from '../../src/completion.ts';
import { makeHarness, tmpDir } from '../helpers/fakes.ts';
import { buildThreadbearTools } from '../../src/threadbear-mcp.ts';
import { ThreadStore } from '../../src/store.ts';

const task = (id: string, status: 'pending' | 'completed' | 'blocked' = 'pending') => ({ id, title: `Task ${id}`, status });

test('completion updates preserve earlier tasks, order and caller state; removal is explicit', () => {
  const first = mergeCompletion(undefined, { tasks: [task('a', 'completed'), task('b')] })!;
  const second = mergeCompletion(first, { tasks: [task('b', 'completed'), task('c', 'blocked')] })!;
  assert.deepEqual(second.tasks.map(t => [t.id, t.status]), [['a', 'completed'], ['b', 'completed'], ['c', 'blocked']]);
  assert.equal(first.tasks[1].status, 'pending');
  assert.equal(completionOpen(second), true);
  assert.equal(completionOpen(mergeCompletion(second, { tasks: [], removeIds: ['c'] })), false);
  assert.equal(mergeCompletion(first, undefined), first);
});

test('invalid, duplicate and over-limit updates cannot erase or complete earlier tasks', () => {
  const first = mergeCompletion(undefined, { tasks: [task('a')] })!;
  for (const input of [{ tasks: [task('b'), task('b')] }, { tasks: [{ ...task('a'), status: 'done' }] }, { tasks: [task('a')], removeIds: ['a'] }, { tasks: [{ ...task('a'), title: '' }] }, { tasks: Array.from({ length: 31 }, (_, i) => task(String(i))) }]) {
    assert.equal(mergeCompletion(first, input), first);
  }
  assert.equal(mergeCompletion(undefined, { tasks: [] }), undefined);
  assert.equal(mergeCompletion(first, { tasks: [], removeIds: ['a'] }), undefined);
});

function setup(on: boolean) {
  const home = tmpDir();
  fs.writeFileSync(path.join(home, 'runner.json'), JSON.stringify({ experiments: { clearerCompletion: on, quietInbox: true } }));
  return makeHarness(home);
}

function finish(h: ReturnType<typeof setup>, id: string, completion?: unknown, outcome: 'completed' | 'failed' = 'completed') {
  const s = h.adapter.sessions.filter(s => s.opts.threadId === id).at(-1)!;
  s.init();
  s.out({ kind: 'event', event: { type: 'tool.call', itemId: `e${Math.random()}`, name: 'mcp__threadbear__end_turn', kind: 'mcp', title: 'end_turn', input: { needs: 'nothing', ...(completion ? { completion } : {}) } } });
  s.text(`m${Math.random()}`, 'Finished this turn.');
  s.result(outcome);
}

test('runner only reports completion with experiment on and retains it on subsequent turns', () => {
  for (const on of [false, true]) {
    const h = setup(on);
    const id = h.runner.create({ text: 'Do two things' }).thread.id;
    finish(h, id, { tasks: [task('a', 'completed'), task('b')] });
    assert.equal(Boolean(h.runner.get(id).completion), on);
    const event = h.events(id).filter(e => e.type === 'turn.completed').at(-1)!;
    assert.equal(Boolean(event.completion), on);
    if (!on) continue;
    h.runner.send(id, 'Finish the second task');
    finish(h, id, { tasks: [task('b', 'completed')] });
    assert.deepEqual(h.runner.get(id).completion?.tasks, [task('a', 'completed'), task('b', 'completed')]);
    h.runner.send(id, 'Check again');
    finish(h, id, { tasks: [task('a', 'blocked')] }, 'failed');
    assert.equal(h.runner.get(id).completion, undefined, 'failed turns do not expose an earlier completion card');
    assert.equal(h.runner.threads.get(id)?.rec.completion?.tasks[0].status, 'completed');
  }
});

test('experiment field is absent for old callers and available in both end_turn forms when on', () => {
  const h = setup(true);
  const id = h.runner.create({ text: 'Do two things' }).thread.id;
  for (const oneReply of [false, true]) {
    for (const on of [false, true]) {
      const tool = buildThreadbearTools(h.runner, id, Date.now, undefined, undefined, undefined, { oneReply: () => oneReply, clearerCompletion: () => on }).find(t => t.name === 'end_turn')!;
      assert.equal('completion' in z.toJSONSchema(tool.inputSchema as unknown as z.ZodObject).properties!, on);
    }
  }
});

test('task progress persists and an unfinished helper stays out of automatic archives', async () => {
  const h = setup(true);
  const parent = h.runner.create({ text: 'Build the feature' }).thread.id;
  const id = h.runner.create({ text: 'Verify the feature', startedBy: { kind: 'thread', label: 'Agent', threadId: parent } }).thread.id;
  finish(h, id, { tasks: [task('verify', 'blocked')] });
  await new Promise(resolve => setTimeout(resolve, 200));
  assert.equal(h.runner.get(id).archivedAt, undefined);
  assert.equal(h.runner.childEnds(h.runner.threads.get(parent)!).some(end => end.threadId === id), true, 'unfinished helpers remain a loose end for their parent');
  assert.equal(h.runner.threads.get(id)?.rec.finished, undefined);
  h.store.save(h.runner.threads.get(id)!.rec);
  const stored = new ThreadStore(h.cfg.home).load(id);
  assert.deepEqual(stored?.completion, { tasks: [task('verify', 'blocked')] });
  h.runner.send(id, 'Continue');
  assert.equal(h.runner.get(id).completion?.tasks[0].status, 'blocked', 'read_thread can recover the ledger during its next active turn');
  finish(h, id);
  assert.equal(h.runner.get(id).completion?.tasks[0].status, 'blocked', 'omitting progress does not finish old tasks');
});
