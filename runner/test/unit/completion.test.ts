import { z } from 'zod';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { mergeCompletion, completionOpen } from '../../src/completion.ts';
import { makeHarness, tmpDir } from '../helpers/fakes.ts';
import { patiently } from '../helpers/patience.ts';
import { buildThreadbearTools } from '../../src/threadbear-mcp.ts';
import { COMPLETION_LINE, COMPLETION_NUDGE } from '../../src/needs.ts';
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

test('a merge past 30 drops the oldest completed tasks first, and keeps every open one', () => {
  const full = mergeCompletion(undefined, { tasks: Array.from({ length: 30 }, (_, i) => task(`t${i}`, i < 20 ? 'completed' : 'pending')) })!;
  const next = mergeCompletion(full, { tasks: [task('n1'), task('n2', 'completed'), task('n3', 'blocked')] })!;
  assert.equal(next.tasks.length, 30);
  assert.deepEqual(next.tasks.slice(0, 2).map(t => t.id), ['t3', 't4']);
  assert.ok(next.tasks.filter(t => t.status !== 'completed').length === 12);
  assert.deepEqual(next.tasks.slice(-3).map(t => t.id), ['n1', 'n2', 'n3']);
});

test('a merge is refused only when the open tasks alone are over 30', () => {
  const open = mergeCompletion(undefined, { tasks: Array.from({ length: 30 }, (_, i) => task(`o${i}`)) })!;
  assert.equal(mergeCompletion(open, { tasks: [task('extra')] }), open);
  const finishing = mergeCompletion(open, { tasks: [task('o0', 'completed'), task('extra')] })!;
  assert.equal(finishing.tasks.length, 30);
  assert.ok(!finishing.tasks.some(t => t.id === 'o0'));
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
  // A finished helper's archive is decided as its turn ends (rec.finished,
  // checked below); what runs after is its loose-ends check, so wait for that
  // to settle rather than a fixed time.
  await patiently(() => !h.runner.threads.get(id)!.rt.looseEndsBusy);
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

test('Highlights carry the task ledger with the experiment on, after a reply and after archiving', () => {
  for (const on of [false, true]) {
    const h = setup(on);
    const id = h.runner.create({ text: 'Do two things' }).thread.id;
    finish(h, id, { tasks: [task('a', 'completed'), task('b')] });
    assert.equal(Boolean(h.runner.highlights(id).completion), on);
    if (!on) continue;
    h.runner.send(id, 'Something else');
    assert.equal(h.runner.get(id).completion?.tasks.length, 2);
    finish(h, id);
    h.runner.update({ threadId: id, archived: true });
    assert.equal(h.runner.get(id).completion, undefined, 'the summary drops the card once archived');
    assert.deepEqual(h.runner.highlights(id).completion?.tasks, [task('a', 'completed'), task('b')], 'Highlights keep it as the record');
  }
});

test('a working turn that leaves the task list out is asked for it once, for Claude and Codex in both forms', async () => {
  for (const provider of ['claude', 'codex'] as const) {
    for (const oneReply of [true, false]) {
      const run = async (o: { on?: boolean; calls?: number; own?: boolean; open?: boolean; completion?: unknown }) => {
        const h = setup(o.on ?? true);
        const id = h.runner.create({ text: 'Do two things' }).thread.id;
        if (o.open) {
          finish(h, id, { tasks: [task('a')] });
          h.runner.send(id, 'Carry on');
        }
        const s = h.adapter.sessions.filter(s => s.opts.threadId === id).at(-1)!;
        s.init();
        const call = (itemId: string, name: string, input: Record<string, unknown> = {}) =>
          s.out({ kind: 'event', event: { type: 'tool.call', itemId, name, kind: 'other', title: name, input } });
        for (let i = 0; i < (o.calls ?? 0); i++) call(`c${i}`, o.own ? 'mcp__threadbear__list_threads' : 'Bash');
        s.text('m1', 'Did the work.');
        const tool = buildThreadbearTools(h.runner, id, Date.now, undefined, undefined, undefined, { provider, oneReply: () => oneReply, clearerCompletion: () => o.on ?? true, sleep: async () => undefined }).find(t => t.name === 'end_turn')!;
        const args = { needs: 'nothing', ...(oneReply ? { headline: 'Did it' } : { summary: ['Did it.'] }), ...(o.completion ? { completion: o.completion } : {}) };
        const end = async (itemId: string) => {
          call(itemId, 'mcp__threadbear__end_turn', args);
          const r = (await tool.handler(args as never, {})) as { content: { text: string }[]; isError?: boolean };
          const text = r.content.map(c => c.text).join('');
          s.out({ kind: 'event', event: { type: 'tool.result', itemId, ok: !r.isError, text, truncated: false, images: [] } });
          return { text, error: Boolean(r.isError) };
        };
        return { end, description: tool.description };
      };
      const label = `${provider} oneReply=${oneReply}`;
      const worked = await run({ calls: 5 });
      assert.ok(worked.description.endsWith(COMPLETION_LINE), label);
      const first = await worked.end('e1');
      assert.ok(first.error && first.text.startsWith(COMPLETION_NUDGE), `${label}: ${first.text}`);
      const second = await worked.end('e2');
      assert.equal(second.error, false, `${label}: ${second.text}`);
      const open = await (await run({ open: true })).end('e1');
      assert.ok(open.text.startsWith(COMPLETION_NUDGE), `${label}: an open list is always owed: ${open.text}`);
      for (const fine of [{ calls: 4 }, { calls: 9, own: true }, { calls: 9, completion: { tasks: [task('a', 'completed')] } }, { calls: 9, on: false }]) {
        const r = await run(fine);
        if (fine.on === false) assert.doesNotMatch(r.description, /pass completion/);
        const ended = await r.end('e1');
        assert.equal(ended.error, false, `${label} ${JSON.stringify(fine)}: ${ended.text}`);
      }
    }
  }
});
