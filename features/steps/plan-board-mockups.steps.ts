// Screens on the plan board (plan-board-mockups.feature, the planBoardMockups
// experiment): the mock's mock.boardFrames writes them; the scripted app's
// `board` command reads the strip.
import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import { board, box, untilBoard, type Board } from '../support/plan-board.ts';
import type { ThreadbearWorld } from '../support/world.ts';


type Screen = { id: string; variant: string; picked: boolean; changed: boolean; drawn: boolean };

Given('the agent of {string} drew screens A for {string}', async function (this: ThreadbearWorld, title: string, node: string) {
  const t = await this.until(`"${title}"`, (d) => this.thread(d, title));
  await this.app.do({ cmd: 'mock', method: 'boardFrames', params: { threadId: t.id, node, variants: ['A'] } });
});

Given('the agent of {string} drew screens A, B and C for {string}', async function (this: ThreadbearWorld, title: string, node: string) {
  const t = await this.until(`"${title}"`, (d) => this.thread(d, title));
  await this.app.do({ cmd: 'mock', method: 'boardFrames', params: { threadId: t.id, node, variants: ['A', 'B', 'C'] } });
});

When('I open the screens of {string}', async function (this: ThreadbearWorld, label: string) {
  await untilBoard(this, `the box "${label}"`, (b) => box(b, label));
  await board(this, { do: 'screens', box: label });
});

const screens = (b: Board) => (b as Board & { screens?: Screen[] }).screens ?? [];

Then('the board shows {int} screens for {string}', async function (this: ThreadbearWorld, n: number, _label: string) {
  await untilBoard(this, `${n} screens`, (b) => screens(b).length === n);
});

Then('every screen is drawn as a picture', async function (this: ThreadbearWorld) {
  await untilBoard(this, 'every screen drawn', (b) => screens(b).length > 0 && screens(b).every((s) => s.drawn));
});

When('I pick screen {string}', async function (this: ThreadbearWorld, variant: string) {
  await untilBoard(this, `screen ${variant}`, (b) => screens(b).some((s) => s.variant === variant));
  await board(this, { do: 'pick', variant });
});

Then('screen {string} is picked', async function (this: ThreadbearWorld, variant: string) {
  await untilBoard(this, `screen ${variant} picked`, (b) => screens(b).find((s) => s.variant === variant)?.picked && screens(b).filter((s) => s.picked).length === 1);
});

Then('screen {string} says Changed', async function (this: ThreadbearWorld, variant: string) {
  await untilBoard(this, `screen ${variant} Changed`, (b) => screens(b).find((s) => s.variant === variant)?.changed);
});

Then('the board has no Screens…', async function (this: ThreadbearWorld) {
  const b = await untilBoard(this, 'the board', (x) => x.version && x);
  assert.equal((b as Board & { screensButton?: boolean }).screensButton, false);
});
