import { Then, When } from '@cucumber/cucumber';
import type { ThreadbearWorld } from '../support/world.ts';
type Tasks = {
  open: boolean;
  loaded: boolean;
  view: string;
  columnOrder: string[];
  listOrder: string[];
  showDone: boolean;
  toast: string | null;
  quickDraft: string;
  tasks: { id: string; title: string; status: string }[];
};
const drawn = (d: unknown) => (d as { tasksDrawn?: string[] }).tasksDrawn ?? [];
const tasks = (d: unknown) => (d as { tasksPage?: Tasks }).tasksPage;
const task = (d: unknown, title: string) => tasks(d)?.tasks?.find((t) => t.title === title);
When('I press ⌘7', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'menu-key', modifiers: ['cmd'], key: '7' });
});
When('I open Tasks', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'tasks' });
  await this.until('Tasks loaded', (d) => tasks(d)?.loaded);
});
When('I add the task {string}', async function (this: ThreadbearWorld, title: string) {
  await this.app.do({ cmd: 'tasks', title, save: true });
});
When('I type {string} in the quick-add box and press Return', async function (this: ThreadbearWorld, text: string) {
  await this.app.do({ cmd: 'tasks', quick: text });
});
Then('Tasks shows the {string} view', async function (this: ThreadbearWorld, view: string) {
  await this.until(`Tasks ${view} view`, (d) => tasks(d)?.open && tasks(d)?.loaded && tasks(d)?.view === view && drawn(d).includes('Tasks'));
});
Then('Tasks contains {string}', async function (this: ThreadbearWorld, title: string) {
  await this.until(`Task ${title}`, (d) => !!task(d, title) && drawn(d).includes(title));
});
Then('the task {string} is in {string}', async function (this: ThreadbearWorld, title: string, status: string) {
  await this.until(`${title} in ${status}`, (d) => task(d, title)?.status === status);
});
Then('the list does not show {string}', async function (this: ThreadbearWorld, title: string) {
  await this.until(`${title} hidden`, (d) => tasks(d)?.view === 'list' && !tasks(d)?.showDone && !drawn(d).includes(title));
});
Then('Tasks offers to undo {string}', async function (this: ThreadbearWorld, text: string) {
  await this.until(`toast ${text}`, (d) => tasks(d)?.toast === text && drawn(d).includes(text));
});
When('I choose the Tasks List view', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', title: 'List' });
});
When('I choose the Tasks Board view', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', title: 'Board' });
});
When('I make this the default Tasks view', async function (this: ThreadbearWorld) {
  const view = tasks(await this.app.dump())?.view;
  await this.app.do({ cmd: 'tasks', makeDefault: true });
  await this.until('default view saved', (d) => (d as { tasksPage?: { defaultView?: string } }).tasksPage?.defaultView === view);
});
When('I close and reopen Tasks', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', title: 'Close' });
  await this.until('Tasks closed', (d) => !tasks(d)?.open);
  await this.app.do({ cmd: 'tasks' });
  await this.until('Tasks loaded', (d) => tasks(d)?.loaded);
});
When('the next task save will fail', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'mock', method: 'tasks', params: { failNext: true } });
});
Then('Tasks shows a save error', async function (this: ThreadbearWorld) {
  await this.until('save error drawn', (d) => drawn(d).includes('Task save rejected by the mock runner.'));
});
When('I retry saving the task draft', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'tasks', save: true });
});
When('I retry the quick-add', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'tasks', retryQuick: true });
});
Then('the quick-add box contains {string}', async function (this: ThreadbearWorld, text: string) {
  await this.until('quick-add draft retained', (d) => tasks(d)?.quickDraft === text);
});
When('I edit the task {string}', async function (this: ThreadbearWorld, title: string) {
  await this.app.do({ cmd: 'tasks', open: title });
  await this.until('editor open', (d) => (d as { tasksPage?: { editorOpen?: boolean } }).tasksPage?.editorOpen === true);
});
When('another device moves the task {string} to {string}', async function (this: ThreadbearWorld, title: string, status: string) {
  await this.app.do({ cmd: 'mock', method: 'tasks', params: { move: title, status } });
  await this.until('external change received', (d) => task(d, title)?.status === status);
});
When('I save the task title as {string}', async function (this: ThreadbearWorld, title: string) {
  await this.app.do({ cmd: 'tasks', title, save: true });
});
When('I complete the task {string}', async function (this: ThreadbearWorld, title: string) {
  await this.app.do({ cmd: 'click', title: `Mark ${title} done` });
});
When('I delete the visible task', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', title: 'Delete' });
});
When('I press Undo', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', title: 'Undo' });
});
Then('Tasks does not contain {string}', async function (this: ThreadbearWorld, title: string) {
  await this.until('task removed from screen', (d) => tasks(d)?.loaded && !task(d, title) && !drawn(d).includes(title));
});
When('I drag the task {string} to {string}', async function (this: ThreadbearWorld, title: string, status: string) {
  await this.app.do({ cmd: 'tasks', moveTask: title, status });
});
When('I drag the {string} column onto {string}', async function (this: ThreadbearWorld, from: string, to: string) {
  await this.app.do({ cmd: 'tasks', moveStatus: from, to });
});
When('I drag the {string} section onto {string}', async function (this: ThreadbearWorld, from: string, to: string) {
  await this.app.do({ cmd: 'tasks', moveStatus: from, to });
});
Then("the board's columns are {string}", async function (this: ThreadbearWorld, order: string) {
  await this.until(`columns ${order}`, (d) => tasks(d)?.columnOrder?.join(', ') === order);
});
Then("the list's sections are {string}", async function (this: ThreadbearWorld, order: string) {
  await this.until(`sections ${order}`, (d) => tasks(d)?.listOrder?.join(', ') === order);
});

When('I narrow the Tasks window', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'resize', w: 1000, h: 700 });
});
