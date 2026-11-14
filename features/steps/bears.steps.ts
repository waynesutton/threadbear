import { Then, When } from '@cucumber/cucumber';
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
