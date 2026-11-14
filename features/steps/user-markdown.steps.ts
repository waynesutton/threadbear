import { When, Then } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import type { ThreadbearWorld } from '../support/world.ts';
type UserRow={kind:string;text:string;renderedText?:string;markdownSource?:string};
const latest=(d:{transcript:{rows:unknown[]}})=>(d.transcript.rows as UserRow[]).filter(r=>r.kind==='user').at(-1);
When('I send the formatted message',async function(this:ThreadbearWorld,text:string){
  await this.app.do({cmd:'type',text});
  await this.app.do({cmd:'return'});
  await this.until('new message',d=>latest(d)?.text.startsWith(text.split('\n')[0]));
});
Then('my rendered message contains {string}',async function(this:ThreadbearWorld,text:string){
  await this.until('rendered text',d=>latest(d)?.renderedText?.includes(text));
});
Then('my rendered message does not contain {string}',async function(this:ThreadbearWorld,text:string){
  assert.ok(!latest(await this.app.dump())?.renderedText?.includes(text));
});
Then('my message keeps a hard break after {string}',async function(this:ThreadbearWorld,text:string){
  assert.ok(latest(await this.app.dump())?.markdownSource?.includes(text+'  \n'));
});
Then('a plain line after my list starts a new paragraph',async function(this:ThreadbearWorld){
  assert.ok(latest(await this.app.dump())?.markdownSource?.includes('- two  \n\nthen this'));
});
Then('my latest message uses plain text',async function(this:ThreadbearWorld){
  assert.equal(latest(await this.app.dump())?.markdownSource,undefined);
});
