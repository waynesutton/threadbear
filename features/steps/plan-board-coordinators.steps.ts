// Plan board coordinators (plan-board-coordinators.feature): skipped on a
// copy without Mike's coordinators. These steps are what a copy with them
// runs; the mock's coordinator demo comes with that copy's coordinators.feature.
import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import { okValue } from '../support/driver.ts';
import { board } from '../support/plan-board.ts';
import type { ThreadbearWorld } from '../support/world.ts';

Then('the send sheet has no coordinator destination', async function (this: ThreadbearWorld) {
  const s = JSON.parse(okValue(await this.app.do({ cmd: 'board', do: 'sheet' }))) as { coordinator?: boolean };
  assert.notEqual(s.coordinator, true);
});

Given('the project of {string} has an active coordinator', async function (this: ThreadbearWorld, title: string) {
  const t = await this.until(`"${title}"`, (d) => this.thread(d, title));
  await this.app.do({ cmd: 'mock', method: 'coordinator', params: { threadId: t.id } });
});

When('I give the box {string} to the project coordinator', async function (this: ThreadbearWorld, label: string) {
  await board(this, { do: 'send', box: label, destination: 'coordinator' });
});

Then('the coordinator has one to-do for {string}', async function (this: ThreadbearWorld, label: string) {
  const r = JSON.parse(okValue(await this.app.do({ cmd: 'mock', method: 'coordinatorTasks', params: {} }))) as { tasks: { title: string }[] };
  assert.equal(r.tasks.filter((t) => t.title === label).length, 1);
});
