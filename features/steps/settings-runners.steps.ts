// Settings › Runners (features/settings-runners.feature). Each runner is a
// line as both apps' dumps give it: "mac-mini: Online", with " *" after the name
// where new threads start, in the order the section lists them.
import { Then, When } from '@cucumber/cucumber';
import type { ThreadbearWorld } from '../support/world.ts';

type RunnersSection = { rows: string[]; default: string; canOrder: boolean; directLink: boolean };
const section = (d: unknown) => (d as { settingsWindow?: { runners?: RunnersSection | null } | null }).settingsWindow?.runners;

Then('Settings lists the runners {string} and {string}', async function (this: ThreadbearWorld, first: string, second: string) {
  await this.until(`Settings listing "${first}", then "${second}"`, (d) => {
    const rows = section(d)?.rows ?? [];
    return rows.length === 2 && rows[0] === first && rows[1] === second;
  });
});

Then('Settings says new threads start on {string}', async function (this: ThreadbearWorld, name: string) {
  await this.until(`new threads starting on ${name}`, (d) => section(d)?.default === name);
});

// As the section's picker does. The Swift picker is an AppKit pop-up, which a
// script can't open, so both apps take it as a command.
When('I pick {string} for new threads in Settings', async function (this: ThreadbearWorld, name: string) {
  await this.app.do({ cmd: 'settings', defaultRunner: name });
});

When('I drag {string} onto {string} in Settings', async function (this: ThreadbearWorld, name: string, onto: string) {
  await this.until('the runners in an order that can change', (d) => section(d)?.canOrder === true);
  await this.app.do({ cmd: 'settings', drag: name, onto });
});

// Cloud runners (settings_window/cloud.rs): the section's dump has each
// runner's name and line, and the last job's steps as "ok Box image" lines.
type CloudSection = { runners?: { name: string; line: string }[]; steps?: string[] | null; running?: boolean };
const cloud = (d: unknown) => (section(d) as (RunnersSection & { cloud?: CloudSection }) | null | undefined)?.cloud;

Then('Settings shows no cloud runner yet', async function (this: ThreadbearWorld) {
  await this.until('an empty Cloud runners section', (d) => Array.isArray(cloud(d)?.runners) && cloud(d)?.runners?.length === 0);
});

When('I add a cloud runner in Settings with a Fly token', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'settings', cloudSetup: { token: `FlyV1 fm2_${'a'.repeat(40)}`, app: 'tb-boxes-alex' } });
});

Then('Settings lists the cloud runner {string} as {string}', async function (this: ThreadbearWorld, name: string, line: string) {
  await this.until(`${name} under Cloud runners`, (d) => Boolean(cloud(d)?.runners?.some((r) => r.name === name && r.line === line)));
});

Then('Settings shows the cloud step {string}', async function (this: ThreadbearWorld, step: string) {
  await this.until(`the step "${step}"`, (d) => Boolean(cloud(d)?.steps?.includes(step)));
});
