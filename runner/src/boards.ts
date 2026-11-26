// Plan boards (experiment planBoard, experiments/planBoard.md): one thread's
// plan as a small Mermaid flowchart the agent writes with update_board, kept
// beside the runner's other data so Wayne can agree the shape of the work
// before anything is built, approve it, then send its boxes out one at a time.
//
// This file is the only Mermaid parser for boards. The `board.updated` event
// carries the parsed graph, so the desktop never parses Mermaid (the
// reviewCanvas split: rules in core, drawing in gpui, files here).
//
// On disk, under <dataDir>/boards/<threadId>/:
//   v<n>.json   one per version the agent wrote (mermaid, note, changed),
//               the last MAX_VERSIONS kept
//   board.json  what's true now: the approved version, the boxes Wayne
//               dragged, the board's skills, the threads (or coordinator
//               to-dos) each box was sent to, and each Start's request id
//
// Plan state is derived, never stored: a draft unless the approved version is
// the latest. Status isn't stored either: the app reads it off the linked
// thread's summary, the same one the sidebar and notifications read.
//
// Self-contained on purpose: no Prints (pages.ts), so it builds on Mike's
// upstream as well as Wayne's fork.
import fs from 'node:fs';
import path from 'node:path';
import { atomicWriteFileSync, readJsonSync } from './fsutil.ts';
import { redactSecrets } from './redaction.ts';

export const MAX_NODES = 25;
export const MAX_EDGES = 40;
export const MAX_LABEL = 60;
export const MAX_NOTE = 400;
export const MAX_MERMAID = 8000;
export const MAX_VERSIONS = 10;
export const MAX_SKILLS = 12;
// Ids an agent can send that are also safe as file names and link parts.
const ID = /^[A-Za-z_][A-Za-z0-9_-]{0,47}$/;
const THREAD_ID = /^[A-Za-z0-9-]{8,64}$/;

export type BoardShape = 'box' | 'db' | 'round';
export type BoardNode = { id: string; label: string; shape: BoardShape; group?: string };
export type BoardGroup = { id: string; label: string; parent?: string };
export type BoardEdge = { from: string; to: string; label?: string; line?: true };
export type BoardDirection = 'LR' | 'TB';
export type BoardGraph = {
  direction: BoardDirection;
  nodes: BoardNode[];
  groups: BoardGroup[];
  edges: BoardEdge[];
  // `%% note:` lines, in order.
  notes: string[];
  // Lines kept in the Mermaid but not drawn (styles, clicks, shapes the
  // subset doesn't have): the board says "N lines not drawn".
  skipped: number;
};

export type BoardNote = { text: string; near?: string };

export type BoardVersion = {
  threadId: string;
  version: number;
  savedAt: string;
  by: 'agent';
  mermaid: string;
  note?: BoardNote;
  changed: string[];
};

// Where a box went. A nested thread (Phase 1), or with planBoardCoordinators
// a to-do on the project's coordinator (Phase 3). `history` holds earlier
// ones, from Send again.
export type BoardThreadLink = { kind?: 'thread'; threadId: string; harness: string; planVersion: number; at: string };
export type BoardCoordinatorLink = { kind: 'coordinator'; projectId: string; taskId: string; threadId?: string; planVersion: number; at: string };
export type BoardLinkTarget = BoardThreadLink | BoardCoordinatorLink;
export type BoardLink = BoardLinkTarget & { history: BoardLinkTarget[] };

export type BoardFile = {
  approved: { version: number; at: string } | null;
  // Only boxes Wayne dragged, in board units; Tidy clears them.
  positions: Record<string, [number, number]>;
  // Names only. Nothing is copied, installed or run.
  skills: string[];
  links: Record<string, BoardLink>;
  // requestId → the box and what Start made, so a second Start (or a slow
  // runner answering twice) returns the first one.
  sends: Record<string, { node: string; ref: string }>;
};

export type PlanState = 'draft' | 'approved';

// What the app and read_board see: the latest version, parsed, with the
// board's state. One shape for the event, board.get and the RPC results.
export type PlanBoard = {
  threadId: string;
  version: number;
  savedAt: string;
  mermaid: string;
  graph: BoardGraph;
  note?: BoardNote;
  changed: string[];
  state: PlanState;
  approved: { version: number; at: string } | null;
  positions: Record<string, [number, number]>;
  skills: string[];
  links: Record<string, BoardLink>;
  versions: { version: number; savedAt: string; note?: string }[];
};

export class BoardError extends Error {}

const emptyFile = (): BoardFile => ({ approved: null, positions: {}, skills: [], links: {}, sends: {} });

// --- The Mermaid subset -------------------------------------------------
//
// flowchart LR|TB (TD is TB; graph works too), id[label], id[(label)],
// id((label)), id(label), subgraph id[label] … end, a -->|label| b,
// a -- label --> b, a --- b, chains a --> b --> c, and `%% note:` lines.
// Anything else is kept in the Mermaid but not drawn.

type Ref = { id: string; label?: string; shape?: BoardShape };

const cleanLabel = (raw: string): string => {
  let s = raw.trim();
  if (s.length >= 2 && s.startsWith('"') && s.endsWith('"')) s = s.slice(1, -1);
  // Mermaid's <br> and entities read as plain words on the board.
  return s.replace(/<br\s*\/?>/gi, ' ').replace(/&quot;/g, '"').replace(/&amp;/g, '&').replace(/\s+/g, ' ').trim();
};

// One node reference: `id`, `id[label]`, `id[(label)]`, `id((label))`, `id(label)`.
function parseRef(raw: string, line: number): Ref {
  const s = raw.trim();
  const m = s.match(/^([A-Za-z_][A-Za-z0-9_-]*)\s*(.*)$/);
  if (!m) throw new BoardError(`Line ${line}: "${s}" isn't a node (an id like kpi, then [label]).`);
  const [, id, rest] = m;
  if (!ID.test(id)) throw new BoardError(`Line ${line}: node id "${id}" is too long (48 characters at most).`);
  if (!rest) return { id };
  const shapes: [RegExp, BoardShape][] = [
    [/^\[\((.*)\)\]$/, 'db'],
    [/^\(\((.*)\)\)$/, 'round'],
    [/^\[(.*)\]$/, 'box'],
    [/^\((.*)\)$/, 'round'],
  ];
  for (const [re, shape] of shapes) {
    const hit = rest.match(re);
    if (hit) return { id, label: cleanLabel(hit[1]), shape };
  }
  throw new BoardError(`Line ${line}: "${s}" uses a node shape the board doesn't draw. Use id[label], id[(label)] or id((label)).`);
}

// Splits `a -->|x| b -- y --> c` into refs and the links between them.
const LINK = /\s*(-->|---|-\.->|==>)\s*(?:\|([^|]*)\|)?\s*|\s+--\s+([^-][^>]*?)\s+-->\s*/g;

function parseEdges(line: string, n: number): { refs: Ref[]; links: { label?: string; line?: true }[] } | undefined {
  const refs: Ref[] = [];
  const links: { label?: string; line?: true }[] = [];
  let last = 0;
  let depth = 0;
  // Walk the line, splitting on links outside brackets (a label may hold "-->").
  const parts: { text: string; link?: { label?: string; line?: true } }[] = [];
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (c === '[' || c === '(' || c === '{') depth++;
    else if (c === ']' || c === ')' || c === '}') depth = Math.max(0, depth - 1);
    if (depth > 0) continue;
    LINK.lastIndex = i;
    const m = LINK.exec(line);
    if (m && m.index === i) {
      parts.push({ text: line.slice(last, i) });
      const op = m[1];
      const label = m[2] ?? m[3];
      parts.push({ text: '', link: { ...(label?.trim() ? { label: cleanLabel(label) } : {}), ...(op === '---' ? { line: true as const } : {}) } });
      i = m.index + m[0].length - 1;
      last = i + 1;
    }
  }
  if (!parts.some((p) => p.link)) return undefined;
  parts.push({ text: line.slice(last) });
  for (const p of parts) {
    if (p.link) links.push(p.link);
    else {
      // `a & b --> c` fans out in Mermaid; the board keeps it simple.
      if (p.text.includes('&')) throw new BoardError(`Line ${n}: write one link per line rather than "&".`);
      refs.push(parseRef(p.text, n));
    }
  }
  if (refs.length !== links.length + 1) throw new BoardError(`Line ${n}: a link needs a node on each side.`);
  return { refs, links };
}

/** Parses the board's Mermaid subset. Throws BoardError with a plain reason. */
export function parseBoard(mermaid: string): BoardGraph {
  if (typeof mermaid !== 'string' || !mermaid.trim()) throw new BoardError('The plan is empty. Send a Mermaid flowchart, starting "flowchart LR".');
  if (mermaid.length > MAX_MERMAID) throw new BoardError(`The plan is ${mermaid.length} characters; the board takes ${MAX_MERMAID} at most. Make it smaller.`);
  const lines = mermaid.replace(/\r\n?/g, '\n').split('\n');
  let direction: BoardDirection | undefined;
  const nodes = new Map<string, BoardNode>();
  const groups = new Map<string, BoardGroup>();
  const edges: BoardEdge[] = [];
  const notes: string[] = [];
  const stack: string[] = [];
  let skipped = 0;
  let anon = 0;

  const touch = (ref: Ref, n: number) => {
    if (ref.label !== undefined && ref.label.length > MAX_LABEL) throw new BoardError(`Line ${n}: the label of "${ref.id}" is ${ref.label.length} characters; keep labels to ${MAX_LABEL}.`);
    if (groups.has(ref.id)) throw new BoardError(`Line ${n}: "${ref.id}" is a group's id; give the box another one.`);
    const group = stack[stack.length - 1];
    const had = nodes.get(ref.id);
    if (!had) {
      nodes.set(ref.id, { id: ref.id, label: ref.label || ref.id, shape: ref.shape ?? 'box', ...(group ? { group } : {}) });
      if (nodes.size > MAX_NODES) throw new BoardError(`The plan has more than ${MAX_NODES} boxes. Keep it to ${MAX_NODES}: group the detail into fewer boxes.`);
      return;
    }
    if (ref.label !== undefined) {
      had.label = ref.label || ref.id;
      had.shape = ref.shape ?? had.shape;
    }
    if (!had.group && group) had.group = group;
  };

  lines.forEach((raw, i) => {
    const n = i + 1;
    const line = raw.trim().replace(/;+$/, '').trim();
    if (!line) return;
    if (line.startsWith('%%')) {
      const note = line.match(/^%%\s*note:\s*(.*)$/i);
      if (note && note[1].trim()) notes.push(cutTo(note[1].trim(), MAX_NOTE));
      return;
    }
    if (!direction) {
      const head = line.match(/^(?:flowchart|graph)\s+(LR|TB|TD|RL|BT)\s*$/i);
      if (!head) throw new BoardError(`Line ${n}: a board starts "flowchart LR" or "flowchart TB".`);
      const d = head[1].toUpperCase();
      direction = d === 'LR' || d === 'RL' ? 'LR' : 'TB';
      if (d === 'RL' || d === 'BT') skipped++;
      return;
    }
    const sub = line.match(/^subgraph\s+(.+)$/);
    if (sub) {
      const body = sub[1].trim();
      let id: string;
      let label: string;
      const withLabel = body.match(/^([A-Za-z_][A-Za-z0-9_-]*)\s*\[(.*)\]$/);
      if (withLabel) {
        id = withLabel[1];
        label = cleanLabel(withLabel[2]);
      } else if (ID.test(body)) {
        id = body;
        label = body;
      } else {
        // `subgraph "Server · Node"` or bare words: a made-up id.
        id = `group${++anon}`;
        label = cleanLabel(body);
      }
      if (!ID.test(id)) throw new BoardError(`Line ${n}: group id "${id}" is too long.`);
      if (label.length > MAX_LABEL) throw new BoardError(`Line ${n}: the group label is ${label.length} characters; keep labels to ${MAX_LABEL}.`);
      if (nodes.has(id) || groups.has(id)) throw new BoardError(`Line ${n}: "${id}" is used twice.`);
      groups.set(id, { id, label: label || id, ...(stack.length ? { parent: stack[stack.length - 1] } : {}) });
      stack.push(id);
      return;
    }
    if (line === 'end') {
      if (!stack.length) throw new BoardError(`Line ${n}: "end" without a subgraph.`);
      stack.pop();
      return;
    }
    if (/^(direction\s|classDef\s|class\s|style\s|linkStyle\s|click\s)/.test(line)) {
      skipped++;
      return;
    }
    const chain = parseEdges(line, n);
    if (chain) {
      chain.refs.forEach((r) => touch(r, n));
      chain.links.forEach((l, k) => {
        edges.push({ from: chain.refs[k].id, to: chain.refs[k + 1].id, ...l });
      });
      if (edges.length > MAX_EDGES) throw new BoardError(`The plan has more than ${MAX_EDGES} arrows. Keep it to ${MAX_EDGES}.`);
      for (const l of chain.links) if (l.label && l.label.length > MAX_LABEL) throw new BoardError(`Line ${n}: an arrow's label is ${l.label.length} characters; keep labels to ${MAX_LABEL}.`);
      return;
    }
    try {
      touch(parseRef(line, n), n);
    } catch (e) {
      // A shape or statement outside the subset: kept, not drawn.
      if (e instanceof BoardError && /node shape/.test(e.message)) {
        skipped++;
        return;
      }
      throw e;
    }
  });
  if (!direction) throw new BoardError('A board starts "flowchart LR" or "flowchart TB".');
  if (stack.length) throw new BoardError(`The subgraph "${stack[stack.length - 1]}" has no "end".`);
  if (!nodes.size) throw new BoardError('The plan has no boxes. Add some, like kpi[KPI cards].');
  return { direction, nodes: [...nodes.values()], groups: [...groups.values()], edges, notes, skipped };
}

function cutTo(s: string, max: number): string {
  return s.length <= max ? s : `${s.slice(0, max - 1).trimEnd()}…`;
}

// --- The brief a box's thread gets --------------------------------------

export type BriefSkill = { name: string; userOnly?: boolean };

/**
 * The fixed header the runner writes first, then the skills line (if any),
 * the box, its neighbours and the planner's note, and a closing line. The app
 * shows the same text, editable, before Start (desktop/core/src/plan_board.rs
 * builds the same thing for the sheet; this is the runner's copy for a send
 * that comes without text).
 */
export function briefHeader(threadId: string, node: BoardNode, version: number): string {
  return `Plan board: thread ${threadId.slice(0, 8)}, box ${node.id} ("${node.label}"), plan v${version} (approved).`;
}

export function skillsLine(skills: readonly string[], provider: string): string {
  if (!skills.length) return '';
  // Claude runs a skill named mid-message when it reads /name (docs/guide.md,
  // "Slash commands"). Codex mentions skills as $name, its own form, which
  // works for any skill in $CODEX_HOME/skills on either tree; the names come
  // from Codex's own skills/list, so a store skill is already threadbear:name.
  const mark = provider === 'codex' ? '$' : '/';
  return `Use these skills: ${skills.map((s) => `${mark}${s}`).join(', ')}`;
}

export function buildBrief(board: PlanBoard, nodeId: string, provider: string, skills: readonly string[] = board.skills): string {
  const node = board.graph.nodes.find((x) => x.id === nodeId);
  if (!node) throw new BoardError(`The plan has no box "${nodeId}".`);
  const label = (id: string) => board.graph.nodes.find((x) => x.id === id)?.label ?? id;
  const into = board.graph.edges.filter((e) => e.to === nodeId).map((e) => `${label(e.from)}${e.label ? ` (${e.label})` : ''} → this`);
  const out = board.graph.edges.filter((e) => e.from === nodeId).map((e) => `this → ${label(e.to)}${e.label ? ` (${e.label})` : ''}`);
  const group = node.group ? board.graph.groups.find((g) => g.id === node.group)?.label : undefined;
  const lines = [briefHeader(board.threadId, node, board.approved?.version ?? board.version)];
  const sk = skillsLine(skills, provider);
  if (sk) lines.push(sk);
  lines.push('');
  lines.push(`You're building "${node.label}" from that plan${group ? `, in the group "${group}"` : ''}.`);
  const neighbours = [...into, ...out];
  if (neighbours.length) lines.push(`Neighbours: ${neighbours.join('; ')}.`);
  const note = board.note?.text ?? board.graph.notes[0];
  if (note) lines.push(`Planner's note: ${note}`);
  lines.push('');
  lines.push("When you finish, say what you did. If you need Wayne, ask him; your box shows it.");
  return lines.join('\n');
}

// --- The store ----------------------------------------------------------

export type BoardUpdate = { mermaid: string; note?: string | BoardNote; near?: string; changed?: string[]; baseVersion?: number };

export class Boards {
  readonly dir: string;
  private readonly now: () => number;
  constructor(dir: string, now: () => number = Date.now) {
    this.dir = dir;
    this.now = now;
  }

  private threadDir(threadId: string): string {
    if (!THREAD_ID.test(threadId)) throw new BoardError(`"${threadId}" isn't a thread id.`);
    return path.join(this.dir, threadId);
  }

  private iso(): string {
    return new Date(this.now()).toISOString();
  }

  private versionNumbers(threadId: string): number[] {
    const dir = this.threadDir(threadId);
    if (!fs.existsSync(dir)) return [];
    return fs.readdirSync(dir)
      .map((f) => f.match(/^v(\d+)\.json$/)?.[1])
      .filter((x): x is string => Boolean(x))
      .map(Number)
      .sort((a, b) => a - b);
  }

  private readVersion(threadId: string, v: number): BoardVersion | undefined {
    return readJsonSync<BoardVersion>(path.join(this.threadDir(threadId), `v${v}.json`));
  }

  private readFile(threadId: string): BoardFile {
    const f = readJsonSync<Partial<BoardFile>>(path.join(this.threadDir(threadId), 'board.json'));
    const base = emptyFile();
    if (!f || typeof f !== 'object') return base;
    return {
      approved: f.approved && typeof f.approved.version === 'number' ? f.approved : null,
      positions: f.positions && typeof f.positions === 'object' ? f.positions : {},
      skills: Array.isArray(f.skills) ? f.skills.filter((s) => typeof s === 'string') : [],
      links: f.links && typeof f.links === 'object' ? f.links : {},
      sends: f.sends && typeof f.sends === 'object' ? f.sends : {},
    };
  }

  private writeFile(threadId: string, file: BoardFile): void {
    const dir = this.threadDir(threadId);
    fs.mkdirSync(dir, { recursive: true });
    atomicWriteFileSync(path.join(dir, 'board.json'), `${JSON.stringify(file, null, 2)}\n`);
  }

  has(threadId: string): boolean {
    try {
      return this.versionNumbers(threadId).length > 0;
    } catch {
      return false;
    }
  }

  /** The board as the app and read_board see it, or undefined with none. */
  get(threadId: string): PlanBoard | undefined {
    const nums = this.versionNumbers(threadId);
    if (!nums.length) return undefined;
    const latest = this.readVersion(threadId, nums[nums.length - 1]);
    if (!latest) return undefined;
    let graph: BoardGraph;
    try {
      graph = parseBoard(latest.mermaid);
    } catch {
      // Written by an older parser that took more: shown empty rather than not at all.
      graph = { direction: 'LR', nodes: [], groups: [], edges: [], notes: [], skipped: 0 };
    }
    const file = this.readFile(threadId);
    const versions = nums.map((v) => {
      const x = v === latest.version ? latest : this.readVersion(threadId, v);
      return { version: v, savedAt: x?.savedAt ?? '', ...(x?.note?.text ? { note: x.note.text } : {}) };
    });
    return {
      threadId,
      version: latest.version,
      savedAt: latest.savedAt,
      mermaid: latest.mermaid,
      graph,
      ...(latest.note ? { note: latest.note } : {}),
      changed: latest.changed ?? [],
      state: file.approved?.version === latest.version ? 'approved' : 'draft',
      approved: file.approved,
      positions: file.positions,
      skills: file.skills,
      links: file.links,
      versions,
    };
  }

  /** One old version's Mermaid and note, read-only (the version menu). */
  version(threadId: string, v: number): BoardVersion | undefined {
    return this.readVersion(threadId, v);
  }

  /**
   * A new version from the agent. Refuses Mermaid that doesn't parse (the old
   * version stays) and a stale baseVersion. Says whether it took an approved
   * plan back to draft.
   */
  update(threadId: string, u: BoardUpdate): { board: PlanBoard; backToDraft: boolean; redacted: boolean } {
    const nums = this.versionNumbers(threadId);
    const current = nums.length ? nums[nums.length - 1] : 0;
    if (u.baseVersion !== undefined && u.baseVersion !== current) {
      throw new BoardError(`The board changed (it's at v${current}, you started from v${u.baseVersion}). Call read_board again, then write your change on top.`);
    }
    // Plan text only: anything that looks like a key is masked before it's kept.
    const masked = redactSecrets(u.mermaid);
    const graph = parseBoard(masked.text);
    const ids = new Set([...graph.nodes.map((x) => x.id), ...graph.groups.map((g) => g.id)]);
    const rawNote = typeof u.note === 'string' ? { text: u.note, ...(u.near ? { near: u.near } : {}) } : u.note;
    const noteText = rawNote?.text ? redactSecrets(cutTo(rawNote.text.trim(), MAX_NOTE)).text : '';
    const note: BoardNote | undefined = noteText ? { text: noteText, ...(rawNote?.near && ids.has(rawNote.near) ? { near: rawNote.near } : {}) } : undefined;
    const changed = (u.changed ?? []).filter((c) => typeof c === 'string' && ids.has(c)).slice(0, MAX_NODES);
    const file = this.readFile(threadId);
    const wasApproved = file.approved !== null && file.approved.version === current;
    const version = current + 1;
    const record: BoardVersion = { threadId, version, savedAt: this.iso(), by: 'agent', mermaid: masked.text, ...(note ? { note } : {}), changed };
    const dir = this.threadDir(threadId);
    fs.mkdirSync(dir, { recursive: true });
    atomicWriteFileSync(path.join(dir, `v${version}.json`), `${JSON.stringify(record, null, 2)}\n`);
    if (!fs.existsSync(path.join(dir, 'board.json'))) this.writeFile(threadId, file);
    // The last MAX_VERSIONS only. The approved one goes too once it's that old:
    // a newer one is the latest by then, so the plan is a draft either way.
    for (const old of [...nums, version].slice(0, -MAX_VERSIONS)) fs.rmSync(path.join(dir, `v${old}.json`), { force: true });
    return { board: this.get(threadId)!, backToDraft: wasApproved, redacted: masked.redacted };
  }

  /** Wayne's Approve plan, for the version he saw. Never an agent's. */
  approve(threadId: string, version: number): PlanBoard {
    const board = this.get(threadId);
    if (!board) throw new BoardError("This thread has no plan.");
    if (version !== board.version) throw new BoardError(`That was v${version}; the plan is at v${board.version} now. Look at the new version, then approve it.`);
    const file = this.readFile(threadId);
    file.approved = { version, at: this.iso() };
    this.writeFile(threadId, file);
    return this.get(threadId)!;
  }

  /** Wayne's drags: a position per box, null to forget one; tidy clears them all. */
  move(threadId: string, positions: Record<string, [number, number] | null>, tidy = false): PlanBoard {
    const board = this.get(threadId);
    if (!board) throw new BoardError("This thread has no plan.");
    const file = this.readFile(threadId);
    const next: Record<string, [number, number]> = tidy ? {} : { ...file.positions };
    for (const [id, at] of Object.entries(positions)) {
      if (!ID.test(id)) continue;
      if (at === null) delete next[id];
      else if (Array.isArray(at) && at.length === 2 && at.every((x) => Number.isFinite(x) && Math.abs(x) < 1e6)) next[id] = [Math.round(at[0]), Math.round(at[1])];
    }
    file.positions = next;
    this.writeFile(threadId, file);
    return this.get(threadId)!;
  }

  /** The board's skills, by name only. */
  setSkills(threadId: string, skills: string[]): PlanBoard {
    if (!this.has(threadId)) throw new BoardError("This thread has no plan.");
    const file = this.readFile(threadId);
    file.skills = [...new Set(skills.map((s) => String(s).trim().replace(/^[/$]/, '')).filter((s) => /^[A-Za-z0-9:_.-]{1,64}$/.test(s)))].slice(0, MAX_SKILLS);
    this.writeFile(threadId, file);
    return this.get(threadId)!;
  }

  /** What an earlier Start with this request id made, if any. */
  sent(threadId: string, requestId: string): { node: string; ref: string } | undefined {
    return this.readFile(threadId).sends[requestId];
  }

  /**
   * Refuses a send unless the plan is approved at its latest version and the
   * box is in it. Returns the board for the brief.
   */
  mustSend(threadId: string, nodeId: string): PlanBoard {
    const board = this.get(threadId);
    if (!board) throw new BoardError("This thread has no plan.");
    if (board.state !== 'approved') throw new BoardError(board.approved ? `The plan changed since Wayne approved it (v${board.approved.version} → v${board.version}). Ask Wayne to press Approve plan again.` : "The plan isn't approved yet. Ask Wayne to press Approve plan.");
    if (!board.graph.nodes.some((x) => x.id === nodeId)) throw new BoardError(`The plan has no box "${nodeId}".`);
    return board;
  }

  /** Records where a box went, keeping what it went to before in history. */
  link(threadId: string, nodeId: string, target: BoardLinkTarget, requestId?: string): PlanBoard {
    const file = this.readFile(threadId);
    const was = file.links[nodeId];
    const history = was ? [...was.history, strip(was)].slice(-10) : [];
    file.links[nodeId] = { ...target, history };
    if (requestId) {
      file.sends[requestId] = { node: nodeId, ref: target.kind === 'coordinator' ? target.taskId : target.threadId };
      // Old request ids only matter for a retry in flight: keep the last 50.
      const keys = Object.keys(file.sends);
      for (const k of keys.slice(0, -50)) delete file.sends[k];
    }
    this.writeFile(threadId, file);
    return this.get(threadId)!;
  }
}

function strip(link: BoardLink): BoardLinkTarget {
  const { history: _h, ...rest } = link;
  return rest;
}

/** read_board's text: plain and short, for the agent. */
export function boardText(board: PlanBoard, statusOf: (threadId: string) => string | undefined = () => undefined): string {
  const lines: string[] = [];
  lines.push(`Plan v${board.version}, ${board.state === 'approved' ? `approved` : board.approved ? `draft (changed since Wayne approved v${board.approved.version})` : 'draft (not approved yet)'}.`);
  if (board.skills.length) lines.push(`Board skills: ${board.skills.map((s) => `/${s}`).join(', ')}`);
  lines.push('Boxes:');
  for (const n of board.graph.nodes) {
    const link = board.links[n.id];
    const where = link ? (link.kind === 'coordinator' ? ` → coordinator to-do ${link.taskId}` : ` → thread ${link.threadId.slice(0, 8)} (${link.harness}${statusOf(link.threadId) ? `, ${statusOf(link.threadId)}` : ''})`) : '';
    lines.push(`- ${n.id}: ${n.label}${n.group ? ` [${n.group}]` : ''}${where}`);
  }
  for (const [id, link] of Object.entries(board.links)) {
    if (!board.graph.nodes.some((n) => n.id === id)) lines.push(`- ${id} (no longer in the plan) → ${link.kind === 'coordinator' ? `to-do ${link.taskId}` : `thread ${link.threadId.slice(0, 8)}`}`);
  }
  if (board.graph.skipped) lines.push(`${board.graph.skipped} line${board.graph.skipped === 1 ? '' : 's'} not drawn.`);
  lines.push('', 'Mermaid:', board.mermaid);
  return lines.join('\n');
}

// What an agent's threadbear tools get while planBoard is on (threadbear-mcp.ts
// read_board, update_board, create_thread's board_node). main.ts makes it, so
// write() also logs the board.updated event the app draws.
export type BoardTools = {
  read(threadId: string): PlanBoard | undefined;
  write(threadId: string, u: BoardUpdate): { board: PlanBoard; backToDraft: boolean; redacted: boolean };
  mustSend(threadId: string, nodeId: string): PlanBoard;
  linkThread(threadId: string, nodeId: string, child: { id: string; provider: string }, planVersion: number): void;
  // A linked thread's state in a word, for read_board (working, needs you, done…).
  statusOf(threadId: string): string | undefined;
};

/** Puts the runner's fixed header first, unless the text already has it. */
export function withBriefHeader(text: string, threadId: string, node: BoardNode, version: number, skills: readonly string[], provider: string): string {
  const head = briefHeader(threadId, node, version);
  if (text.trimStart().startsWith('Plan board: thread ')) return text;
  const sk = skillsLine(skills, provider);
  return `${head}\n${sk ? `${sk}\n` : ''}\n${text}`;
}
