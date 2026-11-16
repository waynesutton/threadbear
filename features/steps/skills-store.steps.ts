// The skills store (features/skills-store.feature): the Skills page over the
// main panel, from the bar's button or ⌘5, while the skillsStore experiment
// is on. Read from the dump's `skillsPage`; driven by the `skills` command
// and the same clicks and keys a person uses.
import { Then, When } from '@cucumber/cucumber';
import type { ThreadbearWorld } from '../support/world.ts';

type SkillsInfo = { button: boolean; open: boolean; skills: { name: string; always: boolean }[]; suggested: string[]; form?: unknown };
const skills = (d: unknown) => (d as { skillsPage?: SkillsInfo | null }).skillsPage;
const names = (d: unknown) => (skills(d)?.skills ?? []).map((s) => s.name).join(', ');

When('I press ⌘5', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'menu-key', modifiers: ['cmd'], key: '5' });
});

When('I open Skills', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'skills' });
  await this.until('Skills open', (d) => skills(d)?.open);
});

Then('Skills is open', async function (this: ThreadbearWorld) {
  await this.until('Skills open', (d) => skills(d)?.open);
});

Then('Skills is closed', async function (this: ThreadbearWorld) {
  await this.until('Skills closed', (d) => skills(d) && !skills(d)?.open);
});

When('I press Escape on Skills', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'escape' });
});

Then('the skills listed are {string}', async function (this: ThreadbearWorld, want: string) {
  await this.until(`skills listed as ${want}`, (d) => names(d) === want);
});

Then('{string} is suggested', async function (this: ThreadbearWorld, name: string) {
  await this.until(`${name} suggested`, (d) => skills(d)?.suggested.includes(name));
});

Then('nothing is suggested', async function (this: ThreadbearWorld) {
  await this.until('nothing suggested', (d) => skills(d)?.suggested.length === 0);
});

When('I write a new skill called {string} that says {string}', async function (this: ThreadbearWorld, name: string, text: string) {
  await this.app.do({ cmd: 'skills', new: true, name, text });
  await this.until('the New Skill form open', (d) => Boolean(skills(d)?.form));
  // Its label; the kit's button keeps "Save it" as a tooltip only.
  await this.app.do({ cmd: 'click', tip: 'Save' });
  await this.until('the form closed', (d) => !skills(d)?.form);
});

When('I add the suggested skill {string}', async function (this: ThreadbearWorld, name: string) {
  await this.app.do({ cmd: 'click', tip: `Add ${name} to your skills` });
});

When('I turn off Always on for {string}', async function (this: ThreadbearWorld, name: string) {
  await this.app.do({ cmd: 'skills', name, always: false });
});

Then('{string} is not Always on', async function (this: ThreadbearWorld, name: string) {
  await this.until(`${name} not Always on`, (d) => skills(d)?.skills.some((s) => s.name === name && !s.always));
});

Then('no suggestion is shown', async function (this: ThreadbearWorld) {
  await this.until('no suggestion shown', (d) => ((skills(d) as { suggestedShown?: string[] } | null | undefined)?.suggestedShown ?? ['?']).length === 0);
});

When('I search skills for {string}', async function (this: ThreadbearWorld, typed: string) {
  await this.app.do({ cmd: 'skills', search: typed });
});

Then('the skills shown are {string}', async function (this: ThreadbearWorld, want: string) {
  await this.until(`skills shown as ${want || 'none'}`, (d) => ((skills(d) as { shown?: string[] } | null | undefined)?.shown ?? []).join(', ') === want);
});

Then('Skills says {string}', async function (this: ThreadbearWorld, words: string) {
  await this.until(`Skills saying ${words}`, (d) => (skills(d) as { noMatch?: string | null } | null | undefined)?.noMatch === words);
});

Then('there is no Skills button', async function (this: ThreadbearWorld) {
  await this.until('no Skills button', (d) => skills(d) && !skills(d)?.button);
});
