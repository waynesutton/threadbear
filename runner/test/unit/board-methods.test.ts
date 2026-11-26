// Plan boards, end to end in the runner (experiments/planBoard.md): the agent's
// tools only with the experiment on, the board.updated event, board_node refused
// on a draft, and Wayne's RPCs (approve one version, send a box once into a
// nested thread, Send again keeping history), plus Phase 3's coordinator
// destination through its port.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { makeHarness } from '../helpers/fakes.ts';
import { buildThreadbearTools, serverInstructions, PLAN_BOARD_LINE, PLAN_BOARD_SCREENS_LINE } from '../../src/threadbear-mcp.ts';
import { Boards, type BoardTools } from '../../src/boards.ts';
import { boardMethods, type BoardCoordinatorPort } from '../../src/board-methods.ts';

const PLAN = `flowchart LR
  summary[GET /api/summary] -->|every 5 s| kpi[KPI cards]
  summary --> revenue[Revenue chart]
`;

function setup(opts: { coordinators?: BoardCoordinatorPort; coordinatorsOn?: boolean; on?: boolean } = {}) {
  const h = makeHarness();
  const { thread } = h.runner.create({ text: 'Plan the Acme dashboard', accountId: 'claude-3' });
  const boards = new Boards(path.join(h.home, 'boards'));
  let on = opts.on ?? true;
  const control: BoardTools = {
    read: (id) => boards.get(id),
    write: (id, u) => {
      const r = boards.update(id, u);
      h.runner.emitBoard(id, r.board);
      return r;
    },
    mustSend: (id, node) => boards.mustSend(id, node),
    linkThread: (id, node, child, v) => {
      boards.link(id, node, { threadId: child.id, harness: child.provider, planVersion: v, at: 'now' });
    },
    statusOf: () => 'working',
  };
  const tools = (provider?: 'codex') => buildThreadbearTools(h.runner, thread.id, Date.now, undefined, undefined, undefined, { boards: () => (on ? control : undefined), ...(provider ? { provider } : {}) });
  const methods = boardMethods({ boards, on: () => on, coordinatorsOn: () => opts.coordinatorsOn ?? false, coordinators: () => opts.coordinators, runner: h.runner });
  return { h, thread, boards, tools, methods, setOn: (v: boolean) => { on = v; } };
}

const call = async (ts: ReturnType<ReturnType<typeof setup>['tools']>, name: string, args: Record<string, unknown>) => {
  const t = ts.find((x) => x.name === name);
  assert.ok(t, `${name} is a tool`);
  const r = await t.handler(args as never, {} as never) as { content: { text: string }[]; isError?: boolean };
  return { text: r.content.map((c) => c.text).join('\n'), error: r.isError === true };
};

test('planBoard off: no board tools, no board_node, no guidance line', () => {
  const s = setup({ on: false });
  const names = s.tools().map((t) => t.name);
  assert.ok(!names.includes('read_board') && !names.includes('update_board'));
  const create = s.tools().find((t) => t.name === 'create_thread')!;
  assert.ok(!('board_node' in (create.inputSchema as Record<string, unknown>)));
  assert.ok(!serverInstructions('x', { one: true, bulletReplies: true, pocket: false, projects: false }).includes(PLAN_BOARD_LINE));
  assert.ok(serverInstructions('x', { one: true, bulletReplies: true, pocket: false, projects: false, planBoard: true }).includes(PLAN_BOARD_LINE));
  // Screens go on the board, not show_options, only with mockups on too.
  assert.ok(!serverInstructions('x', { one: true, bulletReplies: true, pocket: false, projects: false, planBoard: true }).includes(PLAN_BOARD_SCREENS_LINE));
  assert.ok(serverInstructions('x', { one: true, bulletReplies: true, pocket: false, projects: false, planBoard: true, planBoardScreens: true }).includes(PLAN_BOARD_SCREENS_LINE));
  assert.throws(() => s.methods['board.get']({ threadId: s.thread.id }), /Plan board is off/);
});

test('planBoard on: update_board saves a version and logs one board.updated event; a bad plan keeps the old one', async () => {
  const s = setup();
  for (const provider of [undefined, 'codex'] as const) {
    const names = s.tools(provider).map((t) => t.name);
    assert.ok(names.includes('read_board') && names.includes('update_board'), 'both providers get them');
  }
  const ok = await call(s.tools(), 'update_board', { mermaid: PLAN, note: 'First take' });
  assert.match(ok.text, /Saved plan v1: 3 boxes/);
  assert.match(ok.text, /draft until Wayne presses Approve plan/);
  const events = s.h.events(s.thread.id).filter((e) => e.type === 'board.updated');
  assert.equal(events.length, 1);
  assert.equal(events[0].board.graph.nodes.length, 3);
  const bad = await call(s.tools(), 'update_board', { mermaid: 'flowchart LR\n a[(oops' });
  assert.equal(bad.error, true);
  assert.match(bad.text, /Not saved; the board still shows v1/);
  assert.equal(s.h.events(s.thread.id).filter((e) => e.type === 'board.updated').length, 1);
  const read = await call(s.tools(), 'read_board', {});
  assert.match(read.text, /Plan v1, draft \(not approved yet\)/);
});

test('board_node is refused on a draft, then starts a nested thread with the header once approved', async () => {
  const s = setup();
  await call(s.tools(), 'update_board', { mermaid: PLAN });
  const refused = await call(s.tools(), 'create_thread', { text: 'Build the KPI cards', board_node: 'kpi' });
  assert.equal(refused.error, true);
  assert.match(refused.text, /isn't approved yet\. Ask Wayne to press Approve plan/);
  s.methods['board.approve']({ threadId: s.thread.id, version: 1 });
  const started = await call(s.tools(), 'create_thread', { text: 'Build the KPI cards', board_node: 'kpi' });
  assert.equal(started.error, false, started.text);
  const board = s.boards.get(s.thread.id)!;
  const link = board.links.kpi as { threadId: string };
  assert.ok(link.threadId);
  const child = s.h.runner.get(link.threadId);
  assert.equal(child.startedBy?.threadId, s.thread.id);
  const first = s.h.events(link.threadId).find((e) => e.type === 'user.message');
  assert.match(first.text, /Plan board: thread [0-9a-f]{8}, box kpi \("KPI cards"\), plan v1 \(approved\)\./);
  // A write after approval greys it again for the agent too.
  const v2 = await call(s.tools(), 'update_board', { mermaid: PLAN.replace('Revenue chart', 'Revenue') });
  assert.match(v2.text, /so it's a draft again/);
  const again = await call(s.tools(), 'create_thread', { text: 'Build revenue', board_node: 'revenue' });
  assert.match(again.text, /changed since Wayne approved it \(v1 → v2\)/);
});

test('board.approve takes only the latest version; board.send starts one nested thread per request id', () => {
  const s = setup();
  s.boards.update(s.thread.id, { mermaid: PLAN });
  assert.throws(() => s.methods['board.send']({ threadId: s.thread.id, nodeId: 'kpi', requestId: 'r1' }), /isn't approved yet/);
  assert.throws(() => s.methods['board.approve']({ threadId: s.thread.id, version: 3 }), /plan is at v1/);
  s.methods['board.approve']({ threadId: s.thread.id, version: 1 });
  const before = s.h.runner.list(['inbox']).length;
  const a = s.methods['board.send']({ threadId: s.thread.id, nodeId: 'revenue', requestId: 'r1', provider: 'codex', brief: 'Build the revenue chart.' }) as { thread: { id: string; provider: string; title: string } };
  const b = s.methods['board.send']({ threadId: s.thread.id, nodeId: 'revenue', requestId: 'r1', provider: 'codex' }) as { threadId: string; repeated: boolean };
  assert.equal(b.repeated, true);
  assert.equal(b.threadId, a.thread.id);
  assert.equal(s.h.runner.list(['inbox']).length, before + 1, 'one thread, not two');
  // The harness has Claude logins only; the link records whatever it ran on.
  assert.equal((s.boards.get(s.thread.id)!.links.revenue as { harness: string }).harness, a.thread.provider);
  assert.equal(a.thread.title, 'Revenue chart');
  assert.equal(s.h.runner.get(a.thread.id).startedBy?.threadId, s.thread.id);
  assert.equal(s.h.runner.get(a.thread.id).permissionMode, s.h.runner.get(s.thread.id).permissionMode);
  const text = s.h.events(a.thread.id).find((e) => e.type === 'user.message').text as string;
  assert.ok(text.startsWith('Plan board: thread '), 'the runner puts its header first even when the brief has none');
  assert.match(text, /Build the revenue chart\./);
  // Send again: a new thread, the old one in history.
  const c = s.methods['board.send']({ threadId: s.thread.id, nodeId: 'revenue', requestId: 'r2' }) as { board: { links: Record<string, { threadId: string; history: { threadId: string }[] }> } };
  assert.notEqual(c.board.links.revenue.threadId, a.thread.id);
  assert.equal(c.board.links.revenue.history[0].threadId, a.thread.id);
  // Moves and skills, Wayne's own, log no event.
  const logged = s.h.events(s.thread.id).length;
  s.methods['board.move']({ threadId: s.thread.id, positions: { kpi: [10, 20] } });
  s.methods['board.skills']({ threadId: s.thread.id, skills: ['shadcn'] });
  assert.equal(s.h.events(s.thread.id).length, logged);
  assert.deepEqual(s.boards.get(s.thread.id)!.positions, { kpi: [10, 20] });
});

test('board skills go into the brief a send writes', () => {
  const s = setup();
  s.boards.update(s.thread.id, { mermaid: PLAN });
  s.methods['board.approve']({ threadId: s.thread.id, version: 1 });
  s.methods['board.skills']({ threadId: s.thread.id, skills: ['component-canvas'] });
  const r = s.methods['board.send']({ threadId: s.thread.id, nodeId: 'kpi', requestId: 'r1' }) as { thread: { id: string } };
  const text = s.h.events(r.thread.id).find((e) => e.type === 'user.message').text as string;
  assert.match(text, /^Plan board: .*\nUse these skills: \/component-canvas\n/);
});

test('Phase 3: no coordinator destination without the flag, a port, or an active coordinator', () => {
  const s = setup();
  s.boards.update(s.thread.id, { mermaid: PLAN });
  s.methods['board.approve']({ threadId: s.thread.id, version: 1 });
  assert.equal('coordinator' in (s.methods['board.get']({ threadId: s.thread.id }) as object), false);
  assert.throws(() => s.methods['board.send']({ threadId: s.thread.id, nodeId: 'kpi', requestId: 'r1', destination: 'coordinator' }), /no active project coordinator/);
});

test('Phase 3: Give to the project coordinator adds one to-do with the brief header, once per request id, and never a thread', () => {
  const added: { projectId: string; title: string; brief: string }[] = [];
  const port: BoardCoordinatorPort = {
    forThread: () => ({ projectId: 'p1', threadId: 'coord-thread-1' }),
    addTask: (projectId, t) => {
      added.push({ projectId, ...t });
      return { taskId: `t${added.length}` };
    },
  };
  const s = setup({ coordinators: port, coordinatorsOn: true });
  s.boards.update(s.thread.id, { mermaid: PLAN });
  assert.throws(() => s.methods['board.send']({ threadId: s.thread.id, nodeId: 'kpi', requestId: 'r0', destination: 'coordinator' }), /isn't approved yet/);
  s.methods['board.approve']({ threadId: s.thread.id, version: 1 });
  assert.deepEqual((s.methods['board.get']({ threadId: s.thread.id }) as { coordinator: unknown }).coordinator, { projectId: 'p1', threadId: 'coord-thread-1' });
  const threads = s.h.runner.list(['inbox']).length;
  const r = s.methods['board.send']({ threadId: s.thread.id, nodeId: 'kpi', requestId: 'r1', destination: 'coordinator' }) as { taskId: string; board: { links: Record<string, { kind: string; taskId: string }> } };
  const again = s.methods['board.send']({ threadId: s.thread.id, nodeId: 'kpi', requestId: 'r1', destination: 'coordinator' }) as { taskId: string; repeated: boolean };
  assert.equal(added.length, 1);
  assert.equal(again.repeated, true);
  assert.equal(again.taskId, r.taskId);
  assert.equal(s.h.runner.list(['inbox']).length, threads, 'no create_thread on this path');
  assert.equal(r.board.links.kpi.kind, 'coordinator');
  assert.match(added[0].brief, /^Plan board: thread [0-9a-f]{8}, box kpi \("KPI cards"\), plan v1 \(approved\)\./);
  const next = s.methods['board.send']({ threadId: s.thread.id, nodeId: 'kpi', requestId: 'r2', destination: 'coordinator' }) as { board: { links: Record<string, { taskId: string; history: { taskId: string }[] }> } };
  assert.equal(added.length, 2);
  assert.equal(next.board.links.kpi.history[0].taskId, 't1');
});

test('Phase 3 flag off: the coordinator destination is refused even with a port', () => {
  const port: BoardCoordinatorPort = { forThread: () => ({ projectId: 'p1', threadId: 'c' }), addTask: () => ({ taskId: 't' }) };
  const s = setup({ coordinators: port, coordinatorsOn: false });
  s.boards.update(s.thread.id, { mermaid: PLAN });
  s.methods['board.approve']({ threadId: s.thread.id, version: 1 });
  assert.throws(() => s.methods['board.send']({ threadId: s.thread.id, nodeId: 'kpi', requestId: 'r1', destination: 'coordinator' }), /no active project coordinator/);
});

test('boards on disk are left alone with the experiment off', () => {
  const s = setup();
  s.boards.update(s.thread.id, { mermaid: PLAN });
  s.setOn(false);
  assert.ok(fs.existsSync(path.join(s.boards.dir, s.thread.id, 'v1.json')));
  assert.equal(s.tools().some((t) => t.name === 'read_board'), false);
});
