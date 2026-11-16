import { When, Then } from '@cucumber/cucumber';
import type { ThreadbearWorld } from '../support/world.ts';
When('a reply contains the inline code {string}',async function(this:ThreadbearWorld,code:string){
  const d=await this.app.dump();
  await this.app.do({cmd:'mock',method:'stream',params:{threadId:d.selection?.id,text:`See \`${code}\` for the details.`,hz:60}});
  await this.until('reply finished',d=>d.transcript.rows.some(r=>r.kind==='footer'&&r.turn==='turn_stream'));
});

Then('my inline code address points to {string}',async function(this:ThreadbearWorld,url:string){
  await this.until('my rendered link',d=>d.transcript.rows.some(r=>r.kind==='user'&&(r as unknown as {markdownSource?:string}).markdownSource?.includes('](<'+url+'>)')));
});
