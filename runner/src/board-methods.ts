// Plan boards' RPCs (experiment planBoard, experiments/planBoard.md;
// docs/protocol/events.md, "Plan boards"). Wayne's side of a board: read it, approve a
// version, drag boxes, pick the board's skills, and send one box out.
//
// None of these is open to a thread's own key (rpc-gate.ts refuses any method
// it doesn't list), so an agent can't approve its own plan or send a box
// through here: its way to start a box's thread is create_thread with
// board_node, which is refused on a draft.
//
// Every handler is synchronous, like the RPC server's own, so two Starts with
// the same request id can't both get past the check that the first one made
// a thread.
import { ErrorCode, RpcError, type EffortLevel, type PermissionMode, type Provider, type ThreadStartedBy, type ThreadSummary } from './protocol.ts';
import { BoardError, Boards, buildBrief, withBriefHeader, type PlanBoard } from './boards.ts';
import type { BoardFrames } from './board-frames.ts';

type Params = Record<string, unknown>;

// What Phase 3 (planBoardCoordinators) needs from the project coordinators
// (Mike's `coordinators` experiment). Wayne's fork has no coordinators, so
// main.ts passes none there and the destination doesn't exist; nothing here
// imports coordinators.ts.
export type BoardCoordinatorPort = {
  // The project's active coordinator for the planner thread, if any.
  forThread(threadId: string): { projectId: string; threadId: string } | undefined;
  addTask(projectId: string, task: { title: string; doneWhen: string; brief: string }): { taskId: string };
};

export type BoardMethodDeps = {
  boards: Boards;
  on: () => boolean;
  coordinatorsOn?: () => boolean;
  coordinators?: () => BoardCoordinatorPort | undefined;
  // planBoardMockups's screens, while it's on (main.ts framesFor).
  frames?: () => BoardFrames | undefined;
  runner: {
    get(threadId: string): ThreadSummary;
    has(threadId: string): boolean;
    hello(): { accounts: { id: string; provider: Provider; models?: { id: string; model?: string }[] }[]; pools?: { id: string; accountIds: string[] }[] };
    create(params: {
      text: string;
      cwd?: string;
      accountId?: string;
      poolId?: string;
      provider?: Provider;
      model?: string;
      effort?: EffortLevel;
      permissionMode?: PermissionMode;
      title?: string;
      startedBy?: ThreadStartedBy;
    }): { thread: ThreadSummary; turnId: string };
  };
  now?: () => number;
};

const bad = (message: string) => new RpcError(ErrorCode.badParams, message);

function str(p: Params, k: string, required = true): string | undefined {
  const v = p[k];
  if (v === undefined || v === null || v === '') {
    if (required) throw bad(`${k} must be a string`);
    return undefined;
  }
  if (typeof v !== 'string') throw bad(`${k} must be a string`);
  return v;
}

const MODES = new Set(['full', 'ask', 'auto', 'plan']);
const EFFORTS = new Set(['low', 'medium', 'high', 'xhigh', 'max', 'ultra']);

export function boardMethods(d: BoardMethodDeps): Record<string, (p: Params) => unknown> {
  const now = d.now ?? Date.now;
  // Each one refused while the experiment's off, rather than missing: a
  // stale app sees why.
  const guard = <T>(f: (p: Params) => T) => (p: Params): T => {
    if (!d.on()) throw new RpcError(ErrorCode.notAllowed, 'Plan board is off. Turn it on in Settings > General > Experiments.');
    try {
      return f(p ?? {});
    } catch (e) {
      if (e instanceof BoardError) throw bad(e.message);
      throw e;
    }
  };
  const thread = (p: Params): string => {
    const id = str(p, 'threadId')!;
    if (!d.runner.has(id)) throw new RpcError(ErrorCode.threadNotFound, `Thread not found: ${id}`);
    return id;
  };
  const mustBoard = (id: string): PlanBoard => {
    const b = d.boards.get(id);
    if (!b) throw bad('This thread has no plan.');
    return b;
  };
  const providerOf = (accountId?: string, poolId?: string, provider?: string): string => {
    if (provider === 'codex' || provider === 'claude') return provider;
    const h = d.runner.hello();
    const acc = accountId ? h.accounts.find((a) => a.id === accountId) : undefined;
    if (acc) return acc.provider;
    const first = poolId ? (h.pools ?? []).find((x) => x.id === poolId)?.accountIds[0] : undefined;
    return h.accounts.find((a) => a.id === first)?.provider ?? 'claude';
  };
  const coordinatorDestination = (id: string) => (d.coordinatorsOn?.() ? d.coordinators?.()?.forThread(id) : undefined);

  return {
    // The board as it is now, or null for a thread with none.
    'board.get': guard((p) => {
      const id = str(p, 'threadId')!;
      const board = d.boards.get(id) ?? null;
      // Whether Give to the project coordinator can be offered (Phase 3).
      const coordinator = board ? coordinatorDestination(id) : undefined;
      return { board, ...(coordinator ? { coordinator: { projectId: coordinator.projectId, threadId: coordinator.threadId } } : {}) };
    }),
    // One old version, read-only (the version menu).
    'board.version': guard((p) => {
      const id = str(p, 'threadId')!;
      const v = p.version;
      if (typeof v !== 'number' || !Number.isSafeInteger(v)) throw bad('version must be a whole number');
      const got = d.boards.version(id, v);
      if (!got) throw bad(`v${v} isn't kept any more: the board keeps the last 10.`);
      return { version: got };
    }),
    // planBoardMockups: a board's screens, Pick on one, and seeing them.
    'board.frames': guard((p) => {
      const frames = d.frames?.();
      if (!frames) throw new RpcError(ErrorCode.notAllowed, 'Plan board mockups is off. Turn it on in Settings > General > Experiments.');
      const id = thread(p);
      if (typeof p.pick === 'string') frames.pick(id, p.pick);
      if (p.seen === true) frames.seen(id);
      return { frames: d.boards.has(id) ? frames.list(id, typeof p.node === 'string' ? p.node : undefined) : [] };
    }),
    // Wayne's Approve plan, for the version he saw. Refused if it isn't the latest.
    'board.approve': guard((p) => {
      const id = thread(p);
      const v = p.version;
      if (typeof v !== 'number' || !Number.isSafeInteger(v)) throw bad('version must be a whole number');
      return { board: d.boards.approve(id, v) };
    }),
    // Boxes Wayne dragged ({ id: [x, y] | null }), or tidy: true to clear them.
    'board.move': guard((p) => {
      const id = thread(p);
      const raw = p.positions ?? {};
      if (typeof raw !== 'object' || Array.isArray(raw)) throw bad('positions must be an object of box id to [x, y]');
      return { board: d.boards.move(id, raw as Record<string, [number, number] | null>, p.tidy === true) };
    }),
    // The board's skills, by name.
    'board.skills': guard((p) => {
      const id = thread(p);
      if (!Array.isArray(p.skills) || !p.skills.every((s) => typeof s === 'string')) throw bad('skills must be a list of names');
      return { board: d.boards.setSkills(id, p.skills as string[]) };
    }),
    // Send to…'s Start: one box to one new nested thread, or (Phase 3) one
    // to-do on the project's coordinator. The same requestId returns what the
    // first Start made.
    'board.send': guard((p) => {
      const id = thread(p);
      const nodeId = str(p, 'nodeId')!;
      const requestId = str(p, 'requestId')!;
      if (requestId.length > 100) throw bad('requestId is too long');
      const earlier = d.boards.sent(id, requestId);
      if (earlier) return { board: mustBoard(id), repeated: true, ...(d.boards.get(id)?.links[earlier.node]?.kind === 'coordinator' ? { taskId: earlier.ref } : { threadId: earlier.ref }) };
      const board = d.boards.mustSend(id, nodeId);
      const node = board.graph.nodes.find((n) => n.id === nodeId)!;
      const planner = d.runner.get(id);
      const brief = str(p, 'brief', false);
      const title = str(p, 'title', false)?.trim() || node.label;
      const at = new Date(now()).toISOString();

      if (p.destination === 'coordinator') {
        const port = d.coordinatorsOn?.() ? d.coordinators?.() : undefined;
        const where = port?.forThread(id);
        if (!port || !where) throw bad("There's no active project coordinator for this thread, so the box wasn't handed over.");
        // Coordinators run Claude only today.
        const text = withBriefHeader(brief ?? buildBrief(board, nodeId, 'claude'), id, node, board.version, board.skills, 'claude');
        const { taskId } = port.addTask(where.projectId, { title, doneWhen: `The plan board's box "${node.label}" is built, as its brief says.`, brief: text });
        const next = d.boards.link(id, nodeId, { kind: 'coordinator', projectId: where.projectId, taskId, threadId: where.threadId, planVersion: board.version, at }, requestId);
        return { board: next, taskId };
      }
      if (p.destination !== undefined && p.destination !== 'thread') throw bad('destination must be "thread" or "coordinator"');

      const accountId = str(p, 'accountId', false);
      const poolId = accountId ? undefined : str(p, 'poolId', false);
      const provider = providerOf(accountId, poolId, str(p, 'provider', false));
      const mode = str(p, 'permissionMode', false);
      if (mode !== undefined && !MODES.has(mode)) throw bad('permissionMode must be full, ask, auto or plan');
      const effort = str(p, 'effort', false);
      if (effort !== undefined && !EFFORTS.has(effort)) throw bad('effort is not a level');
      const text = withBriefHeader(brief ?? buildBrief(board, nodeId, provider), id, node, board.version, board.skills, provider);
      const res = d.runner.create({
        text,
        // The planner's folder, as create_thread's default is.
        cwd: planner.cwd,
        ...(accountId ? { accountId } : {}),
        ...(poolId ? { poolId } : {}),
        ...(!accountId && !poolId && (provider === 'codex' || provider === 'claude') ? { provider: provider as Provider } : {}),
        ...(str(p, 'model', false) ? { model: str(p, 'model', false) } : {}),
        ...(effort ? { effort: effort as EffortLevel } : {}),
        // Inherits the planner's access unless Wayne picked another in the
        // sheet, but never plan mode: a box's thread is there to build it.
        permissionMode: (mode as PermissionMode | undefined) ?? (planner.permissionMode === 'plan' ? 'full' : planner.permissionMode),
        title,
        // Nested under the planner (docs/protocol/threads.md, "Nested threads").
        startedBy: { kind: 'thread', label: 'Plan board', threadId: id, why: `Box "${node.label}" of the plan` },
      });
      const next = d.boards.link(id, nodeId, { threadId: res.thread.id, harness: res.thread.provider, planVersion: board.version, at }, requestId);
      return { board: next, thread: res.thread };
    }),
  };
}
