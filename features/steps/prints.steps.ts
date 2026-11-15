// Prints (features/prints.feature): the mock runner's seeded Prints
// (app/Tools/mock-runner/server.mjs, addDemoPrints), their cards in a reply,
// the side pane that follows a new version, and Settings' Prints switch.
// The @manual scenarios still need a dev runner.
import { Then, When } from '@cucumber/cucumber';
import type { ThreadbearWorld } from '../support/world.ts';
import type { Dump, Row } from '../support/driver.ts';
import { previewLine } from '../support/pane.ts';

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

type PrintCard = { id: string; shows: string; title?: string | null; meta?: string; buttons?: string[] };

/** Every Print card the open thread's replies show. */
const cards = (d: Dump): PrintCard[] => d.transcript.rows.flatMap((r) => ((r as Row & { prints?: PrintCard[] }).prints ?? []));

Then('the reply shows the Print card {string} saying {string}', async function (this: ThreadbearWorld, title: string, words: string) {
  await this.until(`the card "${title}" saying "${words}"`, (d) => cards(d).some((c) => c.shows === 'card' && c.title === title && c.meta?.startsWith(words)));
});

Then('the card offers Open and Refresh', async function (this: ThreadbearWorld) {
  await this.until('a card with Open and Refresh', (d) => cards(d).some((c) => c.buttons?.join(',') === 'Open,Refresh'));
});

Then('the reply shows its print block as code', async function (this: ThreadbearWorld) {
  await this.until('the print block as code', (d) => cards(d).length > 0 && cards(d).every((c) => c.shows === 'code'));
});

When('a reply with a print block streams in', async function (this: ThreadbearWorld) {
  const d = await this.app.dump();
  await this.app.do({ cmd: 'mock', method: 'stream', params: { threadId: d.selection?.id, text: 'The chart:\n\n```print pg_invoice_totals\n```\n', hz: 60 } });
  await this.until('the streamed reply, its turn finished', (d) => d.transcript.rows.some((r) => r.kind === 'footer' && r.turn === 'turn_stream'), 20_000);
});

When('I click Open on the Print card', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', tip: 'Open this print beside the thread' });
});

Then('the side pane shows version {int} of {string}', async function (this: ThreadbearWorld, version: number, id: string) {
  await this.until(`the pane on ${id} v${version}`, (d) => previewLine(d)?.endsWith(`/pages/${id}/v${version}/index.html`) ?? false);
});

When('the thread saves version {int} of {string}', async function (this: ThreadbearWorld, _version: number, id: string) {
  await this.app.do({ cmd: 'mock', method: 'savePage', params: { pageId: id } });
});
