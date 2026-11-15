import { When, Then } from '@cucumber/cucumber';
import fs from 'node:fs';
import path from 'node:path';
import type { ThreadbearWorld } from '../support/world.ts';

When('the listed deslop skill has a file', async function (this: ThreadbearWorld) {
  const d = await this.app.dump();
  const file = path.join(d.composer.cwd!, '.claude/skills/deslop/SKILL.md');
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, '# Skill preview fixture\n\nUse this skill to tidy the code.\n');
});
Then('the composer highlights the skill {string}', async function (this: ThreadbearWorld, skill: string) {
  await this.until(`the blue skill ${skill}`, (d) => (d.composer as { skills?: string[] }).skills?.includes(skill));
});
Then('the composer highlights only the skill {string}', async function (this: ThreadbearWorld, skill: string) {
  await this.until(`only ${skill} blue`, (d) => JSON.stringify((d.composer as { skills?: string[] }).skills) === JSON.stringify([skill]));
});
When('I open the sent skill {string}', async function (this: ThreadbearWorld, skill: string) {
  const row = await this.until('my message naming the skill', (d) => [...d.transcript.rows].reverse().find((r) => r.kind === 'user' && r.text.includes(skill)));
  await this.app.do({ cmd: 'link', row: row.id, url: `threadbear-skill:${skill.slice(1)}` });
});
