import { Then, When } from '@cucumber/cucumber';
import fs from 'node:fs';
import path from 'node:path';
import type { ThreadbearWorld } from '../support/world.ts';
import { pane, previewLine } from '../support/pane.ts';

// The thread's folder on disk: the mock runner's own, under its temp data
// folder, so a scenario may read and rewrite the files there.
async function threadFolder(world: ThreadbearWorld): Promise<string> {
  const d = await world.until('the composer to know its folder', (x) => (x.composer as unknown as { cwd?: string }).cwd && x);
  return (d.composer as unknown as { cwd: string }).cwd.replace(/\/$/, '');
}

const editing = (d: Parameters<typeof pane>[0]) => pane(d)?.previewMode === 'editing' || pane(d)?.previewMode === 'markdown';

Then('the pane\'s file can be edited', async function (this: ThreadbearWorld) {
  await this.until('the bar to offer the pencil', (d) => pane(d)?.previewMode === 'reading');
});

// A file opens ready to type in unless Edit files like an editor is off:
// then the pencil.
When('I start editing the pane\'s file', async function (this: ThreadbearWorld) {
  const d = await this.until('the bar to offer the pencil, or the file being edited', (d) => (pane(d)?.previewMode === 'reading' || editing(d)) && d);
  if (pane(d)?.previewMode === 'reading') await this.app.do({ cmd: 'preview', edit: true });
});

Then('the pane\'s file is being edited', async function (this: ThreadbearWorld) {
  await this.until('the file being edited', (d) => editing(d) && previewLine(d)?.includes('(editing)'));
});

When('I type {string} at the end of the pane\'s file', async function (this: ThreadbearWorld, text: string) {
  await this.until('the file being edited', (d) => editing(d) && pane(d)?.previewText);
  await this.app.do({ cmd: 'preview', insert: text });
});

When('I type {string} at the end of the pane\'s file, without waiting', async function (this: ThreadbearWorld, text: string) {
  await this.until('the file being edited', (d) => editing(d) && pane(d)?.previewText);
  await this.app.do({ cmd: 'preview', insert: text, wait: false });
});

When('I save the pane\'s file', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'preview', save: true });
});

When('I\'m done editing the pane\'s file', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'preview', edit: false });
});

Then('the file {string} on disk ends with {string}', async function (this: ThreadbearWorld, relative: string, words: string) {
  const file = path.join(await threadFolder(this), relative);
  await this.until(`${relative} on disk to end with "${words}"`, () => fs.readFileSync(file, 'utf8').trimEnd().endsWith(words));
});

When('{string} in the thread\'s folder is rewritten as {string}', async function (this: ThreadbearWorld, relative: string, text: string) {
  fs.writeFileSync(path.join(await threadFolder(this), relative), `${text}\n`);
});

Then('the pane says {string}', async function (this: ThreadbearWorld, words: string) {
  await this.until(`the pane to say "${words}"`, (d) => pane(d)?.previewNote === words);
});

When('I keep mine in the pane', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'preview', resolve: 'mine' });
});

When('I take the file\'s side in the pane', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'preview', resolve: 'file' });
});

Then('the pane\'s file reads {string}', async function (this: ThreadbearWorld, words: string) {
  await this.until(`the editor to read "${words}"`, (d) => pane(d)?.previewText?.trim() === words);
});

// The bar under the text (`EditorStatusBar`): where the cursor is, and
// whether what's typed is in the file yet. Both parts are matched.
Then('the pane\'s status bar says {string}', async function (this: ThreadbearWorld, words: string) {
  await this.until(`the status bar to say "${words}"`, (d) => pane(d)?.previewStatus?.includes(words));
});

Then('the pane\'s file is marked unsaved', async function (this: ThreadbearWorld) {
  await this.until('the dot by the file\'s name', (d) => pane(d)?.previewUnsaved === true);
});

Then('the pane\'s file isn\'t marked unsaved', async function (this: ThreadbearWorld) {
  await this.until('no dot by the file\'s name', (d) => pane(d)?.previewUnsaved === false);
});

// A moment longer than autosave would take, then still as it was.
Then('the file {string} on disk still ends with {string}', async function (this: ThreadbearWorld, relative: string, words: string) {
  const file = path.join(await threadFolder(this), relative);
  await new Promise((r) => setTimeout(r, 1800));
  const text = fs.readFileSync(file, 'utf8').trimEnd();
  if (!text.endsWith(words)) throw new Error(`${relative} ends ${JSON.stringify(text.slice(-40))}`);
});

// Save, Don't Save or Cancel, given before Back, as a scripted app can't
// click an alert (the Swift app's `scriptedAnswer`).
When('I press Back in the pane, answering {string}', async function (this: ThreadbearWorld, answer: string) {
  const name = { 'Save': 'save', 'Don\'t Save': 'discard', 'Cancel': 'cancel' }[answer];
  if (!name) throw new Error(`answer must be Save, Don't Save or Cancel, not ${answer}`);
  await this.app.do({ cmd: 'preview', leaveAnswer: name });
  await this.app.do({ cmd: 'preview', back: true });
});
