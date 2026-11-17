import { fileURLToPath } from 'node:url';
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
  // fileURLToPath, not .pathname: on Windows that's /D:/... and no file.
  const file = fileURLToPath(new URL('../../app/Sources/Threadbear/Resources/Art/librarian.png', import.meta.url));
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
  // Tiles are labelled by their tips (home_page.rs).
  await this.app.do({ cmd: 'label-point', label: 'On schedules due to run in the next 24 hours' });
  await assert.rejects(this.app.do({ cmd: 'label-point', label: 'Schedules that will still run: not paused and not finished' }));
  await assert.rejects(this.app.do({ cmd: 'label-point', label: SCHEDULED_BEAR }));
});
When('I open Customize on the Scheduled page', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'look', page: 'scheduled', customize: true });
  await this.until('the Scheduled panel open', d => (d as unknown as { looks: { editing?: string } }).looks.editing === 'scheduled' && d);
});
Then('its panel has the Layout and Numbers sections', async function(this: ThreadbearWorld) {
  // The panel's switches are labelled by their tips. Reset sits at the
  // panel's foot, below a 768-point screen (GitHub's Linux runner), so it's
  // left to the Projects panel, which is short enough to show it.
  for (const label of ['Show or hide numbers on the page', 'Show or hide due in a day']) {
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
  // Reset sits at the panel's foot, below the fold on GitHub's Mac screen
  // (as on the Scheduled panel), so it isn't looked for here.
  for (const label of ['Show or hide the title, search and New project. Hidden, the cards show every project', 'The wide tile with the project\'s icon. Off, cards are just text']) {
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
const animation = Buffer.from('47494638396101000100800000ce33443355dd21ff0b4e45545343415045322e30030100000021f90400230000002c000000000100010000020244010021f90400230000002c00000000010001000002024c01003b', 'hex');
When('I choose an animated Librarian background', async function(this: ThreadbearWorld) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'look-animation-'));
  const file = path.join(dir, 'motion.gif');
  fs.writeFileSync(file, animation);
  try {
    await this.app.do({ cmd: 'mode', mode: 'librarian' });
    await this.app.do({ cmd: 'look', page: 'librarian', on: true, background: file });
    await this.until('the animated background copied', d => looks(d).librarian.background?.endsWith('.gif'));
  } finally { fs.rmSync(dir, { recursive: true, force: true }); }
});
Then('both background frames are kept and the background is drawn', async function(this: ThreadbearWorld) {
  const file = looks(await this.app.dump()).librarian.background!;
  const home = String(await this.app.do({ cmd: 'data-dir' })).replace(/^ok /, '');
  assert.deepEqual(fs.readFileSync(path.join(home, 'looks', file)), animation);
  await this.app.do({ cmd: 'label-point', label: 'Page background photo' });
});
When('I quickly edit my profile page twice', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'mode', mode: 'librarian' });
  await this.app.do({ cmd: 'look', page: 'librarian', on: true, customize: true });
  await this.until('profile loaded', d => profile(d).loaded && profile(d).draft);
  await this.app.do({ cmd: 'profile', set: { name: 'Wayne first', bio: 'First draft' } });
  await this.app.do({ cmd: 'profile', set: { name: 'Wayne latest', bio: 'Makes things with agents' } });
});
type ProfileDump = { loaded: boolean; saving: boolean; draft?: { name: string; bio: string; published: boolean; home: boolean; show: string[] }; saved?: { name: string; bio: string }; };
const profile = (d: Dump) => (d as unknown as { profilePage: ProfileDump }).profilePage;
Then('the profile page has saved my latest name and bio', async function(this: ThreadbearWorld) {
  await this.until('latest profile saved through the runner', d => !profile(d).saving && profile(d).saved?.name === 'Wayne latest' && profile(d).saved?.bio === 'Makes things with agents');
});
When('I reopen the Librarian Customize panel', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'look', page: 'librarian', customize: false });
  await this.app.do({ cmd: 'look', page: 'librarian', customize: true });
});
Then('the profile page has loaded my latest name and bio', async function(this: ThreadbearWorld) {
  await this.until('latest profile loaded', d => profile(d).loaded && profile(d).draft?.name === 'Wayne latest' && profile(d).draft?.bio === 'Makes things with agents');
});

When('I reopen the profile panel while my latest bio saves', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'profile', set: { bio: 'The newest bio' } });
  await this.app.do({ cmd: 'profile', load: true, delayMs: 4000 });
});
Then('the late load leaves my latest bio saved and on screen', async function(this: ThreadbearWorld) {
  const d = await this.until('the delayed load completed', d => profile(d).loaded && !profile(d).saving && d);
  assert.equal(profile(d).saved?.bio, 'The newest bio');
  assert.equal(profile(d).draft?.bio, 'The newest bio');
});

When('I open my profile controls without editing them', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'mode', mode: 'librarian' });
  await this.app.do({ cmd: 'look', page: 'librarian', on: true, customize: true });
  await this.until('profile controls loaded', d => profile(d).loaded && profile(d).draft);
});
Then('no profile has been sent and all its public parts are off', async function(this: ThreadbearWorld) {
  const state = profile(await this.app.dump());
  assert.equal(state.saved, null);
  assert.equal(state.saving, false);
  assert.equal(state.draft?.published, false);
  assert.equal(state.draft?.home, false);
  assert.deepEqual(state.draft?.show, []);
});
