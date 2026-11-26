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

// Codex loads a store skill only by its own name, threadbear:<name>.
test("Codex: a store skill becomes $ and Codex's own name for it", () => {
  const own = (n: string) => (n === 'pele' ? 'threadbear:pele' : n);
  assert.equal(withSkillMentions('draw it with /pele and /threadbear:pele, in /brand-voice', names, 'codex', { codexName: own }), 'draw it with $threadbear:pele and $threadbear:pele, in $brand-voice');
});

// Cursor, OpenCode and the other ACP agents have no skills of their own to
// load the store's as, so they're sent to the file (Wayne, 26 Nov 2026).
test('another agent is sent to the skill file, wherever /name is; with no file, the words as typed', () => {
  const file = (n: string) => (n === 'pele' ? '/store/pele/SKILL.md' : undefined);
  assert.equal(withSkillMentions('Draw the flow with /pele please', names, 'grok'), 'Draw the flow with /pele please');
  assert.equal(withSkillMentions('use /brand-voice', names, 'cursor', { file }), 'use /brand-voice');
  for (const text of ['/pele draw the flow', 'Draw the flow with /pele please']) {
    const out = withSkillMentions(text, names, 'opencode', { file });
    assert.ok(out.startsWith(`${text}\n\n(Threadbear: the user named a skill from their Skills store.`), out);
    assert.match(out, /not a command of yours: read it in full.*"pele" \(\/pele\): \/store\/pele\/SKILL\.md\)$/);
    assert.doesNotMatch(out, /Skill tool/);
  }
});

test('store skills in an ACP thread: in the picker, and /name is words for the agent, not a command', async () => {
  const { commandsPart } = await import('../../src/runner/commands.ts');
  const storeSkills = () => [
    { name: 'inkworks', description: 'Ink pages', path: '/store/inkworks/SKILL.md' },
    { name: 'handoff', description: 'store handoff', path: '/store/handoff/SKILL.md' },
    { name: 'status', description: 'store status', path: '/store/status/SKILL.md' },
  ];
  // The agent's own /handoff, from its live list.
  const commandCache = new Map([['oc\0/work\0trusted', { at: 0, commands: [{ name: 'handoff', source: 'cli' }] }]]);
  const runner = {
    ...commandsPart,
    storeSkills,
    commandCache,
    accounts: { get: (id: string) => ({ id, provider: id === 'oc' ? 'opencode' : 'claude', title: id, configDir: '/c' }) },
    adapters: {},
    clock: { now: () => 1 },
    log: { warn() {} },
    patchTrust: {},
  };
  const { commands } = await commandsPart.listCommands.call(runner as never, 'oc', '/work', 'auto');
  const skills = commands.filter((c) => c.source === 'skill');
  assert.deepEqual(skills.map((c) => [c.name, c.scope, c.path]), [
    ['inkworks', 'store', '/store/inkworks/SKILL.md'],
    ['status', 'store', '/store/status/SKILL.md'],
  ]);
  assert.equal(commands.find((c) => c.name === 'handoff')?.source, 'cli', "the agent's own /handoff stays its own");
  // Claude and Codex get the store themselves: no rows added here.
  assert.deepEqual(commandsPart.storeRows.call(runner as never, 'claude', []), []);
  assert.deepEqual(commandsPart.storeRows.call(runner as never, 'codex', []), []);

  const st = (provider: string) => ({ rec: { provider, accountId: 'oc', cwd: '/work' } });
  const plan = (provider: string, text: string) => commandsPart.commandPlan.call(runner as never, st(provider) as never, text);
  assert.equal(plan('opencode', '/inkworks a piston pump'), undefined, 'a store skill is words for the agent');
  assert.deepEqual(plan('opencode', '/handoff'), { kind: 'cli' }, "the agent's own command still runs");
  assert.deepEqual(plan('opencode', '/status'), { kind: 'status' }, "the runner's own commands win");
  assert.deepEqual(plan('claude', '/inkworks a piston pump'), { kind: 'cli' });
});

test("codexSkillName: the cached list's name, else threadbear:<name> for a store skill", async () => {
  const { commandsPart } = await import('../../src/runner/commands.ts');
  const commandCache = new Map([['acct\0/work\0trusted', { at: 0, commands: [
    { name: 'threadbear:inkworks', source: 'skill' },
    { name: 'pele', source: 'skill' },
    { name: 'x-search', source: 'skill' },
  ] }]]);
  const own = commandsPart.codexSkillName.call({ commandCache } as never, ['inkworks', 'pele', 'anatomy'], { accountId: 'acct', cwd: '/work' });
  assert.deepEqual(['inkworks', 'pele', 'anatomy', 'x-search'].map(own), ['threadbear:inkworks', 'pele', 'threadbear:anatomy', 'x-search']);
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
