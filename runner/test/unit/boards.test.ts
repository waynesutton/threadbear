// Plan boards (src/boards.ts, experiments/planBoard.md): the Mermaid subset,
// versions, approval tied to a version, sends that a second Start can't
// repeat, and what read_board says.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { BoardError, Boards, MAX_VERSIONS, asksForBoard, boardText, buildBrief, parseBoard } from '../../src/boards.ts';
import { tmpDir } from '../helpers/fakes.ts';

const ACME = `flowchart LR
  %% note: Orders reach the dashboard every 5 s
  store[Storefront] -->|new order| orders[(ORDERS)]
  subgraph server[SERVER · NODE]
    summary[GET /api/summary]
  end
  subgraph dash[DASHBOARD · REACT]
    kpi[KPI cards]
    revenue[Revenue chart]
    live[Live orders]
  end
  orders -->|read| summary
  summary -->|every 5 s| kpi
  summary --> revenue
  summary --- live
  style kpi fill:#f9f
`;

const TID = '5533c870-aaaa-bbbb-cccc-000000000001';

test('boards: the subset parses boxes, groups, labelled arrows and notes, keeping what it skips', () => {
  const g = parseBoard(ACME);
  assert.equal(g.direction, 'LR');
  assert.deepEqual(g.nodes.map((n) => n.id), ['store', 'orders', 'summary', 'kpi', 'revenue', 'live']);
  assert.equal(g.nodes.find((n) => n.id === 'orders')!.shape, 'db');
  assert.equal(g.nodes.find((n) => n.id === 'kpi')!.group, 'dash');
  assert.equal(g.nodes.find((n) => n.id === 'store')!.group, undefined);
  assert.deepEqual(g.groups.map((x) => [x.id, x.label]), [['server', 'SERVER · NODE'], ['dash', 'DASHBOARD · REACT']]);
  assert.deepEqual(g.edges.find((e) => e.to === 'kpi'), { from: 'summary', to: 'kpi', label: 'every 5 s' });
  assert.equal(g.edges.find((e) => e.to === 'live')!.line, true);
  assert.deepEqual(g.notes, ['Orders reach the dashboard every 5 s']);
  assert.equal(g.skipped, 1);
});

test('boards: TB, TD, chains, round nodes and the -- label --> form', () => {
  const g = parseBoard('graph TD\n a((Start)) -- go --> b(Next) --> c\n');
  assert.equal(g.direction, 'TB');
  assert.deepEqual(g.edges, [{ from: 'a', to: 'b', label: 'go' }, { from: 'b', to: 'c' }]);
  assert.equal(g.nodes.find((n) => n.id === 'c')!.label, 'c');
  assert.equal(g.nodes.find((n) => n.id === 'a')!.shape, 'round');
});

test('boards: a label holding an arrow stays one label', () => {
  const g = parseBoard('flowchart LR\n a["a --> b"] --> c');
  assert.equal(g.nodes[0].label, 'a --> b');
  assert.equal(g.edges.length, 1);
});

test('boards: limits and bad input are refused with a plain reason', () => {
  assert.throws(() => parseBoard(''), BoardError);
  assert.throws(() => parseBoard('sequenceDiagram\n a->>b: hi'), /flowchart LR/);
  assert.throws(() => parseBoard('flowchart LR\n subgraph x\n a'), /no "end"/);
  const many = `flowchart LR\n${Array.from({ length: 26 }, (_, i) => ` n${i}[Box ${i}]`).join('\n')}`;
  assert.throws(() => parseBoard(many), /more than 25 boxes/);
  const arrows = `flowchart LR\n${Array.from({ length: 41 }, (_, i) => ` a${i % 5} --> b${i % 8}`).join('\n')}`;
  assert.throws(() => parseBoard(arrows), /more than 40 arrows/);
  assert.throws(() => parseBoard(`flowchart LR\n a[${'x'.repeat(61)}]`), /keep labels to 60/);
});

test('boards: versions, the last ten kept, a stale baseVersion refused, a bad write keeping the old one', () => {
  const b = new Boards(path.join(tmpDir(), 'boards'));
  assert.equal(b.has(TID), false);
  const v1 = b.update(TID, { mermaid: ACME, note: 'First take', near: 'kpi', changed: ['kpi', 'nope'] });
  assert.equal(v1.board.version, 1);
  assert.equal(v1.board.state, 'draft');
  assert.deepEqual(v1.board.changed, ['kpi']);
  assert.deepEqual(v1.board.note, { text: 'First take', near: 'kpi' });
  assert.throws(() => b.update(TID, { mermaid: 'flowchart LR\n a[(oops', baseVersion: 1 }), BoardError);
  assert.equal(b.get(TID)!.version, 1);
  assert.throws(() => b.update(TID, { mermaid: ACME, baseVersion: 0 }), /read_board again/);
  for (let i = 0; i < 12; i++) b.update(TID, { mermaid: ACME });
  const board = b.get(TID)!;
  assert.equal(board.version, 13);
  assert.equal(board.versions.length, MAX_VERSIONS);
  assert.equal(board.versions[0].version, 4);
});

test('boards: approval is for one version, and a write after it returns the plan to draft', () => {
  const b = new Boards(path.join(tmpDir(), 'boards'));
  b.update(TID, { mermaid: ACME });
  assert.throws(() => b.approve(TID, 2), /plan is at v1/);
  assert.equal(b.approve(TID, 1).state, 'approved');
  const v2 = b.update(TID, { mermaid: ACME.replace('KPI cards', 'KPI tiles') });
  assert.equal(v2.backToDraft, true);
  assert.equal(v2.board.state, 'draft');
  assert.deepEqual(v2.board.approved?.version, 1);
  assert.throws(() => b.mustSend(TID, 'kpi'), /changed since Wayne approved it \(v1 → v2\)/);
  assert.equal(b.update(TID, { mermaid: ACME }).backToDraft, false);
});

test('boards: nothing is sent from a draft, and a box must be in the plan', () => {
  const b = new Boards(path.join(tmpDir(), 'boards'));
  b.update(TID, { mermaid: ACME });
  assert.throws(() => b.mustSend(TID, 'kpi'), /isn't approved yet/);
  b.approve(TID, 1);
  assert.throws(() => b.mustSend(TID, 'ghost'), /no box "ghost"/);
  assert.equal(b.mustSend(TID, 'kpi').version, 1);
});

test('boards: a request id records its send once, and Send again keeps the old link in history', () => {
  const b = new Boards(path.join(tmpDir(), 'boards'));
  b.update(TID, { mermaid: ACME });
  b.approve(TID, 1);
  b.link(TID, 'kpi', { threadId: 'a1b2c3d4-0000', harness: 'claude', planVersion: 1, at: 'x' }, 'req-1');
  assert.deepEqual(b.sent(TID, 'req-1'), { node: 'kpi', ref: 'a1b2c3d4-0000' });
  const again = b.link(TID, 'kpi', { threadId: 'ffff0000-1111', harness: 'codex', planVersion: 1, at: 'y' }, 'req-2');
  assert.equal(again.links.kpi.kind ?? 'thread', 'thread');
  assert.equal((again.links.kpi as { threadId: string }).threadId, 'ffff0000-1111');
  assert.equal(again.links.kpi.history.length, 1);
  assert.equal((again.links.kpi.history[0] as { threadId: string }).threadId, 'a1b2c3d4-0000');
});

test('boards: moves are kept per box across a new version, and Tidy clears them', () => {
  const b = new Boards(path.join(tmpDir(), 'boards'));
  b.update(TID, { mermaid: ACME });
  b.move(TID, { kpi: [400.4, 120], 'bad id!': [1, 1] });
  b.update(TID, { mermaid: ACME });
  assert.deepEqual(b.get(TID)!.positions, { kpi: [400, 120] });
  b.move(TID, { kpi: null });
  assert.deepEqual(b.get(TID)!.positions, {});
  b.move(TID, { kpi: [1, 2] });
  assert.deepEqual(b.move(TID, {}, true).positions, {});
});

test('boards: skills are names only, cleaned', () => {
  const b = new Boards(path.join(tmpDir(), 'boards'));
  b.update(TID, { mermaid: ACME });
  assert.deepEqual(b.setSkills(TID, ['/shadcn', 'component-canvas', 'shadcn', 'rm -rf']).skills, ['shadcn', 'component-canvas']);
});

test('boards: the brief starts with the fixed header, then skills in each provider\'s form', () => {
  const b = new Boards(path.join(tmpDir(), 'boards'));
  b.update(TID, { mermaid: ACME, note: 'Push each order' });
  b.approve(TID, 1);
  const board = b.setSkills(TID, ['component-canvas', 'shadcn']);
  const claude = buildBrief(board, 'kpi', 'claude').split('\n');
  assert.equal(claude[0], 'Plan board: thread 5533c870, box kpi ("KPI cards"), plan v1 (approved).');
  assert.equal(claude[1], 'Use these skills: /component-canvas, /shadcn');
  assert.match(claude.join('\n'), /GET \/api\/summary \(every 5 s\) → this/);
  assert.match(claude.join('\n'), /Planner's note: Push each order/);
  assert.match(claude[claude.length - 1], /If you need Wayne, ask him/);
  assert.equal(buildBrief(board, 'kpi', 'codex').split('\n')[1], 'Use these skills: $component-canvas, $shadcn');
  const plain = buildBrief(b.setSkills(TID, []), 'kpi', 'claude').split('\n');
  assert.equal(plain[1], '');
});

test('boards: secrets in plan text are masked before they are kept', () => {
  const b = new Boards(path.join(tmpDir(), 'boards'));
  const r = b.update(TID, { mermaid: 'flowchart LR\n a[key sk-ant-api03-abcdefghijklmnopqrstuvwxyz0123456789]' });
  assert.equal(r.redacted, true);
  const raw = fs.readFileSync(path.join(b.dir, TID, 'v1.json'), 'utf8');
  assert.doesNotMatch(raw, /abcdefghijklmnop/);
});

test('boards: read_board text says the state, the links and the skills', () => {
  const b = new Boards(path.join(tmpDir(), 'boards'));
  b.update(TID, { mermaid: ACME });
  b.approve(TID, 1);
  b.setSkills(TID, ['shadcn']);
  const board = b.link(TID, 'kpi', { threadId: 'a1b2c3d4-0000', harness: 'claude', planVersion: 1, at: 'x' });
  const text = boardText(board, () => 'working');
  assert.match(text, /^Plan v1, approved\./);
  assert.match(text, /Board skills: \/shadcn/);
  assert.match(text, /kpi: KPI cards \[dash\] → thread a1b2c3d4 \(claude, working\)/);
});

test('boards: a thread id that is not one is refused, so no path leaves the folder', () => {
  const b = new Boards(path.join(tmpDir(), 'boards'));
  assert.throws(() => b.update('../../etc', { mermaid: ACME }), BoardError);
  assert.equal(b.has('../x'), false);
});

test('a first board only when Wayne says "plan board"', () => {
  for (const yes of ['Plan board: a todo app on Convex', 'can you make a plan board for this?', 'plan-board the auth work', 'PlanBoard it']) assert.ok(asksForBoard(yes), yes);
  // A plan asked for any other way, plan mode's included, and the Tasks board.
  for (const no of ['Plan the todo app', 'make a plan first', 'put it on my board', 'add to the board', 'a planning boarder']) assert.ok(!asksForBoard(no), no);
});
