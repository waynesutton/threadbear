#!/usr/bin/env node
// the progress print for a long task: one state file, rebuilt into a page
// in under a second after every step, so the thread keeps working.
//
//   node progress.mjs <state.json> init "<title>" "<the task in a line>"
//   node progress.mjs <state.json> step "<name>" todo|doing|done|stuck|skipped ["<note>"]
//   node progress.mjs <state.json> ask "<question>" "<what you'll do if there's no answer>"
//   node progress.mjs <state.json> answer "<question, or its start>" "<the answer>"
//   node progress.mjs <state.json> stuck "<what>" "<why>"      (unstuck "<what>" clears it)
//   node progress.mjs <state.json> deliver "<what>" "<path or link>"
//   node progress.mjs <state.json> note "<one line on where it stands>"
//   node progress.mjs <state.json> build <out dir>
//
// Every change also rebuilds <out dir> when the state has one (`init`
// remembers the folder next to the state as page/). Times are the real
// clock's, in ISO, so the page can say how long ago each thing happened.
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { buildPage } from "./build.mjs";

const now = () => new Date().toISOString();
export const STATUSES = ["todo", "doing", "done", "stuck", "skipped"];

export function load(file) {
  return existsSync(file) ? JSON.parse(readFileSync(file, "utf8")) : null;
}

export function apply(state, cmd, args, at = now()) {
  const s = state ?? {};
  const find = (list, key, text) => list.find((x) => x[key] === text) ?? list.find((x) => x[key].toLowerCase().startsWith(String(text).toLowerCase()));
  switch (cmd) {
    case "init": {
      const [title, task] = args;
      if (!title) throw new Error('init needs a title: init "<title>" "<task>"');
      return { kind: "progress", title, task: task ?? "", startedAt: at, updatedAt: at, steps: [], questions: [], stuck: [], deliverables: [], notes: [] };
    }
    case "step": {
      const [name, status = "todo", note] = args;
      if (!STATUSES.includes(status)) throw new Error(`status is one of ${STATUSES.join(", ")}`);
      const existing = find(s.steps, "title", name);
      if (existing) Object.assign(existing, { status, at, ...(note !== undefined ? { note } : {}) });
      else s.steps.push({ title: name, status, at, ...(note ? { note } : {}) });
      if (status === "doing") for (const x of s.steps) if (x !== (existing ?? s.steps.at(-1)) && x.status === "doing") x.status = "todo";
      break;
    }
    case "ask": {
      const [question, fallback] = args;
      if (!question || !fallback) throw new Error('ask needs the question and the default: ask "<question>" "<default>"');
      s.questions.push({ question, default: fallback, asked: at });
      break;
    }
    case "answer": {
      const [question, answer] = args;
      const q = find(s.questions, "question", question);
      if (!q) throw new Error(`No question starting "${question}".`);
      Object.assign(q, { answer: answer ?? "", answered: at });
      break;
    }
    case "stuck": {
      const [what, why] = args;
      s.stuck.push({ what, why: why ?? "", since: at });
      break;
    }
    case "unstuck": {
      const [what] = args;
      s.stuck = s.stuck.filter((x) => !(x.what === what || x.what.toLowerCase().startsWith(String(what).toLowerCase())));
      break;
    }
    case "deliver": {
      const [what, where] = args;
      s.deliverables.unshift({ what, where: where ?? "", at });
      break;
    }
    case "note": {
      s.notes.unshift({ text: args[0], at });
      s.notes = s.notes.slice(0, 5);
      break;
    }
    default:
      throw new Error(`Unknown command ${cmd}.`);
  }
  s.updatedAt = at;
  return s;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const [file, cmd, ...args] = process.argv.slice(2);
  if (!file || !cmd) {
    console.error("usage: node progress.mjs <state.json> init|step|ask|answer|stuck|unstuck|deliver|note|build ...");
    process.exit(2);
  }
  try {
    const path = resolve(file);
    let state = load(path);
    let out = state?.out;
    if (cmd === "build") out = resolve(args[0] ?? out ?? join(dirname(path), "page"));
    else {
      if (cmd !== "init" && !state) throw new Error(`No progress file at ${path}. Start with init.`);
      state = apply(state, cmd, args);
      if (cmd === "init") state.out = out = join(dirname(path), "page");
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, JSON.stringify(state, null, 2) + "\n");
    }
    if (out) {
      const { html } = buildPage({ ...state, kind: "progress" }, dirname(path));
      mkdirSync(out, { recursive: true });
      writeFileSync(join(out, "index.html"), html);
      const done = state.steps.filter((x) => x.status === "done").length;
      const open = state.questions.filter((q) => q.answer === undefined).length;
      console.log(`${out}  ${done}/${state.steps.length} done, ${open} waiting on you, ${state.stuck.length} stuck`);
    }
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
}
