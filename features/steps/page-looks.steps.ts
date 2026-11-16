import { When, Then } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import type { Dump } from '../support/driver.ts';
import type { ThreadbearWorld } from '../support/world.ts';

type Looks = { on: { librarian: boolean }; librarian: { greeting?: string; quote?: string; picture: string; background?: string }; };
const looks = (d: Dump) => (d as unknown as { looks: Looks }).looks;
const previous = new WeakMap<ThreadbearWorld, string>();

When('I customize the Librarian page with a greeting for {string} and a quote', async function(this: ThreadbearWorld, name: string) {
  await this.app.do({ cmd: 'mode', mode: 'librarian' });
  await this.app.do({ cmd: 'look', page: 'librarian', on: true, set: { greetingShown: true, greetingName: name, quoteShown: true, hidden: ["about", "numbers", "settings"] } });
});
Then('the customized page draws its greeting and quote', async function(this: ThreadbearWorld) {
  const d = await this.until('a greeting and quote', d => looks(d).librarian.greeting && looks(d).librarian.quote && d);
  for (const label of [looks(d).librarian.greeting!, looks(d).librarian.quote!]) {
    await this.app.do({ cmd: 'label-point', label });
  }
  previous.set(this, looks(d).librarian.quote!);
});
When('I ask for another quote', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', tip: 'Show another quote for today' });
});
Then('the customized page draws a different quote', async function(this: ThreadbearWorld) {
  const d = await this.until('another quote', d => looks(d).librarian.quote !== previous.get(this) && d);
  await this.app.do({ cmd: 'label-point', label: looks(d).librarian.quote });
});
When('I turn off the Librarian page customization', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'look', page: 'librarian', on: false });
});
Then('the standard Librarian page is shown and my look is kept', async function(this: ThreadbearWorld) {
  const d = await this.app.dump();
  assert.equal(looks(d).on.librarian, false);
  assert.ok(looks(d).librarian.greeting?.includes('Wayne'));
  assert.equal((d as unknown as { homePage: { title: string } }).homePage.title, 'Librarian');
});
When('I choose my own Librarian picture', async function(this: ThreadbearWorld) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'look-picture-'));
  const file = path.join(dir, 'picture.png');
  fs.copyFileSync(new URL('../../app/Sources/Threadbear/Resources/Art/librarian.png', import.meta.url), file);
  try {
    await this.app.do({ cmd: 'mode', mode: 'librarian' });
    await this.app.do({ cmd: 'look', page: 'librarian', on: true, picture: file });
    await this.until('own picture imported', d => looks(d).librarian.picture === 'own');
  } finally { fs.rmSync(dir, { recursive: true, force: true }); }
});
Then('my Librarian picture is still drawn', async function(this: ThreadbearWorld) {
  assert.equal(looks(await this.app.dump()).librarian.picture, 'own');
  await this.app.do({ cmd: 'label-point', label: 'Your picture' });
});

When('I choose a Librarian background', async function(this: ThreadbearWorld) {
  const file = new URL('../../app/Sources/Threadbear/Resources/Art/librarian.png', import.meta.url).pathname;
  await this.app.do({ cmd: 'look', page: 'librarian', background: file });
  await this.until('background copied', d => looks(d).librarian.background);
});
Then('the customized page draws its background', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'label-point', label: 'Page background photo' });
});

type ScheduledLook = { hidden: string[]; hiddenNumbers: string[] };
const scheduled = (d: Dump) => (d as unknown as { looks: { on: { scheduled: boolean }; scheduled: ScheduledLook } }).looks;
const SCHEDULED_BEAR = 'The bear winding a big alarm clock, beside a calendar';

When('I customize the Scheduled page without its bear or its On number', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'mode', mode: 'scheduled' });
  await this.app.do({ cmd: 'look', page: 'scheduled', on: true, set: { hidden: ['picture'], hiddenNumbers: ['on'] } });
  await this.until('the Scheduled look saved', d => scheduled(d).on.scheduled && scheduled(d).scheduled.hidden.includes('picture') && d);
});
Then('the customized Scheduled page draws its title and numbers without them', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'label-point', label: 'Scheduled' });
  await this.app.do({ cmd: 'label-point', label: 'Due in a day', prefix: true });
  await assert.rejects(this.app.do({ cmd: 'label-point', label: SCHEDULED_BEAR }));
});
When('I open Customize on the Scheduled page', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'look', page: 'scheduled', customize: true });
  await this.until('the Scheduled panel open', d => (d as unknown as { looks: { editing?: string } }).looks.editing === 'scheduled' && d);
});
Then('its panel has the Layout and Numbers sections', async function(this: ThreadbearWorld) {
  for (const label of ['Layout', 'Numbers', 'Reset to the standard page']) {
    await this.app.do({ cmd: 'label-point', label });
  }
});
When('I turn off the Scheduled page customization', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'look', page: 'scheduled', on: false });
});
Then('the standard Scheduled page is shown and my Scheduled look is kept', async function(this: ThreadbearWorld) {
  const d = await this.until('the standard Scheduled page', d => !scheduled(d).on.scheduled && d);
  assert.deepEqual(scheduled(d).scheduled.hiddenNumbers, ['on']);
  assert.equal((d as unknown as { homePage: { title: string } }).homePage.title, 'Scheduled');
  await this.app.do({ cmd: 'label-point', label: SCHEDULED_BEAR });
});

type ProjectsLook = { hidden: string[]; hiddenCardParts: string[] };
const projects = (d: Dump) => (d as unknown as { looks: { on: { projects: boolean }; editing?: string; projects: ProjectsLook } }).looks;

When('I customize the Projects page without its header or the cards\' icon tiles', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'mode', mode: 'projects' });
  await this.app.do({ cmd: 'look', page: 'projects', on: true, set: { hidden: ['header'], hiddenCardParts: ['art'] } });
  await this.until('the Projects look saved', d => projects(d).on.projects && projects(d).projects.hiddenCardParts.includes('art') && d);
});
Then('the customized Projects page draws its cards without them', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'label-point', label: 'Open ', prefix: true });
  await assert.rejects(this.app.do({ cmd: 'label-point', label: 'Search projects' }));
});
When('I open Customize on the Projects page', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'look', page: 'projects', customize: true });
  await this.until('the Projects panel open', d => projects(d).editing === 'projects' && d);
});
Then('its panel has the Layout and Cards sections', async function(this: ThreadbearWorld) {
  for (const label of ['Layout', 'Cards', 'Icon tile', 'Reset to the standard page']) {
    await this.app.do({ cmd: 'label-point', label });
  }
});
When('I turn off the Projects page customization', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'look', page: 'projects', on: false });
});
Then('the standard Projects page is shown and my Projects look is kept', async function(this: ThreadbearWorld) {
  const d = await this.until('the standard Projects page', d => !projects(d).on.projects && d);
  assert.deepEqual(projects(d).projects.hiddenCardParts, ['art']);
  await this.app.do({ cmd: 'label-point', label: 'Search projects' });
});
