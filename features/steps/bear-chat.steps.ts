import { Then, When } from '@cucumber/cucumber';
import {okValue} from '../support/driver.ts';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import type { ThreadbearWorld } from '../support/world.ts';
type Bear={folder:string;thread?:string|null};
const chats=new WeakMap<ThreadbearWorld,string>();
const bear=(d:unknown)=>(d as {bearChat:Bear}).bearChat;
Then('the bear draft uses its own local folder',async function(this:ThreadbearWorld){
  await this.until('local bear draft',d=>!d.selection&&d.composer?.cwd===bear(d).folder&&!d.composer?.runner);
});
Then("the bear's instructions have been prepared",async function(this:ThreadbearWorld){
  const info=bear(await this.app.dump());
  assert.match(await fs.readFile(path.join(info.folder,'CLAUDE.md'),'utf8'),/Remember/);
});
Then('the bear chat is open',async function(this:ThreadbearWorld){
  const id=await this.until('bear thread',d=>bear(d).thread&&d.selection?.id===bear(d).thread?d.selection.id:null);
  chats.set(this,id);
});
Then('the same bear chat is open',async function(this:ThreadbearWorld){
  await this.until('same bear thread',d=>d.selection?.id===chats.get(this));
});

When("I open the bear's menu",async function(this:ThreadbearWorld){
  const [x,y]=okValue(await this.app.do({cmd:'label-point',label:'Chat with the bear'})).split(/\s+/).map(Number);
  await this.app.do({cmd:'click-at',x,y,right:true});
});
