// /name for one of Threadbear's skills (skills.ts) anywhere in Mike's
// message, made to work in either CLI. Claude Code runs a skill's slash
// command only as the first word of a message, so one later on gets a
// line asking the agent to use it; Codex mentions skills as $name, so a
// /name there becomes $name. The app shows the same mentions as links
// (SkillMentions.swift), and both find them the same way: a slash at the
// start or after a space or bracket, an installed skill's name, then no
// more of a word or path.
import type { Provider } from './protocol.ts';

// A plugin's skill is `plugin:name` (Claude's own way of naming them), and
// `/threadbear:name` is a store skill's other name: it means `name`, never
// the store skill called "threadbear" (Wayne, 12 Nov).
const MENTION = /(^|[\s(])\/([a-z0-9][a-z0-9-]*(?::[a-z0-9][a-z0-9-]*)?)(?![\w/:-])/gi;

// The known skill a /word names, or undefined.
function known(word: string, names: Set<string>): string | undefined {
  const w = word.toLowerCase();
  if (names.has(w)) return w;
  const rest = w.startsWith('threadbear:') ? w.slice('threadbear:'.length) : undefined;
  return rest && names.has(rest) ? rest : undefined;
}

/** The installed skills named with /name in the text, in order, once each. */
export function mentionedSkills(text: string, names: Iterable<string>): string[] {
  const set = new Set([...names].map((n) => n.toLowerCase()));
  const out: string[] = [];
  for (const m of text.matchAll(MENTION)) {
    const name = known(m[2], set);
    if (name && !out.includes(name)) out.push(name);
  }
  return out;
}

/**
 * The message as the agent gets it, with its skill mentions made to work.
 * codexName: Codex's own name for a skill, `threadbear:<name>` for a store
 * one; Codex loads a skill only for `$` and that exact name, so `$inkworks`
 * for the store's would load nothing (tried 25 Nov 2026).
 * file: a skill's SKILL.md, for an agent with no skills of its own to load
 * the store's as (Cursor, OpenCode and the other ACP ones). It gets a line
 * sending it to the file, for a /name anywhere in the message, the first
 * word too: /name is no command of its own.
 */
export function withSkillMentions(
  text: string,
  names: Iterable<string>,
  provider: Provider,
  opts: { codexName?: (name: string) => string; file?: (name: string) => string | undefined } = {},
): string {
  const skills = mentionedSkills(text, names);
  if (!skills.length) return text;
  if (provider === 'codex') {
    const set = new Set(skills);
    const own = opts.codexName ?? ((n: string) => n);
    return text.replace(MENTION, (whole, before: string, word: string) => {
      const name = known(word, set);
      return name ? `${before}$${own(name)}` : whole;
    });
  }
  if (provider !== 'claude') {
    // Without a file there is nothing to send it to: the words as typed.
    const found = skills.flatMap((s) => {
      const file = opts.file?.(s);
      return file ? [{ name: s, file }] : [];
    });
    if (!found.length) return text;
    const list = found.map((f) => `"${f.name}" (/${f.name}): ${f.file}`).join('; ');
    const one = found.length === 1;
    return `${text}\n\n(Threadbear: the user named ${one ? 'a skill' : 'skills'} from their Skills store. ${one ? 'It is a file' : 'Each is a file'} of instructions, not a command of yours: read ${one ? 'it' : 'each'} in full, with the files beside it that it points to, and follow it for this. ${list})`;
  }
  // Claude: the first word runs as a slash command already; any other needs asking for.
  const first = /^\s*\/([a-z0-9][a-z0-9-]*(?::[a-z0-9][a-z0-9-]*)?)/i.exec(text)?.[1];
  const leading = first ? known(first, new Set(skills)) : undefined;
  const asked = skills.filter((s) => s !== leading);
  if (!asked.length) return text;
  const list = asked.map((s) => `"${s}"`).join(', ');
  return `${text}\n\n(Threadbear: the user named ${asked.length === 1 ? 'the skill' : 'the skills'} ${list} with /${asked.join(', /')}; use ${asked.length === 1 ? 'it' : 'them'} for this, through the Skill tool.)`;
}
