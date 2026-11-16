import { Then, When } from '@cucumber/cucumber';
import type { ThreadbearWorld } from '../support/world.ts';
type Tasks = {
  open: boolean;
  loaded: boolean;
  view: string;
  tasks: { title: string; status: string }[];
};
const drawn = (d: unknown) => (d as { tasksDrawn?: string[] }).tasksDrawn ?? [];
const tasks = (d: unknown) => (d as { tasksPage?: Tasks }).tasksPage;
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
Then('Tasks shows the {string} view', async function (this: ThreadbearWorld, view: string) {
  await this.until(`Tasks ${view} view`, (d) => tasks(d)?.open && tasks(d)?.loaded && tasks(d)?.view === view && drawn(d).includes('Tasks'));
});
Then('Tasks contains {string}', async function (this: ThreadbearWorld, title: string) {
  await this.until(`Task ${title}`, (d) => tasks(d)?.tasks?.some((t) => t.title === title) && drawn(d).includes(title));
});

When('I choose the Tasks List view', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', title: 'List' });
});
When('I make this the default Tasks view', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', title: 'Set as default' });
  await this.until('default view saved', (d) => tasks(d)?.loaded && drawn(d).includes('Default view'));
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

When('I edit the task {string}', async function (this: ThreadbearWorld, title: string) {
  await this.app.do({ cmd: 'click', title });
});
When('another device completes the task {string}', async function (this: ThreadbearWorld, title: string) {
  await this.app.do({ cmd: 'mock', method: 'tasks', params: { complete: title } });
  await this.until('external change received', (d) => tasks(d)?.loaded && tasks(d)?.tasks.some((t) => t.title === title && t.status === 'done'));
});
Then('the task {string} is done', async function (this: ThreadbearWorld, title: string) {
  await this.until('completed task', (d) => tasks(d)?.tasks.some((t) => t.title === title && t.status === 'done') && drawn(d).includes(title));
});

When('I save the task title as {string}', async function(this:ThreadbearWorld,title:string) {
  await this.app.do({cmd:'tasks',title,save:true});
});

When('I complete the task {string}', async function(this:ThreadbearWorld,title:string) {
  await this.app.do({cmd:'click',title:`Mark ${title} done`});
});
When('I delete the visible task', async function(this:ThreadbearWorld) {
  await this.app.do({cmd:'click',title:'Delete'});
});
When('I undo the task deletion', async function(this:ThreadbearWorld) {
  await this.app.do({cmd:'click',title:'Undo deletion'});
});
Then('Tasks does not contain {string}', async function(this:ThreadbearWorld,title:string) {
  await this.until('task removed from screen',(d)=>tasks(d)?.loaded && !tasks(d)?.tasks.some(t=>t.title===title) && !drawn(d).includes(title));
});
