import { When, Then } from '@cucumber/cucumber';
import type { ThreadbearWorld } from '../support/world.ts';
type Skills = { open: boolean; list: {name:string}[]; text?:string };
const skills = (d: unknown) => (d as {skillsPage?:Skills}).skillsPage;
When('I open the Skills page', async function(this:ThreadbearWorld) {
  await this.app.do({cmd:'menu-key',key:'5',modifiers:['cmd']});
});
Then('the Skills page lists {string}', async function(this:ThreadbearWorld,name:string) {
  await this.until(`Skills lists ${name}`,d=>skills(d)?.open && skills(d)?.list.some(s=>s.name===name));
});
Then('the Skills page is closed', async function(this:ThreadbearWorld) {
  await this.until('Skills closed',d=>!skills(d)?.open);
});
When('I fill in a skill named {string} with {string}', async function(this:ThreadbearWorld,name:string,text:string) {
  await this.app.do({cmd:'skills',name,text});
});
Then('the skill editor contains {string}', async function(this:ThreadbearWorld,text:string) {
  await this.until('skill text',d=>skills(d)?.text===text);
});
When('I enter the skill link {string}', async function(this:ThreadbearWorld,url:string) {
  await this.app.do({cmd:'skills',url});
});
Then('the skill {string} is only used by its slash command', async function(this:ThreadbearWorld,name:string) {
  await this.until('automatic use off',d=>(skills(d)?.list as ({name:string;always:boolean}[]))?.some(s=>s.name===name&&!s.always));
});
Then('the Skills page does not list {string}', async function(this:ThreadbearWorld,name:string) {
  await this.until('skill removed',d=>!skills(d)?.list.some(s=>s.name===name));
});
Then('the Skills page explains {string}', async function(this:ThreadbearWorld,text:string) {
  await this.until('skill error',d=>(skills(d) as (Skills & {error?:string}))?.error?.includes(text));
});

When('I scroll down the Skills page', async function(this:ThreadbearWorld) {
  await this.app.do({cmd:'skills',scroll:300});
});
Then('the shared skill store is {word}', async function(this:ThreadbearWorld,state:string) {
  await this.until(`skill store ${state}`,d=>skills(d)?.enabled===(state==='enabled'));
});
