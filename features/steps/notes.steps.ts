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
