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

When('I customize the Scheduled page with its numbers at the bottom', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'mode', mode: 'scheduled' });
  await this.app.do({ cmd: 'look', page: 'scheduled', on: true, set: { hidden: ['picture', 'about'], places: { numbers: 'bottom' }, hiddenNumbers: ['on', 'dueSoon', 'byThreads', 'paused', 'runs'] } });
});
Then('the Scheduled numbers are drawn near the bottom of the window', async function(this: ThreadbearWorld) {
  const geometry = String(await this.app.do({ cmd: 'window' }));
  const height = Number(/x([0-9]+)$/.exec(geometry)?.[1]);
  const point = String(await this.app.do({ cmd: 'label-point', label: 'On schedules that run over and over, rather than once' }));
  const y = Number(point.split(' ').at(-1));
  assert.ok(height > 0 && y > height * 0.6, `${point} should be in the bottom of ${geometry}`);
});
When('I search for projects then hide the customized page header', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'mode', mode: 'projects' });
  await this.app.do({ cmd: 'search', projects: true, text: 'nothing matches this query' });
  await this.app.do({ cmd: 'look', page: 'projects', on: true, set: { hidden: ['header'], hiddenCardParts: ['art', 'state'] } });
});
Then('every project card is shown', async function(this: ThreadbearWorld) {
  await this.app.do({ cmd: 'label-point', label: 'Open Threadbear' });
  await this.app.do({ cmd: 'label-point', label: "Open Photo sync" });
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
