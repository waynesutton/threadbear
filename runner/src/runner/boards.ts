// Plan boards (experiment planBoard, experiments/planBoard.md): the one event
// the runner adds to a thread's log when its agent writes a new version of
// its plan (boards.ts has the files). Part of the Runner (runner.ts), as the
// other areas are. Wayne's own changes (Approve plan, a drag, the skills)
// don't log an event: the app gets the board back from the RPC, so they
// don't move the thread up the list or look like new activity.
import type { Runner } from '../runner.ts';
import type { RunnerPart } from './part.ts';
import type { PlanBoard } from '../boards.ts';

export const boardsPart = {
  emitBoard(this: Runner, threadId: string, board: PlanBoard): void {
    if (!this.has(threadId)) return;
    const st = this.mustGet(threadId);
    this.emit(st, { type: 'board.updated', board });
  },

  // When Wayne acts on the Plan board itself (Approve plan, Pick take, Send box),
  // any open asks card on the planner thread is settled to needs "nothing" so
  // the thread doesn't stay stuck saying "Needs you" in two places at once.
  settleBoardNeeds(this: Runner, threadId: string): void {
    if (!this.has(threadId)) return;
    const st = this.mustGet(threadId);
    const { rec } = st;
    if (rec.needs?.needs === 'you') {
      rec.needs = { seq: rec.needs.seq, needs: 'nothing' };
      this.markDirty(st);
    }
  },
};

export type BoardsPart = RunnerPart<typeof boardsPart>;
