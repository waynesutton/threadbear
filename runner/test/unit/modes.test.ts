// Plan first's plans saved as files (Runner.savePlan, modes.ts).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { makeHarness, tick, tmpDir } from '../helpers/fakes.ts';
import { planPath } from '../../src/modes.ts';

test('a Plan first plan is saved, pinned and opened; a revision overwrites it; after Go ahead the next is new', async () => {
  const cwd = tmpDir();
  const h = makeHarness();
  const { thread } = h.runner.create({ text: 'Add billing', cwd, accountId: 'claude-3' });
  await tick();
  const s = h.adapter.last;
  s.init();
  s.out({ kind: 'planProposed', itemId: 'p1', text: '## Plan\n\n1. Stripe' } as never);
  await tick();
  const plans = () => fs.readdirSync(path.join(cwd, 'plans')).sort();
  assert.equal(plans().length, 1);
  const first = path.join(cwd, 'plans', plans()[0]);
  assert.match(fs.readFileSync(first, 'utf8'), /^# Plan: [\s\S]*1\. Stripe/);
  assert.ok(h.events(thread.id).some((e) => e.type === 'system.note' && /^Saved the plan in \[plans\/[^\]]+\]\(<\/.+>\)\.$/.test(e.text)));
  const pinned = h.runner.highlights(thread.id).highlights.find((i) => i.path === first);
  assert.ok(pinned, 'pinned');
  assert.equal(h.runner.get(thread.id).panelRequest?.highlight, pinned.id, 'and asked to be opened in the pane');
  // Sent back and proposed again: the same file, said as updated.
  s.out({ kind: 'planProposed', itemId: 'p2', text: '## Plan\n\n1. Stripe\n2. Webhooks' } as never);
  await tick();
  assert.equal(plans().length, 1);
  assert.match(fs.readFileSync(first, 'utf8'), /2\. Webhooks/);
  assert.ok(h.events(thread.id).some((e) => e.type === 'system.note' && /^Updated the plan in /.test(e.text)));
  // Go ahead, then a new plan: a file of its own.
  h.runner.approvePlan({ threadId: thread.id, itemId: 'p2' });
  await tick();
  s.out({ kind: 'planProposed', itemId: 'p3', text: '## Next plan' } as never);
  await tick();
  assert.equal(plans().length, 2);
});

test('plan files are named for the day and project, and never overwrite another', () => {
  const cwd = tmpDir();
  const day = new Date('2026-11-06T10:00:00Z');
  const first = planPath(cwd, 'Hello App!', day);
  assert.equal(path.basename(first), '2026-11-06-hello-app.md');
  fs.mkdirSync(path.dirname(first), { recursive: true });
  fs.writeFileSync(first, 'x');
  assert.equal(path.basename(planPath(cwd, 'Hello App!', day)), '2026-11-06-hello-app-2.md');
});
