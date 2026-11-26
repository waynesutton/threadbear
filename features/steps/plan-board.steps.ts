// The plan board (plan-board.feature, the planBoard experiment): what the
// scripted app's `board` command says about the board
// (desktop/gpui/src/plan_board.rs, `automation`), and the mock's
// mock.boardPlan and mock.boardThread for what agents and threads do.
import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import { okValue, sleep } from '../support/driver.ts';
import type { ThreadbearWorld } from '../support/world.ts';

type Box = { id: string; label: string; status: string | null; sendEnabled: boolean; changed: boolean };
type Board = {
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

const PLANNER = 'Plan the Acme dashboard';

async function board(world: ThreadbearWorld, cmd: Record<string, unknown> = {}): Promise<Board> {
  return JSON.parse(okValue(await world.app.do({ cmd: 'board', ...cmd }, 30))) as Board;
}

async function untilBoard<T>(world: ThreadbearWorld, what: string, check: (b: Board) => T | undefined | false | null, cmd: Record<string, unknown> = {}): Promise<T> {
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

const box = (b: Board, label: string) => b.boxes?.find((x) => x.label === label);

async function showBoard(world: ThreadbearWorld, title = PLANNER): Promise<void> {
  const d = await world.app.dump();
  if (d.selection?.title !== title) await world.open(title);
  await untilBoard(world, 'the board open with its plan', (b) => b.version, { do: 'show' });
}

async function approved(world: ThreadbearWorld, title: string): Promise<void> {
  await showBoard(world, title);
  await board(world, { do: 'approve' });
  await untilBoard(world, 'the plan approved', (b) => b.state?.startsWith('Approved'));
}

async function sent(world: ThreadbearWorld, label: string, harness: string): Promise<void> {
  await showBoard(world);
  const b = await board(world);
  if (!b.state?.startsWith('Approved')) await approved(world, PLANNER);
  await board(world, { do: 'send', box: label });
  await world.app.do({ cmd: 'board', do: 'start' });
  await untilBoard(world, `"${label}" sent to ${harness}`, (x) => box(x, label)?.status);
}

Given('the experiment {string} is on', async function (this: ThreadbearWorld, id: string) {
  // The scenario's tag turns it on in the mock's hello (features/support/world.ts).
  assert.equal(id, 'planBoard');
});

Given('the experiment {string} is off', async function (this: ThreadbearWorld, id: string) {
  assert.equal(id, 'planBoard');
});

Then('there is no {string} button', async function (this: ThreadbearWorld, title: string) {
  // A moment for hello and the thread's events to land, then the label is looked for.
  await sleep(800);
  await assert.rejects(this.app.do({ cmd: 'click', title }), /no button called/);
});

Then('the board has the box {string}', async function (this: ThreadbearWorld, label: string) {
  await untilBoard(this, `the box "${label}"`, (b) => box(b, label));
});

Then('the board has the group {string}', async function (this: ThreadbearWorld, label: string) {
  await untilBoard(this, `the group "${label}"`, (b) => b.groups?.includes(label));
});

Then('the board has an arrow {string} from {string} to {string}', async function (this: ThreadbearWorld, label: string, from: string, to: string) {
  await untilBoard(this, `an arrow "${label}"`, (b) => b.arrows?.some((a) => a.label === label && a.from === from && a.to === to));
});

Then('the board says {string}', async function (this: ThreadbearWorld, words: string) {
  await untilBoard(this, `the board saying "${words}"`, (b) => b.state?.startsWith(words));
});

When('I click the box {string} on the board', async function (this: ThreadbearWorld, label: string) {
  // The board reads its plan from the runner as it opens.
  await untilBoard(this, `the box "${label}"`, (b) => box(b, label));
  await board(this, { do: 'click', box: label });
});

When('I press Escape on the board', async function (this: ThreadbearWorld) {
  await board(this, { do: 'escape' });
});

Then('the composer has the chip {string}', async function (this: ThreadbearWorld, label: string) {
  await this.until(`a chip "${label}" in the message box`, (d) => d.composer.text.includes(`[${label}](board:`));
});

Then('the composer has no chip {string}', async function (this: ThreadbearWorld, label: string) {
  await this.until(`no chip "${label}" in the message box`, (d) => !d.composer.text.includes(`[${label}](board:`));
});

Then('{string} on the box {string} is disabled', async function (this: ThreadbearWorld, _button: string, label: string) {
  await untilBoard(this, `"${label}" can't be sent`, (b) => box(b, label) && !box(b, label)!.sendEnabled);
});

Then('{string} on the box {string} is enabled', async function (this: ThreadbearWorld, _button: string, label: string) {
  await untilBoard(this, `"${label}" can be sent`, (b) => box(b, label)?.sendEnabled);
});

Given('the plan of {string} is approved at version {int}', async function (this: ThreadbearWorld, title: string, version: number) {
  await approved(this, title);
  const b = await board(this);
  assert.equal(b.version, version);
});

When('the agent of {string} sends a new plan', async function (this: ThreadbearWorld, title: string) {
  const t = await this.until(`"${title}"`, (d) => this.thread(d, title));
  await this.app.do({ cmd: 'mock', method: 'boardPlan', params: { threadId: t.id } });
});

When('the agent of {string} sends a new plan that only changes {string}', async function (this: ThreadbearWorld, title: string, _label: string) {
  const t = await this.until(`"${title}"`, (d) => this.thread(d, title));
  await this.app.do({ cmd: 'mock', method: 'boardPlan', params: { threadId: t.id } });
});

When('the agent of {string} sends a plan that doesn\'t parse', async function (this: ThreadbearWorld, title: string) {
  await showBoard(this, title);
  const t = await this.until(`"${title}"`, (d) => this.thread(d, title));
  await this.app.do({ cmd: 'mock', method: 'boardPlan', params: { threadId: t.id, bad: true } });
});

Then('the board still shows version {int}', async function (this: ThreadbearWorld, version: number) {
  await sleep(500);
  const b = await board(this, { do: 'reload' });
  await untilBoard(this, `version ${version}`, (x) => x.version === version && b);
});

When('I send the box {string} to {string}', async function (this: ThreadbearWorld, label: string, _harness: string) {
  await board(this, { do: 'send', box: label });
});

Then('the send sheet shows the brief for {string}', async function (this: ThreadbearWorld, label: string) {
  const s = JSON.parse(okValue(await this.app.do({ cmd: 'board', do: 'sheet' }))) as { brief: string };
  assert.match(s.brief, new RegExp(`^Plan board: thread \\S+, box \\S+ \\("${label}"\\), plan v1 \\(approved\\)\\.`));
});

Then('the send sheet\'s brief names the skill {string}', async function (this: ThreadbearWorld, skill: string) {
  const deadline = Date.now() + 10_000;
  for (;;) {
    const s = JSON.parse(okValue(await this.app.do({ cmd: 'board', do: 'sheet' }))) as { brief: string };
    if (s.brief.includes(`Use these skills: /${skill}`)) return;
    if (Date.now() > deadline) throw new Error(`The brief doesn't name /${skill}:\n${s.brief}`);
    await sleep(150);
  }
});

When('I press Start twice', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'board', do: 'start' });
  await this.app.do({ cmd: 'board', do: 'start' }).catch(() => undefined);
});

Then('one thread {string} is nested under {string}', async function (this: ThreadbearWorld, title: string, planner: string) {
  await this.until(`one "${title}" under "${planner}"`, (d) => {
    // Nested: no row of its own, one thread in the planner's group (counted
    // whether or not the group is open).
    const rows = d.sidebar.sections.flatMap((s) => s.threads);
    const host = rows.find((t) => t.title === planner);
    const g = host?.group;
    return !rows.some((t) => t.title === title) && g && g.working + g.needsYou + g.finished === 1;
  });
});

Then('the box {string} says {string}', async function (this: ThreadbearWorld, label: string, words: string) {
  await untilBoard(this, `"${label}" saying ${words}`, (b) => box(b, label)?.status === words);
});

Then('the box {string} still says {string}', async function (this: ThreadbearWorld, label: string, words: string) {
  await untilBoard(this, `"${label}" still saying ${words}`, (b) => box(b, label)?.status === words);
});

Given('the box {string} of {string} was sent to {string}', async function (this: ThreadbearWorld, label: string, _planner: string, harness: string) {
  await sent(this, label, harness);
});

When('the thread {string} asks a question', async function (this: ThreadbearWorld, title: string) {
  await this.app.do({ cmd: 'mock', method: 'boardThread', params: { title, do: 'ask' } });
});

When('the thread {string} finishes', async function (this: ThreadbearWorld, title: string) {
  await this.app.do({ cmd: 'mock', method: 'boardThread', params: { title, do: 'finish' } });
});

Then('{string} is in the Inbox', async function (this: ThreadbearWorld, title: string) {
  await this.until(`"${title}" in the Inbox`, (d) => d.sidebar.sections.find((s) => s.name === 'Inbox')?.threads.some((t) => t.title === title));
});

When('I pick the board skill {string}', async function (this: ThreadbearWorld, skill: string) {
  await board(this, { do: 'skill', skill });
  await untilBoard(this, `the skill ${skill} picked`, (b) => b.skills?.includes(skill));
});

Then('the planner of {string} has no new message', async function (this: ThreadbearWorld, _title: string) {
  const b = await board(this);
  assert.equal(b.asked ?? null, null, 'Ask planner to review was never pressed');
  const d = await this.app.dump();
  assert.ok(!d.transcript.rows.some((r) => r.kind === 'user' && r.text.includes('Please review the plan')), 'no review message in the planner');
});

Then('the board shows {string}', async function (this: ThreadbearWorld, _words: string) {
  await untilBoard(this, 'Ask planner to review offered', (b) => b.review);
});

Then('the planner of {string} has one new message asking it to review the plan', async function (this: ThreadbearWorld, _title: string) {
  await this.until('one review message in the planner', (d) => d.transcript.rows.filter((r) => r.kind === 'user' && r.text.includes('Please review the plan')).length === 1);
});
