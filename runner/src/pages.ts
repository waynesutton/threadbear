// Pages (the pages experiment): a named, versioned page a thread makes for
// the user, like a bundle size report or a progress card. It outlives the
// reply it was posted in and can be updated in place. Unlike a pin, the
// runner keeps it, so it can belong to zero, one or several projects the
// way a thread can, whatever folder the thread ran in.
//
// Kept in <home>/pages: index.json for the records, and one folder per
// version, <id>/v<n>/, with index.html and the assets it names. Only the
// newest MAX_VERSIONS are kept.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { atomicWriteFileSync, readJsonSync } from './fsutil.ts';
import type { ProjectMaker, ThreadProject } from './protocol.ts';
import { ErrorCode, RpcError } from './protocol.ts';

export const MAX_VERSIONS = 10;
export const MAX_HTML_BYTES = 1_000_000;
// Room for one short MP4 clip next to the page.
export const MAX_PAGE_BYTES = 25_000_000;
export const MAX_SUMMARY = 280;
export const MAX_TITLE = 120;
// What a page folder may hold besides index.html: passive assets only.
// No .js (scripts live inline, and only on live pages) and no .json (a
// page can't read data files; that's for a later host channel).
export const PAGE_ASSETS = new Set(['.html', '.css', '.svg', '.png', '.jpg', '.jpeg', '.webp', '.gif', '.mp4', '.webm', '.woff2']);

export type PageProject = { id: string; by: ProjectMaker; pinned?: boolean };

export type PageRecord = {
  id: string;
  title: string;
  summary: string;
  ownerThread: string;
  projects: PageProject[];
  version: number;
  bytes: number;
  // Scripts allowed: they run only in the side pane, after the user's
  // "Run them" for that exact file. Static pages are refused any.
  live: boolean;
  // What made it, so the same report updates rather than piles up
  // ("bundle-analyzer summary").
  source?: string;
  createdAt: string;
  updatedAt: string;
};

export type PageContent = { html?: string; dir?: string };

export type CreatePage = PageContent & {
  title: string;
  summary: string;
  ownerThread: string;
  // Left out: the page joins the owner thread's project when it's pinned
  // to exactly one. [] always means no project.
  projects?: string[];
  ownerProjects?: readonly ThreadProject[];
  source?: string;
  live?: boolean;
};

export type UpdatePage = PageContent & { title?: string; summary?: string };

export type PageFilter = { project?: string; thread?: string; none?: boolean; source?: string };

const SCRIPT = /<script\b/i;

export class PageError extends Error {}

export class Pages {
  readonly dir: string;
  private readonly now: () => number;
  private readonly onChange: ((pages: PageRecord[]) => void) | undefined;
  private records: PageRecord[];

  // onChange: every change, with the whole list, for pages.updated.
  constructor(home: string, now: () => number = Date.now, onChange?: (pages: PageRecord[]) => void) {
    this.dir = path.join(home, 'pages');
    this.now = now;
    this.onChange = onChange;
    this.records = readJsonSync<PageRecord[]>(this.indexFile()) ?? [];
  }

  list(f: PageFilter = {}): PageRecord[] {
    return this.records
      .filter((p) => (f.thread ? p.ownerThread === f.thread : true))
      .filter((p) => (f.project ? p.projects.some((x) => x.id === f.project) : true))
      .filter((p) => (f.none ? p.projects.length === 0 : true))
      .filter((p) => (f.source ? p.source === f.source : true))
      .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
  }

  get(id: string): PageRecord | undefined {
    return this.records.find((p) => p.id === id);
  }

  // The page the side pane opens: the newest version's index.html.
  file(id: string, version?: number): string {
    const p = this.must(id);
    return path.join(this.versionDir(p.id, version ?? p.version), 'index.html');
  }

  create(c: CreatePage): PageRecord {
    const title = cleanTitle(c.title);
    const at = new Date(this.now()).toISOString();
    const id = this.newId(title);
    const live = c.live === true;
    const projects = c.projects
      ? [...new Set(c.projects)].map((pid): PageProject => ({ id: pid, by: 'agent', pinned: true }))
      : inherited(c.ownerProjects ?? []);
    const bytes = this.write(id, 1, c, live);
    const rec: PageRecord = {
      id,
      title,
      summary: cleanSummary(c.summary),
      ownerThread: c.ownerThread,
      projects,
      version: 1,
      bytes,
      live,
      ...(c.source ? { source: c.source.trim().slice(0, MAX_TITLE) } : {}),
      createdAt: at,
      updatedAt: at,
    };
    this.records.push(rec);
    this.persist();
    return rec;
  }

  update(id: string, u: UpdatePage): PageRecord {
    const p = this.must(id);
    const next = { ...p };
    if (u.title !== undefined) next.title = cleanTitle(u.title);
    if (u.summary !== undefined) next.summary = cleanSummary(u.summary);
    if (u.html !== undefined || u.dir !== undefined) {
      next.version = p.version + 1;
      next.bytes = this.write(p.id, next.version, u, p.live);
      this.prune(p.id, next.version);
    }
    next.updatedAt = new Date(this.now()).toISOString();
    this.records = this.records.map((r) => (r.id === p.id ? next : r));
    this.persist();
    return next;
  }

  setProjects(id: string, change: { add?: string[]; remove?: string[]; by?: ProjectMaker }): PageRecord {
    const p = this.must(id);
    const remove = new Set(change.remove ?? []);
    const kept = p.projects.filter((x) => !remove.has(x.id));
    const have = new Set(kept.map((x) => x.id));
    const added = (change.add ?? []).filter((x) => !have.has(x)).map((x): PageProject => ({ id: x, by: change.by ?? 'agent', pinned: true }));
    const next = { ...p, projects: [...kept, ...added], updatedAt: new Date(this.now()).toISOString() };
    this.records = this.records.map((r) => (r.id === p.id ? next : r));
    this.persist();
    return next;
  }

  // By id or a unique prefix, or a PageError saying how to find one.
  find(id: string): PageRecord {
    return this.must(id);
  }

  private must(id: string): PageRecord {
    const p = this.get(id) ?? this.records.find((r) => r.id.startsWith(id));
    if (!p) throw new PageError(`No page ${id}. list_pages shows them.`);
    return p;
  }

  private newId(title: string): string {
    const slug = title.toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_+|_+$/g, '').slice(0, 32) || 'page';
    let id = `pg_${slug}`;
    while (this.get(id)) id = `pg_${slug}_${crypto.randomBytes(2).toString('hex')}`;
    return id;
  }

  private versionDir(id: string, v: number): string {
    return path.join(this.dir, id, `v${v}`);
  }

  // Checks everything before anything is written, then copies into a temp
  // folder and renames it into place, so a refused page leaves nothing.
  private write(id: string, v: number, c: PageContent, live: boolean): number {
    if ((c.html === undefined) === (c.dir === undefined)) throw new PageError('Give the page as html or as dir, one of them.');
    const files: { rel: string; data: Buffer }[] = c.html !== undefined ? [{ rel: 'index.html', data: Buffer.from(c.html, 'utf8') }] : readPageDir(c.dir!);
    const index = files.find((f) => f.rel === 'index.html');
    if (!index) throw new PageError(`${c.dir} has no index.html.`);
    if (index.data.length > MAX_HTML_BYTES) throw new PageError(`index.html is ${index.data.length} bytes; the most is ${MAX_HTML_BYTES}. Move pictures into files next to it.`);
    const total = files.reduce((n, f) => n + f.data.length, 0);
    if (total > MAX_PAGE_BYTES) throw new PageError(`The page is ${total} bytes; the most is ${MAX_PAGE_BYTES}.`);
    if (!live) {
      for (const f of files) {
        if (f.rel.endsWith('.html') && SCRIPT.test(f.data.toString('utf8'))) {
          throw new PageError(`${f.rel} has a <script>. Pages are static unless made with live: true; draw with HTML, CSS and SVG instead.`);
        }
      }
    }
    const dest = this.versionDir(id, v);
    const tmp = `${dest}.${process.pid}.tmp`;
    fs.rmSync(tmp, { recursive: true, force: true });
    for (const f of files) {
      const to = path.join(tmp, f.rel);
      fs.mkdirSync(path.dirname(to), { recursive: true });
      fs.writeFileSync(to, f.data);
    }
    fs.rmSync(dest, { recursive: true, force: true });
    fs.renameSync(tmp, dest);
    return total;
  }

  private prune(id: string, newest: number): void {
    for (let v = newest - MAX_VERSIONS; v >= 1; v--) {
      const d = this.versionDir(id, v);
      if (!fs.existsSync(d)) break;
      fs.rmSync(d, { recursive: true, force: true });
    }
  }

  private indexFile(): string {
    return path.join(this.dir, 'index.json');
  }

  // The versions still on disk, oldest first.
  versions(id: string): number[] {
    const p = this.must(id);
    const out: number[] = [];
    for (let v = Math.max(1, p.version - MAX_VERSIONS + 1); v <= p.version; v++) if (fs.existsSync(this.versionDir(p.id, v))) out.push(v);
    return out;
  }

  private persist(): void {
    atomicWriteFileSync(this.indexFile(), JSON.stringify(this.records, null, 2) + '\n');
    this.onChange?.(this.list());
  }
}

// One pinned project, and the page goes there too; a thread in several, or
// only guessed into one by the librarian, leaves the page in none.
function inherited(ps: readonly ThreadProject[]): PageProject[] {
  const pinned = ps.filter((p) => p.pinned);
  return pinned.length === 1 ? [{ id: pinned[0]!.id, by: 'agent' }] : [];
}

function cleanTitle(t: string): string {
  const s = t.trim().slice(0, MAX_TITLE);
  if (!s) throw new PageError('A page needs a title.');
  return s;
}

function cleanSummary(t: string): string {
  return t.trim().slice(0, MAX_SUMMARY);
}

// Every file under dir, refusing links and anything not on PAGE_ASSETS, so
// a page can't carry scripts or data files in beside it.
export function readPageDir(dir: string): { rel: string; data: Buffer }[] {
  let st: fs.Stats;
  try {
    st = fs.lstatSync(dir);
  } catch {
    throw new PageError(`No folder ${dir}.`);
  }
  if (!st.isDirectory()) throw new PageError(`${dir} isn't a folder.`);
  const out: { rel: string; data: Buffer }[] = [];
  const walk = (d: string) => {
    for (const e of fs.readdirSync(d, { withFileTypes: true })) {
      const full = path.join(d, e.name);
      const rel = path.relative(dir, full);
      if (e.name.startsWith('.')) continue;
      if (e.isSymbolicLink()) throw new PageError(`${rel} is a link. Copy the file in instead.`);
      if (e.isDirectory()) walk(full);
      else if (e.isFile()) {
        if (!PAGE_ASSETS.has(path.extname(e.name).toLowerCase())) throw new PageError(`${rel} can't go in a page. Allowed: ${[...PAGE_ASSETS].join(' ')}.`);
        out.push({ rel, data: fs.readFileSync(full) });
      }
    }
  };
  walk(dir);
  return out;
}

// docs/protocol.md, "Pages". `pages` is undefined while the experiment is
// off: lists come back empty and the rest say so.
export function pageMethods(pages: () => Pages | undefined): Record<string, (p: Record<string, unknown>) => unknown> {
  const must = (): Pages => {
    const s = pages();
    if (!s) throw new RpcError(ErrorCode.badParams, 'Pages is off. Turn it on in Settings > General > Experiments.');
    return s;
  };
  const str = (p: Record<string, unknown>, k: string): string | undefined => {
    const v = p[k];
    if (v === undefined || v === null) return undefined;
    if (typeof v !== 'string') throw new RpcError(ErrorCode.badParams, `${k} must be a string`);
    return v;
  };
  const list = (p: Record<string, unknown>, k: string): string[] | undefined => {
    const v = p[k];
    if (v === undefined || v === null) return undefined;
    if (!Array.isArray(v) || v.some((x) => typeof x !== 'string')) throw new RpcError(ErrorCode.badParams, `${k} must be a list of strings`);
    return v as string[];
  };
  const id = (p: Record<string, unknown>): string => {
    const v = str(p, 'pageId');
    if (!v) throw new RpcError(ErrorCode.badParams, 'pageId is required');
    return v;
  };
  const found = <T>(fn: () => T): T => {
    try {
      return fn();
    } catch (e) {
      if (e instanceof PageError) throw new RpcError(ErrorCode.badParams, e.message);
      throw e;
    }
  };
  return {
    'pages.list': (p) => {
      const s = pages();
      if (!s) return { pages: [] };
      const project = str(p, 'project');
      const thread = str(p, 'thread');
      return { pages: s.list({ ...(project ? { project } : {}), ...(thread ? { thread } : {}), ...(p.none === true ? { none: true } : {}) }) };
    },
    'pages.get': (p) =>
      found(() => {
        const s = must();
        const page = s.find(id(p));
        return { page, file: s.file(page.id), versions: s.versions(page.id) };
      }),
    'pages.setProjects': (p) => {
      const add = list(p, 'add');
      const remove = list(p, 'remove');
      return found(() => ({ page: must().setProjects(id(p), { ...(add ? { add } : {}), ...(remove ? { remove } : {}), by: 'mike' }) }));
    },
  };
}
