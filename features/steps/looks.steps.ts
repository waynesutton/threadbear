// Customizing the Librarian and Scheduled pages (features/looks.feature):
// Settings > General > Look's switches, the Customize button and its panel.
// Read from the dump's `homePage.look`; driven by the clicks a person uses,
// found by their tooltips.
import assert from 'node:assert/strict';
import { Given, Then, When } from '@cucumber/cucumber';
import type { ThreadbearWorld } from '../support/world.ts';

type Look = { editing: boolean; layout: string[]; picture: string } | null;
const look = (d: unknown): Look | undefined => (d as { homePage?: { look?: Look } | null }).homePage?.look;

const switches: Record<string, string> = {
  'Customize the Librarian page': 'Add Customize to the Librarian page: its picture, a background, the layout, and what shows',
  'Customize the Scheduled page': 'Add Customize to the Scheduled page: its picture, a background, the layout, and what shows',
};

async function turnOn(world: ThreadbearWorld, name: string) {
  const tip = switches[name];
  assert.ok(tip, `no Look switch called ${name}`);
  await world.app.do({ cmd: 'settings' });
  await world.until('the Settings page', (d) => (d as { settingsWindow?: unknown }).settingsWindow);
  // Tall enough that the Look section is on screen, as a person would scroll to it.
  await world.app.do({ cmd: 'resize', settings: true, w: 880, h: 1600 });
  await world.app.do({ cmd: 'click', tip });
  await world.app.do({ cmd: 'settings', close: true });
}

When('I turn on {string} in Settings', async function (this: ThreadbearWorld, name: string) {
  await turnOn(this, name);
});

Given('{string} is on in Settings', async function (this: ThreadbearWorld, name: string) {
  await turnOn(this, name);
});

Then("the page isn't customized", async function (this: ThreadbearWorld) {
  await this.until('the page as it ships', (d) => look(d) === null);
});

Then('the page is customized with {string}', async function (this: ThreadbearWorld, parts: string) {
  const want = parts.split(', ');
  const got = await this.until(`the page laid out as ${parts}`, (d) => {
    const l = look(d);
    return l && l.layout.join(', ') === want.join(', ') ? l : undefined;
  });
  assert.deepEqual(got.layout, want);
});

When('I open Customize on the page', async function (this: ThreadbearWorld) {
  await this.until('a customized page', (d) => look(d));
  await this.app.do({ cmd: 'click', tip: 'Change the picture, the background, and where each part of this page sits' });
});

Then('the Customize panel is open', async function (this: ThreadbearWorld) {
  await this.until('the Customize panel', (d) => look(d)?.editing);
});

When("I switch {string} off in Customize's layout", async function (this: ThreadbearWorld, part: string) {
  await this.app.do({ cmd: 'click', tip: `Show or hide ${part.toLowerCase()} on the page` });
});

When('I reset the page in Customize', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', tip: 'Put the bear, the layout and every part back as Threadbear ships them, and take away your pictures' });
});

When('I choose {string} in Customize', async function (this: ThreadbearWorld, tip: string) {
  await this.app.do({ cmd: 'click', tip });
});
