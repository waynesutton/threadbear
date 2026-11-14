// Prints (features/prints.feature): the mock runner's seeded Prints
// (app/Tools/mock-runner/server.mjs, addDemoPrints), their cards in a reply,
// the side pane that follows a new version, and Settings' Prints switch.
// The @manual scenarios still need a dev runner.
import { Then } from '@cucumber/cucumber';
import type { ThreadbearWorld } from '../support/world.ts';

/** Settings' "Only make Prints when I ask", or null while it isn't shown. */
const whenAsked = (d: unknown) => (d as { settingsWindow?: { printsWhenAsked?: boolean | null } | null }).settingsWindow?.printsWhenAsked;

Then('Settings says threads make Prints without asking first', async function (this: ThreadbearWorld) {
  await this.until('Only make Prints when I ask off', (d) => whenAsked(d) === false);
});

Then('Settings says threads make Prints only when I ask', async function (this: ThreadbearWorld) {
  await this.until('Only make Prints when I ask on', (d) => whenAsked(d) === true);
});

Then('Settings has no Prints switch', async function (this: ThreadbearWorld) {
  // Settings has loaded the runner's settings (its default folder) first.
  await this.until('Settings without a Prints switch', (d) => {
    const s = (d as { settingsWindow?: { defaultFolder?: string | null; printsWhenAsked?: boolean | null } | null }).settingsWindow;
    return Boolean(s?.defaultFolder) && s?.printsWhenAsked == null;
  });
});

