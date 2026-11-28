// Notes (features/notes.feature): the page over the main panel, its list,
// the open note's editor and the Settings switch. Read from the dump's
// `notesPage`; driven by the `notes` command and the same clicks and keys a
// person uses.
import { Then, When } from '@cucumber/cucumber';
import type { ThreadbearWorld } from '../support/world.ts';

type NotesInfo = { open: boolean; button: boolean; listed: string[]; title?: string | null; body?: string | null; asking?: string | null };
const notes = (d: unknown) => (d as { notesPage?: NotesInfo | null }).notesPage;

When('I press ⌘6', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'menu-key', modifiers: ['cmd'], key: '6' });
});

When('I open Notes', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'notes' });
  await this.until('Notes open', (d) => notes(d)?.open);
});

Then('Notes is open', async function (this: ThreadbearWorld) {
  await this.until('Notes open', (d) => notes(d)?.open);
});

Then('Notes is closed', async function (this: ThreadbearWorld) {
  await this.until('Notes closed', (d) => notes(d) && !notes(d)?.open);
});

Then('thread {string} is still selected under Notes', async function (this: ThreadbearWorld, id: string) {
  await this.until(`thread ${id} still selected`, (d) => d.selection?.id === id);
});

When('I press Escape on Notes', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'escape' });
});

When('I close Notes with its X', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', tip: 'Close Notes (Esc)' });
  await this.until('Notes closed', (d) => notes(d) && !notes(d)?.open);
});

When('I start a new note', async function (this: ThreadbearWorld) {
  const before = (await this.app.dump()) as unknown;
  const count = notes(before)?.listed.length ?? 0;
  await this.app.do({ cmd: 'click', tip: 'Start a new note' });
  await this.until('a new note open', (d) => (notes(d)?.listed.length ?? 0) === count + 1 && notes(d)?.body === '');
});

Then('the open note is called {string}', async function (this: ThreadbearWorld, title: string) {
  await this.until(`the note "${title}" open`, (d) => notes(d)?.title === title);
});

When('I type {string} in the note', async function (this: ThreadbearWorld, words: string) {
  await this.app.do({ cmd: 'type', focused: true, text: words });
});

When('I press Return in the note', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'menu-key', modifiers: [], key: 'enter' });
});

When('I rename the open note to {string}', async function (this: ThreadbearWorld, title: string) {
  await this.app.do({ cmd: 'notes', rename: title });
  await this.until(`the note renamed "${title}"`, (d) => notes(d)?.title === title);
});

When('I pick the note {string}', async function (this: ThreadbearWorld, title: string) {
  await this.app.do({ cmd: 'notes', pick: title });
  await this.until(`the note "${title}" open`, (d) => notes(d)?.title === title);
});

Then('the notes listed are {string}', async function (this: ThreadbearWorld, titles: string) {
  const want = titles ? titles.split(', ') : [];
  await this.until(`notes listed as ${titles || 'none'}`, (d) => JSON.stringify(notes(d)?.listed) === JSON.stringify(want));
});

Then('the note says {string}', async function (this: ThreadbearWorld, words: string) {
  const want = words.replaceAll('\\n', '\n');
  await this.until(`the note saying ${JSON.stringify(want)}`, (d) => notes(d)?.body === want);
});

When('I click the box on line {int} of the note', async function (this: ThreadbearWorld, line: number) {
  await this.app.do({ cmd: 'notes', tick: line });
});

When('I click {string} in the note', async function (this: ThreadbearWorld, text: string) {
  await this.app.do({ cmd: 'notes', clickLink: text });
});

When("I press the note's bin", async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', tip: 'Delete this note' });
});

Then('Notes asks {string}', async function (this: ThreadbearWorld, question: string) {
  await this.until(`"${question}"`, (d) => notes(d)?.asking === question);
});

When('I confirm the delete', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'notes', confirm: true });
});

async function showNotes(world: ThreadbearWorld, on: boolean) {
  await world.app.do({ cmd: 'settings' });
  await world.until('the Settings page', (d) => (d as { settingsWindow?: unknown }).settingsWindow);
  // Tall enough that the switch is on screen, as a person would scroll to it.
  // Wide enough for the sidebar and side pane too: Settings shares the main
  // window now.
  await world.app.do({ cmd: 'resize', settings: true, w: 1500, h: 1300 });
  const now = notes(await world.app.dump());
  // Show Notes is the first row of Experiments, at the foot of General:
  // `settings: true` scrolls it into view first, as a person would.
  if (now?.button !== on) await world.app.do({ cmd: 'click', tip: 'Show Notes in the bar on the left', settings: true });
  await world.until(`Show Notes ${on ? 'on' : 'off'}`, (d) => notes(d)?.button === on);
}

When('I turn Show Notes off in Settings', async function (this: ThreadbearWorld) {
  await showNotes(this, false);
});

When('I turn Show Notes on in Settings', async function (this: ThreadbearWorld) {
  await showNotes(this, true);
});

Then('the bar has no Notes button', async function (this: ThreadbearWorld) {
  await this.until('no Notes button', (d) => notes(d)?.button === false);
});

// The note's type (notes_page.rs): the − and + buttons and Sans or Mono,
// read from the dump's `textSize` and `font`.
const noteType = (d: unknown) => notes(d) as (NotesInfo & { textSize?: number; font?: string }) | null | undefined;

When('I press the note\'s {string} button {int} times', async function (this: ThreadbearWorld, which: string, times: number) {
  const tip = which === '+' ? 'Larger text' : 'Smaller text';
  for (let i = 0; i < times; i++) await this.app.do({ cmd: 'click', tip });
});

When('I set the note in {string}', async function (this: ThreadbearWorld, face: string) {
  await this.app.do({ cmd: 'click', tip: face === 'Mono' ? "Set the note in the app's code font" : "Set the note in the app's own type" });
});

Then('the note\'s text is {int} points in {string}', async function (this: ThreadbearWorld, size: number, face: string) {
  await this.until(`the note in ${face} at ${size}`, (d) => noteType(d)?.textSize === size && noteType(d)?.font === face.toLowerCase());
});
