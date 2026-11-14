import { Then, When } from '@cucumber/cucumber';
import type { ThreadbearWorld } from '../support/world.ts';

type Page = { tab: string; keyboard?: { rows?: string[] } };
const page = (d: unknown) => (d as { settingsPage?: Page | null }).settingsPage;
When('I press the Settings shortcut', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'menu-key', key: ',', modifiers: ['cmd'] });
});
When('I press the Settings gear', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', tipPrefix: 'Settings (' });
});
When('I close the standalone Settings window', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'settings', close: true });
});
Then('Settings covers the main panel on {string}', async function (this: ThreadbearWorld, tab: string) {
  await this.until(`Settings page on ${tab}`, (d) => page(d)?.tab === tab);
});
Then('Settings no longer covers the main panel', async function (this: ThreadbearWorld) {
  await this.until('the Settings page closed', (d) => !page(d));
});
Then('no separate Settings window is open', async function (this: ThreadbearWorld) {
  await this.until('no Settings window', (d) => !(d as { settingsWindow?: unknown }).settingsWindow);
});
When('I search the embedded Keyboard page for {string}', async function (this: ThreadbearWorld, text: string) {
  await this.app.do({ cmd: 'shortcuts', search: text });
});
Then('the embedded Keyboard page lists {string}', async function (this: ThreadbearWorld, text: string) {
  await this.until(`Keyboard command ${text}`, (d) => page(d)?.keyboard?.rows?.some((r) => r.startsWith(text + ':')));
});
When('I press Back from Settings', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'menu-key', key: '[', modifiers: ['cmd'] });
});
