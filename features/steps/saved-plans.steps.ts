// A Plan first turn's plan, saved as a file (saved-plans.feature): the
// runner pins it with show and leaves a note linking it (runner.ts
// `savePlan`), played by the mock's savedPlan.
import { Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import path from 'node:path';
import { okValue, type Dump } from '../support/driver.ts';
import { previewLine } from '../support/pane.ts';
import type { ThreadbearWorld } from '../support/world.ts';

const saved = new WeakMap<ThreadbearWorld, string>();

When('the agent of {string} saved its plan', async function (this: ThreadbearWorld, title: string) {
  const threadId = (await this.until(`a thread called "${title}"`, (d) => this.thread(d, title)))?.id;
  assert.ok(threadId, `no thread called "${title}"`);
  const out = JSON.parse(okValue(await this.app.do({ cmd: 'mock', method: 'savedPlan', params: { threadId } }))) as { path: string };
  saved.set(this, out.path);
});

Then('the pane is showing the saved plan', async function (this: ThreadbearWorld) {
  const file = saved.get(this);
  assert.ok(file, 'no plan was saved');
  await this.until(`the pane to show ${path.basename(file)}`, (d) => previewLine(d)?.endsWith(path.basename(file)));
});

const note = (d: Dump, words: string) => d.transcript.rows.find((r) => r.kind === 'note' && r.text.startsWith(words));

Then('the transcript shows the note {string} outside the turn\'s work', async function (this: ThreadbearWorld, words: string) {
  const row = await this.until(`the note "${words}"`, (d) => note(d, words));
  assert.ok(!row.inGroup, 'the note folded into the work');
});

When('I click the link {string} in the note {string}', async function (this: ThreadbearWorld, link: string, words: string) {
  const row = await this.until(`the note "${words}"`, (d) => note(d, words));
  await this.app.do({ cmd: 'peek', row: row.id, words: link, follow: true });
});
