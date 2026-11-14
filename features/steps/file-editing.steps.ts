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

When('I start editing the pane\'s file', async function (this: ThreadbearWorld) {
  // Like an editor (the default), it opened ready to type in already.
  const d = await this.until('the bar to offer the pencil, or the file being edited', (x) => (pane(x)?.previewMode === 'reading' || editing(x)) && x);
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

When('I turn on Edit files like an editor', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'preview', editor: true });
});

When('I turn off Edit files like an editor', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'preview', editor: false });
});

When('I turn off Save files automatically', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'preview', autosave: false });
});

When('I\'d answer {string} to saving the changes', async function (this: ThreadbearWorld, answer: string) {
  await this.app.do({ cmd: 'preview', answer });
});

const unsaved = (d: Parameters<typeof pane>[0]) => (pane(d) as { previewUnsaved?: boolean } | null | undefined)?.previewUnsaved;

Then('the pane\'s file isn\'t saved yet', async function (this: ThreadbearWorld) {
  await this.until('the file marked unsaved', (d) => unsaved(d) === true);
});

Then('the pane\'s file is saved', async function (this: ThreadbearWorld) {
  await this.until('the file marked saved', (d) => unsaved(d) === false);
});

Then('the file {string} on disk doesn\'t end with {string}', async function (this: ThreadbearWorld, relative: string, words: string) {
  const file = path.join(await threadFolder(this), relative);
  if (fs.readFileSync(file, 'utf8').trimEnd().endsWith(words)) throw new Error(`${relative} ends with "${words}"`);
});

const status = (d: Parameters<typeof pane>[0]) => (pane(d) as { previewStatus?: string | null } | null | undefined)?.previewStatus;

Then('the pane\'s status bar says {string}', async function (this: ThreadbearWorld, words: string) {
  await this.until(`the status bar saying "${words}"`, (d) => status(d) === words);
});

Then('the pane\'s status bar ends {string}', async function (this: ThreadbearWorld, words: string) {
  await this.until(`the status bar ending "${words}"`, (d) => status(d)?.endsWith(words));
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
