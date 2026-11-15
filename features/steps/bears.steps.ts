import { Then, When } from '@cucumber/cucumber';
import { sleep } from '../support/driver.ts';
import assert from 'node:assert/strict';
import type { ThreadbearWorld } from '../support/world.ts';

Then('the bear thread {string} has the tile {string}', async function (this: ThreadbearWorld, id: string, tile: string) {
  await this.until(`${id} with ${tile}`, d => d.sidebar.sections.flatMap(s => s.threads).some(t => t.id === id && t.tile === tile));
});
When('I open the visible Designer thread menu', async function (this: ThreadbearWorld) {
  await this.until('Designer in the sidebar', d => d.sidebar.sections.flatMap(s => s.threads).some(t => t.id === 'demo_codex_poster'));
  const answer = await this.app.do({cmd:'realMenu', tipPrefix:'Make a poster for the meetup', right:true});
  this.lastAnswer = JSON.stringify(answer);
});
Then('the Designer menu offers Bear Colour', async function (this: ThreadbearWorld) {
  assert.ok(this.lastAnswer?.includes('Bear Colour'), this.lastAnswer);
});

When('I type a Writer mention in a local thread', async function (this: ThreadbearWorld) {
  await this.open('Tidy the imports in the uploader');
  await this.app.do({cmd:'focus', target:'composer'});
  await this.app.do({cmd:'type', text:'Hey @wr', replace:true});
});
Then('the bear picker offers Writer', async function (this: ThreadbearWorld) {
  for (let n=0; n<50; n++) {
    try { await this.app.do({cmd:'label-point', label:'Mention Writer'}); return; }
    catch { await sleep(100); }
  }
  throw new Error('Writer never appeared in the bear picker');
});
When('I complete the bear mention with Tab', async function (this: ThreadbearWorld) {
  await this.app.do({cmd:'menu-key', key:'tab'});
});
Then('the message box reads {string}', async function (this: ThreadbearWorld, text: string) {
  await this.until(`composer reads ${text}`, d => d.composer.text === text);
});
When('I dismiss the bear picker', async function (this: ThreadbearWorld) {
  await this.app.do({cmd:'menu-key', key:'escape'});
});
Then('the bear picker stays closed', async function (this: ThreadbearWorld) {
  await sleep(250);
  await assert.rejects(this.app.do({cmd:'label-point', label:'Mention Writer'}));
});
When('I open the Head table project page', async function (this: ThreadbearWorld) {
  await this.until('projects loaded', d => d.sidebar.sections.some(s => s.threads.length > 0));
  await this.app.do({cmd:'project', id:'p_7ab1e001'});
});
Then('the table page shows Head Bear and Writer', async function (this: ThreadbearWorld) {
  for (const label of ['Head Bear sits at this table', 'Writer sits at this table']) {
    let found = false;
    for (let n=0; n<50; n++) {
      try { await this.app.do({cmd:'label-point', label}); found = true; break; }
      catch { await sleep(100); }
    }
    assert.ok(found, label);
  }
});
When("I type a Writer mention in another runner's thread", async function (this: ThreadbearWorld) {
  await this.open('Check the photo library backup');
  await this.app.do({cmd:'focus', target:'composer'});
  await this.app.do({cmd:'type', text:'Hey @wr', replace:true});
});
When('I open the parent of Launch crew', async function (this: ThreadbearWorld) {
  await this.until('projects loaded', d => d.sidebar.sections.some(s => s.threads.length > 0));
  await this.app.do({cmd:'project', title:'Threadbear'});
});
Then('the project page shows Writer at its table', async function (this: ThreadbearWorld) {
  for (let n=0; n<50; n++) {
    try { await this.app.do({cmd:'label-point', label:'Writer sits at this table'}); return; }
    catch { await sleep(100); }
  }
  throw new Error('Writer never appeared at Launch crew');
});
When("I put Designer's thread in Head table", async function (this: ThreadbearWorld) {
  await this.until('Designer in the sidebar', d => d.sidebar.sections.flatMap(s => s.threads).some(t => t.id === 'demo_codex_poster'));
  const raw = await this.app.do({cmd:'label-point', label:'Make a poster for the meetup', prefix:true});
  const point = String(raw).replace(/^ok /,'').trim().split(/\s+/).map(Number);
  assert.equal(point.length, 2, String(raw));
  await this.app.do({cmd:'click-at', x:point[0], y:point[1], right:true});
  await this.app.do({cmd:'click', title:'Projects'});
  await this.app.do({cmd:'click', title:'Head table'});
});
Then('the table page shows Designer', async function (this: ThreadbearWorld) {
  for (let n=0; n<50; n++) {
    try { await this.app.do({cmd:'label-point', label:'Designer sits at this table'}); return; }
    catch { await sleep(100); }
  }
  throw new Error('Designer never took its seat');
});
When('I press Shift-Return over the bear list', async function (this: ThreadbearWorld) {
  await this.app.do({cmd:'menu-key', key:'enter', modifiers:['shift']});
});
Then('the message box keeps the typed bear name and a new line', async function (this: ThreadbearWorld) {
  await this.until('newline without completing Writer', d => d.composer.text === 'Hey @wr\n');
});
When('I move focus out of the bear message box', async function (this: ThreadbearWorld) {
  await this.app.do({cmd:'click', tip:'Search threads'});
});
