// The plan board's specs' shared helpers (plan-board.feature,
// plan-board-mockups.feature): the scripted app's `board` command
// (desktop/gpui/src/plan_board.rs, `automation`).
import { okValue, sleep } from './driver.ts';
import type { ThreadbearWorld } from './world.ts';

export type Box = { id: string; label: string; status: string | null; sendEnabled: boolean; changed: boolean };
export type Board = {
  state?: string;
  version?: number;
  boxes?: Box[];
  groups?: string[];
  arrows?: { from: string; to: string; label: string | null }[];
  skills?: string[];
  chips?: string[];
  review?: string | null;
  asked?: string | null;
  loading?: boolean;
  error?: string | null;
};


export async function board(world: ThreadbearWorld, cmd: Record<string, unknown> = {}): Promise<Board> {
  return JSON.parse(okValue(await world.app.do({ cmd: 'board', ...cmd }, 30))) as Board;
}

export async function untilBoard<T>(world: ThreadbearWorld, what: string, check: (b: Board) => T | undefined | false | null, cmd: Record<string, unknown> = {}): Promise<T> {
  const deadline = Date.now() + 15_000;
  let last: Board | string = 'no answer yet';
  for (;;) {
    try {
      last = await board(world, cmd);
      const got = check(last);
      if (got) return got;
    } catch (e) {
      last = (e as Error).message;
    }
    if (Date.now() > deadline) throw new Error(`Never saw: ${what}\n\nThe board said: ${JSON.stringify(last)}`);
    await sleep(150);
  }
}

export const box = (b: Board, label: string) => b.boxes?.find((x) => x.label === label);

