import { Then, When } from '@cucumber/cucumber';
import { sleep } from '../support/driver.ts';
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
When('I open the Tasks customize panel', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', title: 'Customize Tasks' });
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
  // The alert animates into place; click its final position, not the moving button.
  await sleep(300);
  await this.app.do({ cmd: 'click', title: 'Delete task' });
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

When('I press Escape in Tasks', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'menu-key', key: 'escape' });
});
Then('the Tasks editor is closed', async function (this: ThreadbearWorld) {
  await this.until('editor closed', d => (d as { tasksPage?: { editorOpen: boolean } }).tasksPage?.editorOpen === false);
});
Then('the Tasks editor is open', async function (this: ThreadbearWorld) {
  await this.until('editor open', d => (d as { tasksPage?: { editorOpen: boolean } }).tasksPage?.editorOpen === true);
});
When('I cancel deleting the visible task', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', title: 'Delete' });
  // The alert animates into place; click its final position, not the moving button.
  await sleep(300);
  await this.app.do({ cmd: 'click', title: 'Keep task' });
});

When('I press {string} in Tasks', async function (this: ThreadbearWorld, key: string) {
  await this.app.do({ cmd: 'menu-key', key });
});
Then('the selected task is {string}', async function (this: ThreadbearWorld, title: string) {
  await this.until('selected task', d => (d as { tasksPage?: { selectedTask?: string } }).tasksPage?.selectedTask === task(d, title)?.id);
});
When('I send the visible task to a thread', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', title: 'Send to Thread' });
});
Then('the task {string} has a thread', async function (this: ThreadbearWorld, title: string) {
  await this.until('task thread linked', d => !!(task(d, title) as { thread?: { id?: string } } | undefined)?.thread?.id);
});
Then('Tasks offers View Thread', async function (this: ThreadbearWorld) {
  await this.until('View Thread shown', d => drawn(d).includes('View Thread'));
});

When('I choose a Tasks photo', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'tasks', photoPath: new URL('../../app/icons/threadbear.png', import.meta.url).pathname });
});
Then('the Tasks photo is displayed', async function (this: ThreadbearWorld) {
  await this.until('photo decoded for display', d => (d as { tasksPage?: { photoLoaded?: boolean } }).tasksPage?.photoLoaded === true);
});

When('I narrow the Tasks window', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'resize', w: 1000, h: 700 });
});

// The task's own page (desktop/gpui/src/task_detail.rs).
type TaskFull = { id: string; title: string; priority?: string | null; dueDate?: string | null; parentId?: string | null; links?: { title?: string }[] };
type Detail = { task?: string; picker?: string | null; activity?: { kind: string; body: string }[] | { Err: string } | null };
const full = (d: unknown, title: string) => (tasks(d)?.tasks as TaskFull[] | undefined)?.find((t) => t.title === title);
const detail = (d: unknown) => (d as { tasksPage?: { detail?: Detail | null } }).tasksPage?.detail;
Then("the task's page is open on {string}", async function (this: ThreadbearWorld, title: string) {
  await this.until(`page of ${title}`, (d) => !!detail(d) && detail(d)?.task === full(d, title)?.id);
});
When("I set the task's priority to {string}", async function (this: ThreadbearWorld, priority: string) {
  await this.app.do({ cmd: 'tasks', priority });
});
When("I set the task's due date to {string}", async function (this: ThreadbearWorld, due: string) {
  await this.app.do({ cmd: 'tasks', due });
});
Then('the task {string} has priority {string} and a due date', async function (this: ThreadbearWorld, title: string, priority: string) {
  await this.until(`${title} ${priority}, due`, (d) => full(d, title)?.priority === priority && !!full(d, title)?.dueDate);
});
Then('the task {string} has a due date', async function (this: ThreadbearWorld, title: string) {
  await this.until(`${title} due`, (d) => !!full(d, title)?.dueDate);
});
// Scheduled's Tasks due section (desktop/gpui/src/scheduled.rs).
type DueRow = { id: string; title: string; label: string };
const dueRows = (d: unknown) => (d as { sidebar?: { dueTasks?: { tasks?: DueRow[] } } }).sidebar?.dueTasks?.tasks ?? [];
Then("the Scheduled list's Tasks due shows {string} due {string}", async function (this: ThreadbearWorld, title: string, label: string) {
  await this.until(`${title} due ${label} in Scheduled`, (d) => dueRows(d).some((t) => t.title === title && t.label === label));
});
When('I click the due task {string} in Scheduled', async function (this: ThreadbearWorld, title: string) {
  const row = dueRows(await this.app.dump()).find((t) => t.title === title);
  if (!row) throw new Error(`no due task ${title} in Scheduled`);
  await this.app.do({ cmd: 'click', title: `${title}, due ${row.label}` });
});
Then('the task {string} has no due date', async function (this: ThreadbearWorld, title: string) {
  await this.until(`${title} not due`, (d) => !!full(d, title) && !full(d, title)?.dueDate);
});
When('I add the sub-task {string}', async function (this: ThreadbearWorld, title: string) {
  await this.app.do({ cmd: 'tasks', subtask: title });
});
Then('the task {string} is a sub-task of {string}', async function (this: ThreadbearWorld, title: string, parent: string) {
  await this.until(`${title} under ${parent}`, (d) => !!full(d, parent) && full(d, title)?.parentId === full(d, parent)?.id);
});
Then('the top level of Tasks does not list {string}', async function (this: ThreadbearWorld, title: string) {
  await this.until(`${title} not at the top level`, (d) => {
    const top = (tasks(d) as { topLevel?: string[] } | undefined)?.topLevel;
    return !!top && !top.includes(title);
  });
});
When('I comment {string}', async function (this: ThreadbearWorld, body: string) {
  await this.app.do({ cmd: 'tasks', comment: body });
});
Then("the task's activity shows the comment {string}", async function (this: ThreadbearWorld, body: string) {
  await this.until(`comment ${body}`, (d) => {
    const items = detail(d)?.activity;
    // The box is at the foot of the page, often below the fold, so this reads the page's list.
    return Array.isArray(items) && items.some((i) => i.kind === 'comment' && i.body === body);
  });
});
When('I add the link {string} titled {string}', async function (this: ThreadbearWorld, url: string, title: string) {
  await this.app.do({ cmd: 'tasks', addLink: url, linkTitle: title });
});
Then('the task {string} links to {string}', async function (this: ThreadbearWorld, title: string, link: string) {
  await this.until(`${title} links to ${link}`, (d) => !!full(d, title)?.links?.some((l) => l.title === link));
});
Then('Tasks shows {string}', async function (this: ThreadbearWorld, words: string) {
  await this.until(`"${words}" drawn`, (d) => drawn(d).includes(words));
});
When("I open the task's due date picker", async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'tasks', picker: 'due' });
  await this.until('due picker open', (d) => detail(d)?.picker === 'due');
});
When('I turn on Wrap Columns', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'tasks', wrap: true });
});
Then('the Tasks board wraps its columns', async function (this: ThreadbearWorld) {
  await this.until('columns wrapped', (d) => (tasks(d) as { wrapColumns?: boolean } | undefined)?.wrapColumns === true && tasks(d)?.view === 'board');
});
Then('the app says {string}', async function (this: ThreadbearWorld, words: string) {
  await this.until(`the note "${words}"`, (d) => (d as unknown as { importNote?: string | null }).importNote === words);
});

When("I open the task's Print search", async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', title: 'Attach Print' });
  await this.until('Print search open', d => detail(d)?.picker === 'print');
});
When("I search the task's Prints for {string}", async function (this: ThreadbearWorld, query: string) {
  await this.app.do({ cmd: 'menu-key', modifiers: ['cmd'], key: 'a' });
  await this.app.do({ cmd: 'type', focused: true, text: query });
});
When('I choose the matching Print {string}', async function (this: ThreadbearWorld, title: string) {
  await this.app.do({ cmd: 'click', title });
});

Then('the Print search offers {string}', async function (this: ThreadbearWorld, title: string) {
  await this.app.do({ cmd: 'label-point', label: title });
});

Then('the Print search remains open', async function (this: ThreadbearWorld) {
  await this.until('attached result does not dismiss search', d => detail(d)?.picker === 'print');
});
