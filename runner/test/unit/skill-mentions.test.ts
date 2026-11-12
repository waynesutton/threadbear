// /name for a Threadbear skill made to work in either CLI.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mentionedSkills, withSkillMentions } from '../../src/skill-mentions.ts';

const names = ['pele', 'brand-voice'];

test('mentions are installed skills as words, not paths', () => {
  assert.deepEqual(mentionedSkills('/pele draw it, then (/brand-voice) and /pele again', names), ['pele', 'brand-voice']);
  assert.deepEqual(mentionedSkills('open src/pele.ts and ~/pele and /peler and /unknown', names), []);
});

test('Claude: a leading /name runs as it is; one later on is asked for', () => {
  assert.equal(withSkillMentions('/pele draw the checkout flow', names, 'claude'), '/pele draw the checkout flow');
  assert.match(withSkillMentions('Draw the flow with /pele please', names, 'claude'), /^Draw the flow with \/pele please\n\n\(Threadbear: the user named the skill "pele" with \/pele; use it for this, through the Skill tool\.\)$/);
  assert.match(withSkillMentions('/pele and /brand-voice', names, 'claude'), /named the skill "brand-voice" with \/brand-voice/);
  assert.equal(withSkillMentions('no skills here', names, 'claude'), 'no skills here');
});

test('Codex: /name becomes $name', () => {
  assert.equal(withSkillMentions('/pele draw it in src/pele.ts with /Brand-Voice', names, 'codex'), '$pele draw it in src/pele.ts with $brand-voice');
});

import { makeHarness, tick, tmpDir } from '../helpers/fakes.ts';

test('the runner gives the agent the message with its skill mentions made to work', async () => {
  let names = ['pele'];
  const h = makeHarness(tmpDir(), {}, { skillNames: () => names });
  const { thread } = h.runner.create({ text: 'Draw the flow with /pele', cwd: tmpDir(), accountId: 'claude-3' });
  await tick();
  assert.match(h.adapter.last.sent.at(-1)!.text, /^Draw the flow with \/pele\n\n\(Threadbear: the user named the skill "pele"/);
  // What Mike sees is what he typed.
  assert.equal(h.events(thread.id).find((e) => e.type === 'user.message')?.text, 'Draw the flow with /pele');
  // The store off: nothing added.
  h.adapter.last.init();
  h.adapter.last.result('completed');
  await tick();
  names = [];
  h.runner.send(thread.id, 'Again with /pele');
  await tick();
  assert.equal(h.adapter.last.sent.at(-1)!.text, 'Again with /pele');
});

// Skills the CLI finds itself (~/.claude/skills) count as mentions too, from
// the command list cached for the thread's login and folder (Wayne, 11 Nov).
test('listedSkills: the cached list for that login and folder, skills only, not first-word-only ones', async () => {
  const { commandsPart } = await import('../../src/runner/commands.ts');
  const commandCache = new Map([
    ['acct\0/work\0trusted', { at: 0, commands: [
      { name: 'make-print', source: 'skill' },
      { name: 'builder-sec-check', source: 'skill', path: '/x/SKILL.md' },
      { name: 'deploy', source: 'skill', userOnly: true },
      { name: 'compact', source: 'cli' },
    ] }],
    ['acct\0/elsewhere\0trusted', { at: 0, commands: [{ name: 'other', source: 'skill' }] }],
    ['other\0/work\0trusted', { at: 0, commands: [{ name: 'theirs', source: 'skill' }] }],
  ]);
  const names = commandsPart.listedSkills.call({ commandCache } as never, 'acct', '/work');
  assert.deepEqual(names.sort(), ['builder-sec-check', 'make-print']);
  assert.equal(
    withSkillMentions('/make-print then /builder-sec-check', names, 'claude'),
    '/make-print then /builder-sec-check\n\n(Threadbear: the user named the skill "builder-sec-check" with /builder-sec-check; use it for this, through the Skill tool.)',
  );
});

// Claude names a store skill `threadbear:name`; that's the skill `name`,
// never the store skill called "threadbear" (Wayne, 12 Nov).
test('/threadbear:name is the store skill name; plugin:name mentions work', () => {
  const known = ['threadbear', 'pele', 'convex:add'];
  assert.deepEqual(mentionedSkills('use /threadbear:pele and /convex:add', known), ['pele', 'convex:add']);
  assert.deepEqual(mentionedSkills('/threadbear:nope here', known), []);
  assert.equal(withSkillMentions('draw it with /threadbear:pele', known, 'codex'), 'draw it with $pele');
  assert.equal(withSkillMentions('/threadbear:pele draw it', known, 'claude'), '/threadbear:pele draw it');
});
