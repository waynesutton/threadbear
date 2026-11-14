import { When, Then } from '@cucumber/cucumber';
import type { ThreadbearWorld } from '../support/world.ts';
const notes = (d: unknown) => (d as { notes: { open: boolean; folded: boolean; title?: string; body?: string; saved: boolean } }).notes;
When('I open my notes', async function (this: ThreadbearWorld) { await this.app.do({cmd:'click', tipPrefix:'Notes ('}); });
When('I make a new note', async function (this: ThreadbearWorld) { await this.app.do({cmd:'click', tip:'New note'}); });
When('I rename the note to {string}', async function (this: ThreadbearWorld, title: string) { await this.app.do({cmd:'notes', title}); });
When('I write {string} in the note', async function (this: ThreadbearWorld, body: string) { await this.app.do({cmd:'notes', body}); });
When('I leave my notes', async function (this: ThreadbearWorld) { await this.app.do({cmd:'click', tip:'Close Notes'}); });
When('I fold the notes list', async function (this: ThreadbearWorld) { await this.app.do({cmd:'click', tip:'Show or hide the notes list'}); });
Then('the notes list is folded', async function (this: ThreadbearWorld) { await this.until('notes list folded', d => notes(d).open && notes(d).folded); });
Then('my notes show {string} with {string}', async function (this: ThreadbearWorld, title: string, body: string) { await this.until('saved note and editor', d => notes(d).open && notes(d).title === title && notes(d).body === body && notes(d).saved); });

When('I click the Threads button from Notes', async function (this: ThreadbearWorld) { await this.app.do({cmd:'click',tipPrefix:'Threads ('}); });
Then('my notes are closed', async function (this: ThreadbearWorld) { await this.until('Notes closed', d => !notes(d).open); });

When('I write this note:', async function (this: ThreadbearWorld, body: string) { await this.app.do({cmd:'notes',body}); });
When('I click the note\'s task {string}', async function (this: ThreadbearWorld, title: string) {
  const d=await this.app.dump();
  const body=notes(d).body ?? '';
  const at=body.indexOf(`[ ] ${title}`);
  if(at<0) throw new Error(`No task ${title}`);
  await this.app.do({cmd:'notes',toggleAt:Buffer.byteLength(body.slice(0,at))+1});
});
When('I undo my note edit', async function (this: ThreadbearWorld) { await this.app.do({cmd:'notes',undo:true}); });
When('I press Return at the end of the note', async function (this: ThreadbearWorld) {
  const body=notes(await this.app.dump()).body ?? '';
  await this.app.do({cmd:'notes',returnAt:Buffer.byteLength(body)});
});
When('I go Back from Notes', async function (this: ThreadbearWorld) { await this.app.do({cmd:'menu-item',title:'Back'}); });
Then('the saved note ends with {string}', async function (this: ThreadbearWorld, suffix: string) {
  suffix=suffix.replaceAll('\\n','\n');
  await this.until(`note saved ending ${JSON.stringify(suffix)}`,d=>notes(d).saved && notes(d).body?.endsWith(suffix));
});
Then('the note\'s heading sizes decrease down to plain text', async function (this: ThreadbearWorld) {
  await this.until('four shaped lines with descending font sizes', d=>{
    const sizes=(d as unknown as {notes:{renderedFontSizes?:[number,number][]}}).notes.renderedFontSizes;
    const at=(row:number)=>sizes?.find(s=>s[0]===row)?.[1] ?? 0;
    return at(0)>at(1) && at(1)>at(2) && at(2)>at(3) && at(3)>0;
  });
});

Then('Notes has left me on {string}', async function (this: ThreadbearWorld, title:string) { await this.until('same thread after leaving Notes',d=>d.selection?.title===title); });
