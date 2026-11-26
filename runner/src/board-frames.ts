// Plan board screens (experiment planBoardMockups, experiments/planBoardMockups.md;
// needs planBoard): the agent's own HTML takes on a box ("three takes on KPI
// cards"), kept beside the board and drawn by the Mac app as sealed stills.
//
// Agent HTML is untrusted. Here it's checked and kept; it's only ever drawn
// by the app's sealed offscreen renderer (desktop/web/src/mac/sealed.rs:
// scripts off, no network, no navigation, nothing kept), as a picture. The
// policy below goes first in every stored file's <head>, so the live Open in
// browser view runs no scripts either. A route frame names a page on the
// thread's own dev server instead (loopback only).
//
// On disk: <dataDir>/boards/<threadId>/frames/<frameId>/v<n>/index.html, and
// frames.json beside them. The last MAX_FRAME_VERSIONS kept.
import fs from 'node:fs';
import path from 'node:path';
import { atomicWriteFileSync, readJsonSync } from './fsutil.ts';
import { BoardError, type Boards } from './boards.ts';

export const FRAME_POLICY = "default-src 'none'; style-src 'unsafe-inline'; img-src data: 'self'; font-src data: 'self'; script-src 'none'; form-action 'none'; base-uri 'none'; frame-src 'none'";
export const MAX_FRAME_BYTES = 2 * 1024 * 1024;
export const MAX_VARIANTS = 3;
export const MAX_FRAMES_PER_BOX = 6;
export const MAX_FRAME_VERSIONS = 10;
export const VIEWPORTS = ['desktop', 'tablet', 'phone'] as const;
export type Viewport = (typeof VIEWPORTS)[number];

const ID = /^[A-Za-z_][A-Za-z0-9_-]{0,47}$/;

export type FrameRecord = {
  id: string;
  node: string;
  frame: string;
  variant: string;
  title: string;
  viewport: Viewport;
  version: number;
  savedAt: string;
  route?: string;
  picked?: boolean;
  // The version Wayne last saw, for the Changed badge.
  seen?: number;
};

type FramesFile = { frames: Record<string, FrameRecord> };

/** Refuses HTML that would run anything or reach out, with a plain reason. */
export function passiveHtml(html: string): void {
  if (Buffer.byteLength(html, 'utf8') > MAX_FRAME_BYTES) throw new BoardError(`The screen is ${Math.round(Buffer.byteLength(html, 'utf8') / 1024)} KB; keep it under 2 MB.`);
  if (/<script[\s>/]/i.test(html)) throw new BoardError("The screen has a <script>. Screens are pictures: scripts never run, so take it out (render React to static HTML first).");
  if (/\son[a-z]+\s*=/i.test(html)) throw new BoardError('The screen has an on… event attribute. Take it out: nothing runs on a screen.');
  if (/javascript:/i.test(html)) throw new BoardError('The screen has a javascript: link. Take it out.');
  if (/<(iframe|object|embed|frame|frameset|base|meta\s+http-equiv\s*=\s*["']?refresh)/i.test(html)) throw new BoardError('The screen embeds or redirects to another page. Draw it inline instead.');
  if (/<link[^>]+rel\s*=\s*["']?stylesheet[^>]*href\s*=\s*["']?(https?:)?\/\//i.test(html) || /@import\s+(url\()?["']?(https?:)?\/\//i.test(html)) throw new BoardError('The screen loads CSS from the network. Compile Tailwind and put the CSS inline.');
  passiveSvg(html);
}

/** Active SVG (scripts, handlers, SMIL, external url()) is refused. */
export function passiveSvg(text: string): void {
  for (const svg of text.match(/<svg[\s\S]*?<\/svg>/gi) ?? []) {
    if (/<(animate|animateMotion|animateTransform|set|foreignObject)[\s>/]/i.test(svg)) throw new BoardError('An SVG in the screen animates or embeds HTML. Keep SVGs still.');
    if (/url\(\s*["']?\s*(?![#"']|data:)/i.test(svg)) throw new BoardError('An SVG in the screen points at an outside url(). Inline it as data: or use #ids.');
    if (/(xlink:)?href\s*=\s*["']?\s*(?![#"']|data:)/i.test(svg)) throw new BoardError('An SVG in the screen links outside itself. Use #ids or data: only.');
  }
}

/** The policy as the first thing in <head>; a later one can only tighten it. */
export function withPolicy(html: string): string {
  const meta = `<meta http-equiv="Content-Security-Policy" content="${FRAME_POLICY}">`;
  if (/<head[^>]*>/i.test(html)) return html.replace(/<head[^>]*>/i, (h) => `${h}${meta}`);
  if (/<html[^>]*>/i.test(html)) return html.replace(/<html[^>]*>/i, (h) => `${h}<head>${meta}</head>`);
  return `<!doctype html><html><head>${meta}</head><body>${html}</body></html>`;
}

/** A route frame's address: a page on this Mac's own dev server, nothing else. */
export function loopbackRoute(route: string): string {
  let u: URL;
  try {
    u = new URL(route);
  } catch {
    throw new BoardError(`"${route}" isn't an address. Give the dev server's page, like http://localhost:5173/__mockups/kpi?v=b.`);
  }
  if (u.protocol !== 'http:' && u.protocol !== 'https:') throw new BoardError('A route frame is an http page on the dev server.');
  if (!['localhost', '127.0.0.1', '[::1]'].includes(u.hostname)) throw new BoardError(`A route frame is on this Mac's dev server (localhost or 127.0.0.1), not ${u.hostname}.`);
  if (!u.port) throw new BoardError("Give the dev server's port, like http://localhost:5173/….");
  if (u.username || u.password) throw new BoardError('A route frame has no sign-in in its address.');
  return u.toString();
}

export type FrameWrite = { node: string; frame: string; variant?: string; title: string; viewport?: string; html?: string; route?: string; baseVersion?: number };

export class BoardFrames {
  readonly boards: Boards;
  private readonly now: () => number;
  constructor(boards: Boards, now: () => number = Date.now) {
    this.boards = boards;
    this.now = now;
  }

  private dir(threadId: string): string {
    if (!this.boards.has(threadId)) throw new BoardError('This thread has no plan board yet. Draw one with update_board first.');
    return path.join(this.boards.dir, threadId, 'frames');
  }

  private read(threadId: string): FramesFile {
    const f = readJsonSync<FramesFile>(path.join(this.dir(threadId), 'frames.json'));
    return f && typeof f.frames === 'object' ? f : { frames: {} };
  }

  private save(threadId: string, f: FramesFile): void {
    const dir = this.dir(threadId);
    fs.mkdirSync(dir, { recursive: true });
    atomicWriteFileSync(path.join(dir, 'frames.json'), `${JSON.stringify(f, null, 2)}\n`);
  }

  /** The file a frame's version is kept in, for the still and Open in browser. */
  file(threadId: string, frameId: string, version: number): string {
    if (!ID.test(frameId)) throw new BoardError(`"${frameId}" isn't a screen id.`);
    return path.join(this.dir(threadId), frameId, `v${version}`, 'index.html');
  }

  list(threadId: string, node?: string): (FrameRecord & { path?: string; changed: boolean })[] {
    return Object.values(this.read(threadId).frames)
      .filter((f) => !node || f.node === node)
      .sort((a, b) => a.node.localeCompare(b.node) || a.frame.localeCompare(b.frame) || a.variant.localeCompare(b.variant))
      .map((f) => ({ ...f, ...(f.route ? {} : { path: this.file(threadId, f.id, f.version) }), changed: f.version > 1 && (f.seen ?? 0) < f.version }));
  }

  write(threadId: string, w: FrameWrite): FrameRecord {
    const board = this.boards.get(threadId);
    if (!board) throw new BoardError('This thread has no plan board yet. Draw one with update_board first.');
    if (!board.graph.nodes.some((n) => n.id === w.node)) throw new BoardError(`The plan has no box "${w.node}".`);
    if (!ID.test(w.frame)) throw new BoardError(`"${w.frame}" isn't a screen id: letters, digits, - and _.`);
    const variant = (w.variant ?? 'A').trim().toUpperCase();
    if (!/^[A-C]$/.test(variant)) throw new BoardError('A variant is A, B or C: three takes at most.');
    const viewport = (w.viewport ?? 'desktop') as Viewport;
    if (!VIEWPORTS.includes(viewport)) throw new BoardError('viewport is desktop, tablet or phone.');
    if ((w.html === undefined) === (w.route === undefined)) throw new BoardError('Give html for a sketch screen, or route for a page on the dev server: one of them.');
    const title = w.title.replace(/\s+/g, ' ').trim().slice(0, 80) || w.frame;
    const route = w.route !== undefined ? loopbackRoute(w.route) : undefined;
    if (w.html !== undefined) passiveHtml(w.html);
    const id = `${w.node}-${w.frame}-${variant}`.slice(0, 48);
    const f = this.read(threadId);
    const was = f.frames[id];
    if (w.baseVersion !== undefined && w.baseVersion !== (was?.version ?? 0)) throw new BoardError(`That screen changed (it's at v${was?.version ?? 0}). Call read_frames again.`);
    if (!was) {
      const mine = Object.values(f.frames).filter((x) => x.node === w.node);
      if (mine.length >= MAX_FRAMES_PER_BOX * MAX_VARIANTS) throw new BoardError(`"${w.node}" has ${mine.length} screens already, the most a box keeps.`);
      if (new Set(mine.filter((x) => x.frame !== w.frame).map((x) => x.frame)).size >= MAX_FRAMES_PER_BOX && !mine.some((x) => x.frame === w.frame)) throw new BoardError(`A box has ${MAX_FRAMES_PER_BOX} screens at most.`);
    }
    const version = (was?.version ?? 0) + 1;
    if (w.html !== undefined) {
      const file = this.file(threadId, id, version);
      fs.mkdirSync(path.dirname(file), { recursive: true });
      atomicWriteFileSync(file, withPolicy(w.html));
      // The last MAX_FRAME_VERSIONS only.
      const old = version - MAX_FRAME_VERSIONS;
      if (old > 0) fs.rmSync(path.join(this.dir(threadId), id, `v${old}`), { recursive: true, force: true });
    }
    const record: FrameRecord = { id, node: w.node, frame: w.frame, variant, title, viewport, version, savedAt: new Date(this.now()).toISOString(), ...(route ? { route } : {}), ...(was?.picked ? { picked: true } : {}), ...(was?.seen ? { seen: was.seen } : {}) };
    f.frames[id] = record;
    this.save(threadId, f);
    return record;
  }

  /** Wayne's Pick: one variant of a screen, the others unpicked. */
  pick(threadId: string, frameId: string): FrameRecord {
    const f = this.read(threadId);
    const hit = f.frames[frameId];
    if (!hit) throw new BoardError(`No screen ${frameId}.`);
    for (const x of Object.values(f.frames)) if (x.node === hit.node && x.frame === hit.frame) x.picked = x.id === frameId;
    this.save(threadId, f);
    return f.frames[frameId];
  }

  /** Wayne has seen these versions: their Changed badges go. */
  seen(threadId: string): void {
    const f = this.read(threadId);
    for (const x of Object.values(f.frames)) x.seen = x.version;
    this.save(threadId, f);
  }
}

/** read_frames' text, for the agent. */
export function framesText(list: ReturnType<BoardFrames['list']>): string {
  if (!list.length) return 'No screens yet. Write one with update_frame.';
  return list.map((f) => `- ${f.id}: box ${f.node}, screen ${f.frame}, variant ${f.variant}, ${f.viewport}, v${f.version}${f.picked ? ', picked by Wayne' : ''}${f.route ? `, route ${f.route}` : ''}: ${f.title}`).join('\n');
}
