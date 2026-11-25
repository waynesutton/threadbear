// Native Tasks and Linear (docs/protocol.md, "Tasks and Linear"). The runner
// talks to Linear through the Connectors gateway, as its own `threadbear`
// login: that login sees only the Linear tools this needs, may create an
// issue without asking (the click in the send sheet is the approval), and
// still has to ask before changing one. Without that login it falls back on
// the main one, whose creates wait for approval like an agent's.
//
// Sending makes one issue per task (a claim in Convex, then a marker search
// so a send that crashed after creating links the issue it made). Sync only
// reads Linear, only for tasks with sync on, and Convex decides what a
// change does to the task (convex/tasksLinear.ts). No model is involved.
import { randomUUID } from 'node:crypto';
import type { Logger } from './log.ts';
import { helperForAgent, helperHeaders, parseRpc } from './notion-tasks.ts';
import { credentialUrlProblem } from './secure-url.ts';
import type { NativeTask, TaskLink, TaskPriority } from './tasks.ts';

export const LINEAR_API = 109;
const CALL_MS = 60_000;
// A held call waits 45 seconds at the gateway; approval can take a day.
const APPROVAL_MS = 24 * 3600_000;
const CACHE_MS = 5 * 60_000;
export const SWEEP_TICK_MS = 10 * 60_000;
const AT_ONCE = 3;
const MAX_IMAGES = 10;
const IMAGE_LIMIT = 10 * 1024 * 1024;
const ISSUE_FIELDS = ['id', 'uuid', 'title', 'url', 'priority', 'dueDate', 'status', 'statusType', 'updatedAt', 'team', 'teamId', 'project', 'projectId', 'parentId'];

export type CallResult = { text: string } | { waiting: { id: string; url: string } };
export type LinearCall = (name: string, args: Record<string, unknown>, timeoutMs?: number) => Promise<CallResult>;

// A held call answers in plain text, not as an error: "Waiting for Wayne to
// approve this at http://127.0.0.1:8795/approvals/ap_… ".
export function waitingFor(text: string): { id: string; url: string } | undefined {
  if (!/waiting for/i.test(text)) return undefined;
  const m = /(https?:\/\/\S+?\/approvals\/(ap_[\w-]+))/.exec(text);
  return m ? { id: m[2]!, url: m[1]! } : undefined;
}

// Calls the gateway's tools. Tries the Threadbear login first and the main
// login when that one isn't set up, so the feature keeps working (asking
// twice) on a Mac that only has the main one.
export function linearCaller(opts: { mcpUrl: string; headers: () => Promise<Record<string, string>>; fallback?: () => Promise<Record<string, string>>; fetch?: typeof fetch; log?: Logger }): LinearCall {
  const f = opts.fetch ?? fetch;
  const problem = credentialUrlProblem(opts.mcpUrl);
  let id = 0;
  let useFallback = false;
  const headers = async () => {
    if (!useFallback) {
      try {
        return await opts.headers();
      } catch (err) {
        if (!opts.fallback) throw err;
        useFallback = true;
        opts.log?.warn('linear: no Threadbear login for Connectors, using the main one (creates will ask first)');
      }
    }
    return opts.fallback!();
  };
  return async (name, args, timeoutMs = CALL_MS) => {
    if (problem) throw new Error(`The Connectors gateway URL ${problem}.`);
    const res = await f(opts.mcpUrl, {
      method: 'POST',
      headers: { ...(await headers()), 'content-type': 'application/json', accept: 'application/json, text/event-stream' },
      body: JSON.stringify({ jsonrpc: '2.0', id: ++id, method: 'tools/call', params: { name, arguments: args } }),
      redirect: 'error',
      signal: AbortSignal.timeout(timeoutMs),
    });
    if (res.status === 401 && !useFallback && opts.fallback) {
      useFallback = true;
      return linearCaller({ ...opts, headers: opts.fallback, fallback: undefined })(name, args, timeoutMs);
    }
    if (!res.ok) throw new Error(`the Connectors gateway answered ${res.status}`);
    const msg = parseRpc(await res.text());
    if (msg.error) throw new Error(msg.error.message ?? 'the Connectors gateway gave an error');
    const text = (msg.result?.content ?? []).filter((c) => c.type === 'text').map((c) => c.text ?? '').join('\n');
    if (msg.result?.isError) throw new LinearError(text);
    const waiting = waitingFor(text);
    return waiting ? { waiting } : { text };
  };
}
export class LinearError extends Error {
  constructor(text: string) {
    let message = text.slice(0, 300) || 'Linear gave an error';
    try {
      const j = JSON.parse(text) as { message?: string };
      if (typeof j.message === 'string') message = j.message;
    } catch {
      // plain text
    }
    super(message);
  }
  get notFound() {
    return /could not find|not found|entity not found/i.test(this.message);
  }
}

// ------------------------------------------------------------- shapes

export type IssueSnap = { issueId: string; identifier: string; url: string; title: string; priority: number; dueDate: string | null; status: string; statusType: string; updatedAt: string };
export type IssueRow = IssueSnap & { teamId: string; team: string; projectId?: string; project?: string; parentId?: string };
type Json = Record<string, unknown>;
const str = (v: unknown) => (typeof v === 'string' ? v : undefined);
export function parseJson(text: string): Json {
  try {
    const j = JSON.parse(text) as unknown;
    if (j && typeof j === 'object') return j as Json;
  } catch {
    // fall through
  }
  throw new Error("Linear's answer wasn't JSON");
}
export function issueRow(raw: Json): IssueRow | undefined {
  const issue = (raw.issue && typeof raw.issue === 'object' ? raw.issue : raw) as Json;
  const identifier = str(issue.identifier) ?? str(issue.id);
  const issueId = str(issue.uuid) ?? (str(issue.id) && /^[0-9a-f-]{36}$/.test(str(issue.id)!) ? str(issue.id) : undefined);
  const url = str(issue.url);
  if (!identifier || !issueId || !url) return undefined;
  const p = issue.priority;
  const priority = typeof p === 'number' ? p : p && typeof p === 'object' && typeof (p as Json).value === 'number' ? ((p as Json).value as number) : 0;
  const status = issue.status && typeof issue.status === 'object' ? (str((issue.status as Json).name) ?? '') : (str(issue.status) ?? '');
  const statusType = str(issue.statusType) ?? (issue.status && typeof issue.status === 'object' ? str((issue.status as Json).type) : undefined) ?? 'unstarted';
  const team = issue.team && typeof issue.team === 'object' ? (str((issue.team as Json).name) ?? '') : (str(issue.team) ?? '');
  const project = issue.project && typeof issue.project === 'object' ? str((issue.project as Json).name) : str(issue.project);
  return {
    issueId,
    identifier,
    url,
    title: str(issue.title) ?? identifier,
    priority,
    dueDate: str(issue.dueDate) ?? null,
    status,
    statusType,
    updatedAt: str(issue.updatedAt) ?? new Date(0).toISOString(),
    teamId: str(issue.teamId) ?? '',
    team,
    ...(str(issue.projectId) ? { projectId: str(issue.projectId)! } : {}),
    ...(project ? { project } : {}),
    ...(str(issue.parentId) ? { parentId: str(issue.parentId)! } : {}),
  };
}
export const snapOf = (r: IssueRow): IssueSnap => ({ issueId: r.issueId, identifier: r.identifier, url: r.url, title: r.title, priority: r.priority, dueDate: r.dueDate, status: r.status, statusType: r.statusType, updatedAt: r.updatedAt });
export type CommentSnap = { id: string; body: string; author: string; createdAt: string };
export function commentsOf(raw: Json): CommentSnap[] {
  const list = Array.isArray(raw.comments) ? (raw.comments as Json[]) : [];
  return list.flatMap((c) => {
    const id = str(c.id);
    const body = str(c.body);
    if (!id || !body) return [];
    const author = c.author && typeof c.author === 'object' ? (str((c.author as Json).name) ?? 'Someone') : 'Someone';
    return [{ id, body, author, createdAt: str(c.createdAt) ?? new Date().toISOString() }];
  });
}
// An issue key or URL as typed: ENG-123, or https://linear.app/x/issue/ENG-123/...
export function issueKey(input: string): string | undefined {
  const t = input.trim();
  const m = /linear\.app\/[^/]+\/issue\/([A-Za-z][A-Za-z0-9]*-\d+)/.exec(t) ?? /^([A-Za-z][A-Za-z0-9]*-\d+)$/.exec(t);
  return m ? m[1]!.toUpperCase() : undefined;
}

export const priorityNumber = (p: TaskPriority | null | undefined) => (p === 'urgent' ? 1 : p === 'high' ? 2 : p === 'normal' ? 3 : p === 'low' ? 4 : 0);
export const marker = (taskId: string) => `Threadbear task ${taskId}`;

export type Picture = { storageId: string; title: string; imageUrl: string };
// What a send puts in the issue's description, before pictures: the notes,
// sub-tasks as a checklist, and the line that finds this issue again.
export function describe(task: Pick<NativeTask, 'id' | 'notes'>, subtasks: Pick<NativeTask, 'title' | 'status'>[], pictures: { title: string; url: string }[] = []): string {
  const parts: string[] = [];
  if (task.notes.trim()) parts.push(task.notes.trim());
  if (subtasks.length) parts.push(['**Sub-tasks**', ...subtasks.map((s) => `- [${s.status === 'done' ? 'x' : ' '}] ${s.title.replace(/\n/g, ' ')}`)].join('\n'));
  if (pictures.length) parts.push(pictures.map((p) => `![${p.title.replace(/[[\]]/g, '')}](${p.url})`).join('\n\n'));
  parts.push(`---\nSent from Threadbear · ${marker(task.id)}`);
  return parts.join('\n\n');
}
// Web links go with the issue; prints, threads and pictures don't (pictures
// go into the description as Linear uploads).
export function webLinks(links: TaskLink[] | undefined): { url: string; title: string }[] {
  return (links ?? []).filter((l) => l.kind === 'link' && l.id !== 'linear-issue' && l.url && /^https?:\/\//i.test(l.url)).slice(0, 20).map((l) => ({ url: l.url!, title: l.title || l.url! }));
}
export function picturesOf(links: TaskLink[] | undefined): Picture[] {
  return (links ?? []).flatMap((l) => (l.kind === 'image' && l.storageId && l.imageUrl ? [{ storageId: l.storageId, title: l.title || 'Image', imageUrl: l.imageUrl }] : []));
}

// ------------------------------------------------------------- the feature

type TaskLinear = NonNullable<NativeTask['linear']>;
export type LinearDeps = {
  // The gateway, or undefined when there's none.
  call: () => LinearCall | undefined;
  // Is Linear signed in at the gateway (connectors.ts status)?
  connected: () => boolean;
  apiVersion: () => number | undefined;
  tasksOn: () => boolean;
  // tasks:* and tasksLinear:* on the backend, through the Tasks bridge.
  tasks: (op: string, args?: Record<string, unknown>) => Promise<unknown>;
  linear: (op: string, args?: Record<string, unknown>) => Promise<unknown>;
  fetch?: typeof fetch;
  log?: Logger;
  now?: () => number;
};
type Team = { id: string; key: string; name: string; displayName: string };
type Project = { id: string; identifier: string; name: string; status: string };

export class LinearTasks {
  private deps: LinearDeps;
  private cache = new Map<string, { at: number; value: unknown }>();
  private sending = new Set<string>();
  private timer?: NodeJS.Timeout;
  private sweeping?: Promise<unknown>;
  constructor(deps: LinearDeps) {
    this.deps = deps;
  }
  private now() {
    return this.deps.now?.() ?? Date.now();
  }
  // Why Linear isn't offered, or undefined when it is.
  why(): string | undefined {
    if (!this.deps.tasksOn()) return 'Turn on Tasks in Settings > Experiments.';
    if (!this.deps.call()) return 'Connectors isn’t set up on this Mac.';
    if (!this.deps.connected()) return 'Linear isn’t connected. Connect it in Connectors.';
    const api = this.deps.apiVersion();
    if (api !== undefined && api < LINEAR_API) return 'Update the Threadbear backend to use Linear with Tasks.';
    return undefined;
  }
  status() {
    const why = this.why();
    return { ready: !why, ...(why ? { why } : {}) };
  }
  private gate(): LinearCall {
    const why = this.why();
    if (why) throw new Error(why);
    return this.deps.call()!;
  }
  private async json(name: string, args: Record<string, unknown>): Promise<Json> {
    const r = await this.gate()(name, args);
    if ('waiting' in r) throw new Error('Connectors is asking before this Linear call; approve it in Connectors and try again.');
    return parseJson(r.text);
  }
  private async cached<T>(key: string, refresh: boolean, load: () => Promise<T>): Promise<T> {
    const hit = this.cache.get(key);
    if (!refresh && hit && this.now() - hit.at < CACHE_MS) return hit.value as T;
    const value = await load();
    this.cache.set(key, { at: this.now(), value });
    return value;
  }

  // linear.options: the workspace and its teams, for the send sheet and picker.
  async options(p: { refresh?: boolean } = {}) {
    return this.cached('options', p.refresh === true, async () => {
      const ws = await this.json('linear__get_workspace', {});
      const teams: Team[] = [];
      let cursor: string | undefined;
      for (let page = 0; page < 4; page++) {
        const r = await this.json('linear__list_teams', { limit: 250, ...(cursor ? { cursor } : {}) });
        for (const t of (Array.isArray(r.teams) ? r.teams : []) as Json[]) {
          const id = str(t.id);
          if (id) teams.push({ id, key: str(t.key) ?? '', name: str(t.name) ?? '', displayName: str(t.displayName) ?? str(t.name) ?? '' });
        }
        cursor = r.hasNextPage ? str(r.cursor) : undefined;
        if (!cursor) break;
      }
      teams.sort((a, b) => a.displayName.localeCompare(b.displayName));
      return { workspace: { name: str(ws.name) ?? 'Linear', url: str(ws.url) ?? '' }, teams };
    });
  }
  // linear.projects: a team's open projects. id is the uuid issues use.
  async projects(p: { teamId?: unknown; refresh?: boolean }) {
    if (typeof p.teamId !== 'string' || !p.teamId) throw new Error('Choose a team');
    const teamId = p.teamId;
    return this.cached(`projects:${teamId}`, p.refresh === true, async () => {
      const projects: Project[] = [];
      let cursor: string | undefined;
      for (let page = 0; page < 6; page++) {
        const r = await this.json('linear__list_projects', { team: teamId, limit: 50, fields: ['id', 'uuid', 'name', 'status'], ...(cursor ? { cursor } : {}) });
        for (const x of (Array.isArray(r.projects) ? r.projects : []) as Json[]) {
          const uuid = str(x.uuid);
          const type = x.status && typeof x.status === 'object' ? str((x.status as Json).type) : undefined;
          if (!uuid || type === 'completed' || type === 'canceled') continue;
          projects.push({ id: uuid, identifier: str(x.id) ?? '', name: str(x.name) ?? '', status: (x.status && typeof x.status === 'object' ? str((x.status as Json).name) : undefined) ?? '' });
        }
        cursor = r.hasNextPage ? str(r.cursor) : undefined;
        if (!cursor) break;
      }
      projects.sort((a, b) => a.name.localeCompare(b.name));
      return { projects };
    });
  }

  private async task(id: unknown): Promise<NativeTask> {
    if (typeof id !== 'string' || !id) throw new Error('Choose a task');
    const task = (await this.deps.tasks('get', { id })) as NativeTask | null;
    if (!task || task.deleted) throw new Error('That task no longer exists');
    return task;
  }
  private async subtasks(id: string): Promise<NativeTask[]> {
    const out: NativeTask[] = [];
    let after: string | undefined;
    for (let page = 0; page < 20; page++) {
      const r = (await this.deps.tasks('list', { limit: 250, ...(after ? { after } : {}) })) as { items: NativeTask[]; next: string | null };
      out.push(...r.items.filter((t) => t.parentId === id));
      if (!r.next) break;
      after = r.next;
    }
    return out.sort((a, b) => a.position - b.position);
  }

  // linear.preview: exactly what a send would put in Linear.
  async preview(p: { taskId?: unknown }) {
    this.gate();
    const task = await this.task(p.taskId);
    const subs = await this.subtasks(task.id);
    return {
      title: task.title,
      description: describe(task, subs),
      priority: priorityNumber(task.priority),
      dueDate: task.dueDate ?? null,
      subtasks: subs.length,
      links: webLinks(task.links),
      pictures: picturesOf(task.links).map((x) => ({ id: x.storageId, title: x.title, imageUrl: x.imageUrl })),
    };
  }

  // linear.send: claims the task, then makes the issue in the background.
  // The task's linear field says how it went (creating, linked, failed).
  async send(p: { taskId?: unknown; teamId?: unknown; projectId?: unknown; title?: unknown; imageIds?: unknown; sync?: unknown }) {
    this.gate();
    const task = await this.task(p.taskId);
    if (task.status === 'done') throw new Error('This task is done: reopen it to send it to Linear.');
    if (task.linear?.state === 'linked') throw new Error(`Already in Linear as ${task.linear.identifier}.`);
    if (typeof p.teamId !== 'string' || !p.teamId) throw new Error('Choose a team');
    const { teams } = await this.options();
    const team = teams.find((t) => t.id === p.teamId);
    if (!team) throw new Error('That team isn’t in your Linear workspace any more.');
    let project: Project | undefined;
    if (typeof p.projectId === 'string' && p.projectId) {
      // Checked fresh: a project that left the team since the sheet opened can't be used.
      const { projects } = await this.projects({ teamId: team.id, refresh: true });
      project = projects.find((x) => x.id === p.projectId);
      if (!project) throw new Error('That project isn’t open in this team any more. Choose another.');
    }
    const title = typeof p.title === 'string' && p.title.trim() ? p.title.trim().slice(0, 255) : task.title;
    const wanted = new Set(Array.isArray(p.imageIds) ? p.imageIds.filter((x): x is string => typeof x === 'string') : []);
    const requestId = randomUUID();
    const sync = p.sync !== false;
    const { uploads } = (await this.deps.linear('claim', { id: task.id, requestId, teamId: team.id, teamName: team.displayName, ...(project ? { projectId: project.id, projectName: project.name } : {}), sync })) as { uploads: { storageId: string; assetUrl: string }[] };
    this.sending.add(task.id);
    void this.create(task, { requestId, team, project, title, pictures: picturesOf(task.links).filter((x) => wanted.has(x.storageId)).slice(0, MAX_IMAGES), uploads })
      .catch((err: unknown) => this.deps.linear('settle', { id: task.id, requestId, error: err instanceof Error ? err.message : String(err) }).catch(() => undefined))
      .finally(() => this.sending.delete(task.id));
    return { started: true, requestId };
  }

  // An issue an earlier try of this send made, found by its marker line.
  private async findSent(taskId: string, teamId: string): Promise<IssueRow | undefined> {
    const r = await this.json('linear__list_issues', { query: marker(taskId), team: teamId, limit: 10, includeArchived: true, fields: ['id', 'uuid'] });
    for (const x of (Array.isArray(r.issues) ? r.issues : []) as Json[]) {
      const key = str(x.id);
      if (!key) continue;
      // Linear's search is fuzzy: only an exact marker in the full description counts.
      const full = await this.json('linear__get_issue', { id: key, fields: [...ISSUE_FIELDS, 'description'] });
      if (str(full.description)?.includes(marker(taskId))) return issueRow(full);
    }
    return undefined;
  }
  private async upload(picture: Picture): Promise<string> {
    const f = this.deps.fetch ?? fetch;
    const got = await f(picture.imageUrl, { redirect: 'follow', signal: AbortSignal.timeout(CALL_MS) });
    if (!got.ok) throw new Error(`couldn't read ${picture.title}`);
    const bytes = new Uint8Array(await got.arrayBuffer());
    if (!bytes.byteLength || bytes.byteLength > IMAGE_LIMIT) throw new Error(`${picture.title} is too big`);
    const contentType = got.headers.get('content-type')?.split(';')[0]?.trim() || 'image/png';
    const ext = contentType.split('/')[1]?.replace('jpeg', 'jpg') ?? 'png';
    const filename = `${picture.title.replace(/[^\w.-]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 60) || 'picture'}${/\.\w{2,4}$/.test(picture.title) ? '' : `.${ext}`}`;
    // Prepare, then PUT straight away: the signed URL lasts 60 seconds.
    const prep = await this.json('linear__prepare_file_upload', { filename, contentType, size: bytes.byteLength });
    const req = prep.uploadRequest as { url?: unknown; method?: unknown; headers?: unknown } | undefined;
    const assetUrl = str(prep.assetUrl);
    const url = str(req?.url);
    if (!url || !assetUrl || !/^https:\/\//.test(url) || !/^https:\/\//.test(assetUrl)) throw new Error('Linear didn’t give an upload address');
    const headers: Record<string, string> = {};
    if (Array.isArray(req!.headers)) for (const h of req!.headers as Json[]) { if (str(h.key) && str(h.value) !== undefined) headers[str(h.key)!] = str(h.value)!; }
    else if (req!.headers && typeof req!.headers === 'object') for (const [k, v] of Object.entries(req!.headers as Json)) if (typeof v === 'string') headers[k] = v;
    const put = await f(url, { method: str(req!.method) ?? 'PUT', headers, body: bytes, redirect: 'error', signal: AbortSignal.timeout(CALL_MS) });
    if (!put.ok) throw new Error(`Linear refused ${picture.title} (${put.status})`);
    return assetUrl;
  }
  private async create(task: NativeTask, o: { requestId: string; team: Team; project?: Project; title: string; pictures: Picture[]; uploads: { storageId: string; assetUrl: string }[] }) {
    const settle = (args: Record<string, unknown>) => this.deps.linear('settle', { id: task.id, requestId: o.requestId, ...args });
    const already = await this.findSent(task.id, o.team.id);
    if (already) return settle({ issue: snapOf(already), found: true });
    const done = new Map(o.uploads.map((u) => [u.storageId, u.assetUrl]));
    const pictures: { title: string; url: string }[] = [];
    let failed = 0;
    for (const picture of o.pictures) {
      try {
        let assetUrl = done.get(picture.storageId);
        if (!assetUrl) {
          assetUrl = await this.upload(picture);
          await this.deps.linear('uploaded', { id: task.id, requestId: o.requestId, storageId: picture.storageId, assetUrl });
        }
        pictures.push({ title: picture.title, url: assetUrl });
      } catch (err) {
        failed++;
        this.deps.log?.warn(`linear: picture for task ${task.id} not uploaded: ${err instanceof Error ? err.message : String(err)}`);
      }
    }
    const subs = await this.subtasks(task.id);
    const description = describe(task, subs, pictures);
    const links = webLinks(task.links);
    const due = task.dueDate && /^\d{4}-\d{2}-\d{2}$/.test(task.dueDate) ? task.dueDate : undefined;
    const args = {
      team: o.team.id,
      title: o.title,
      description,
      ...(o.project ? { project: o.project.id } : {}),
      ...(priorityNumber(task.priority) ? { priority: priorityNumber(task.priority) } : {}),
      ...(due ? { dueDate: due } : {}),
      ...(links.length ? { links } : {}),
    };
    const call = this.gate();
    let r = await call('linear__save_issue', args, CALL_MS);
    if ('waiting' in r) {
      await this.deps.linear('waiting', { id: task.id, requestId: o.requestId, url: r.waiting.url });
      const until = this.now() + APPROVAL_MS;
      const approval = r.waiting.id;
      while ('waiting' in r && this.now() < until) r = await call('connectors__await_approval', { id: approval }, CALL_MS);
      if ('waiting' in r) throw new Error('Nobody approved the Linear call within a day');
    }
    const made = parseJson(r.text);
    const key = issueKey(str(made.identifier) ?? str(made.id) ?? str((made.issue as Json | undefined)?.identifier) ?? '') ?? issueKey(str(made.url) ?? str((made.issue as Json | undefined)?.url) ?? '') ?? /\b([A-Z][A-Z0-9]*-\d+)\b/.exec(r.text)?.[1];
    // The full issue, so the task has the same fields a sync would give it.
    const full = key ? issueRow(await this.json('linear__get_issue', { id: key, fields: ISSUE_FIELDS })) : issueRow(made);
    if (!full) throw new Error('Linear made the issue but its answer didn’t say which');
    await settle({ issue: snapOf(full) });
    if (failed) this.deps.log?.warn(`linear: ${failed} picture${failed === 1 ? '' : 's'} left out of ${full.identifier}`);
    return undefined;
  }

  private async fetchIssue(key: string) {
    const raw = await this.json('linear__get_issue', { id: key, fields: ISSUE_FIELDS });
    const row = issueRow(raw);
    if (!row) throw new Error('Linear’s answer had no issue in it');
    return row;
  }
  private async comments(issue: string): Promise<CommentSnap[]> {
    return commentsOf(await this.json('linear__list_comments', { issueId: issue, limit: 100, orderBy: 'createdAt' }));
  }
  // Brings one task up to date with its issue (linear.refresh, and the sweep).
  async refresh(p: { taskId?: unknown }) {
    this.gate();
    const task = await this.task(p.taskId);
    const l = task.linear;
    if (!l || l.state !== 'linked' || !l.issueId) return { changed: [] };
    return this.refreshOne(task.id, l);
  }
  private async refreshOne(taskId: string, l: Pick<TaskLinear, 'issueId' | 'identifier' | 'sync'>) {
    let row: IssueRow;
    try {
      row = await this.fetchIssue(l.identifier || l.issueId!);
    } catch (err) {
      if (err instanceof LinearError && err.notFound) {
        await this.deps.linear('missing', { id: taskId, issueId: l.issueId });
        return { changed: [], missing: true };
      }
      throw err;
    }
    const comments = l.sync ? await this.comments(row.identifier) : undefined;
    return this.deps.linear('apply', { id: taskId, issue: snapOf(row), ...(comments ? { comments } : {}) });
  }

  // linear.search: issues for the Add from Linear picker.
  async search(p: { query?: unknown; teamId?: unknown; projectId?: unknown; assignedToMe?: unknown; openOnly?: unknown; cursor?: unknown }) {
    const args: Record<string, unknown> = { limit: 50, fields: ISSUE_FIELDS };
    if (typeof p.query === 'string' && p.query.trim()) {
      const key = issueKey(p.query);
      if (key) {
        const row = await this.fetchIssue(key).catch(() => undefined);
        return this.marked(row ? [row] : [], undefined);
      }
      args.query = p.query.trim().slice(0, 200);
    }
    if (typeof p.teamId === 'string' && p.teamId) args.team = p.teamId;
    if (typeof p.projectId === 'string' && p.projectId) args.project = p.projectId;
    if (p.assignedToMe !== false) args.assignee = 'me';
    if (typeof p.cursor === 'string' && p.cursor) args.cursor = p.cursor;
    const r = await this.json('linear__list_issues', args);
    let rows = ((Array.isArray(r.issues) ? r.issues : []) as Json[]).map(issueRow).filter((x): x is IssueRow => Boolean(x));
    if (p.openOnly !== false) rows = rows.filter((x) => x.statusType !== 'completed' && x.statusType !== 'canceled');
    return this.marked(rows, r.hasNextPage ? str(r.cursor) : undefined);
  }
  private async marked(rows: IssueRow[], cursor: string | undefined) {
    const linked = rows.length ? ((await this.deps.linear('linkedIssues', { issueIds: rows.map((x) => x.issueId) })) as { issueId: string; taskId: string; title: string }[]) : [];
    const by = new Map(linked.map((x) => [x.issueId, x]));
    return {
      issues: rows.map((x) => ({ ...x, ...(by.get(x.issueId) ? { inTasks: { taskId: by.get(x.issueId)!.taskId, title: by.get(x.issueId)!.title } } : {}) })),
      ...(cursor ? { cursor } : {}),
    };
  }

  // linear.import: picked issues become tasks. Sub-issues come one level deep when asked.
  async import(p: { issues?: unknown; project?: unknown; sync?: unknown; includeSubIssues?: unknown }) {
    this.gate();
    const keys = Array.isArray(p.issues) ? p.issues.filter((x): x is string => typeof x === 'string').slice(0, 25) : [];
    if (!keys.length) throw new Error('Pick at least one issue');
    const { teams } = await this.options();
    const teamName = (id: string, fallback: string) => teams.find((t) => t.id === id)?.displayName ?? fallback;
    const items: Record<string, unknown>[] = [];
    const seen = new Set<string>();
    const add = async (row: IssueRow, parentIssueId?: string) => {
      if (seen.has(row.issueId)) return;
      seen.add(row.issueId);
      const full = await this.json('linear__get_issue', { id: row.identifier, fields: ['id', 'description'] });
      items.push({
        issue: snapOf(row),
        description: str(full.description) ?? '',
        ...(parentIssueId ? { parentIssueId } : {}),
        teamId: row.teamId,
        teamName: teamName(row.teamId, row.team),
        ...(row.projectId ? { projectId: row.projectId, projectName: row.project ?? '' } : {}),
        comments: await this.comments(row.identifier),
      });
    };
    for (const key of keys) {
      const row = await this.fetchIssue(key);
      await add(row);
      if (p.includeSubIssues === true) {
        const r = await this.json('linear__list_issues', { parentId: row.identifier, limit: 50, fields: ISSUE_FIELDS });
        for (const x of (Array.isArray(r.issues) ? r.issues : []) as Json[]) {
          const child = issueRow(x);
          if (child) await add(child, row.issueId);
        }
      }
    }
    const project = p.project && typeof p.project === 'object' ? p.project : null;
    return this.deps.linear('importIssues', { issues: items, project, sync: p.sync !== false });
  }

  // linear.link: ties a task to an issue that already exists.
  async link(p: { taskId?: unknown; issue?: unknown; sync?: unknown }) {
    this.gate();
    const task = await this.task(p.taskId);
    const key = typeof p.issue === 'string' ? issueKey(p.issue) : undefined;
    if (!key) throw new Error('Paste an issue key like ENG-123, or its Linear link');
    const row = await this.fetchIssue(key).catch((err: unknown) => {
      throw err instanceof LinearError && err.notFound ? new Error(`Couldn’t find ${key} in Linear`) : err;
    });
    const { teams } = await this.options();
    await this.deps.linear('link', {
      id: task.id,
      issue: snapOf(row),
      teamId: row.teamId,
      teamName: teams.find((t) => t.id === row.teamId)?.displayName ?? row.team,
      ...(row.projectId ? { projectId: row.projectId, projectName: row.project ?? '' } : {}),
      sync: p.sync !== false,
    });
    return { identifier: row.identifier };
  }

  // linear.setSync / linear.unlink: only Convex changes; Linear is never touched.
  async setSync(p: { taskId?: unknown; sync?: unknown }) {
    if (typeof p.taskId !== 'string' || typeof p.sync !== 'boolean') throw new Error('Choose a task');
    await this.deps.linear('setSync', { id: p.taskId, sync: p.sync });
    // Turning sync on catches up straight away.
    if (p.sync && !this.why()) await this.refresh({ taskId: p.taskId }).catch(() => undefined);
    return null;
  }
  async unlink(p: { taskId?: unknown }) {
    if (typeof p.taskId !== 'string') throw new Error('Choose a task');
    return this.deps.linear('unlink', { id: p.taskId });
  }

  // ---- the sync sweep: one list_issues per team asks what changed since the
  // last sweep; only those issues are read again. Nothing runs without synced tasks.
  start() {
    this.stop();
    this.timer = setInterval(() => void this.sweep().catch((err: unknown) => this.deps.log?.warn(`linear: sync failed: ${err instanceof Error ? err.message : String(err)}`)), SWEEP_TICK_MS);
    this.timer.unref?.();
    setTimeout(() => void this.sweep().catch(() => undefined), 60_000).unref?.();
  }
  stop() {
    if (this.timer) clearInterval(this.timer);
    this.timer = undefined;
  }
  async sweep(force = false): Promise<{ checked: number; changed: number } | undefined> {
    if (this.why()) return undefined;
    if (this.sweeping) return undefined;
    const run = (async () => {
      const synced = (await this.deps.linear('synced', {})) as { id: string; issueId: string; identifier: string; teamId: string; issueUpdatedAt?: string }[];
      if (!synced.length) return { checked: 0, changed: 0 };
      const claim = (await this.deps.linear('claimSweep', force ? { force: true } : {})) as { since: number | null } | null;
      if (!claim) return undefined;
      let due = synced;
      if (claim.since !== null) {
        const since = new Date(claim.since - 60_000).toISOString();
        const changed = new Set<string>();
        const byTeam = new Map<string, number>();
        for (const s of synced) byTeam.set(s.teamId, (byTeam.get(s.teamId) ?? 0) + 1);
        for (const teamId of byTeam.keys()) {
          let cursor: string | undefined;
          for (let page = 0; page < 4; page++) {
            const r = await this.json('linear__list_issues', { team: teamId, updatedAt: since, includeArchived: true, limit: 250, fields: ['id', 'uuid', 'updatedAt'], ...(cursor ? { cursor } : {}) });
            for (const x of (Array.isArray(r.issues) ? r.issues : []) as Json[]) if (str(x.uuid)) changed.add(str(x.uuid)!);
            cursor = r.hasNextPage ? str(r.cursor) : undefined;
            if (!cursor) break;
          }
        }
        due = synced.filter((s) => changed.has(s.issueId));
      } else due = synced.slice(0, 50);
      let changedCount = 0;
      for (let i = 0; i < due.length; i += AT_ONCE) {
        const out = await Promise.allSettled(due.slice(i, i + AT_ONCE).map((s) => this.refreshOne(s.id, { issueId: s.issueId, identifier: s.identifier, sync: true })));
        for (const o of out) if (o.status === 'fulfilled' && ((o.value as { changed?: string[] }).changed?.length ?? 0) > 0) changedCount++;
      }
      return { checked: due.length, changed: changedCount };
    })();
    this.sweeping = run;
    try {
      return await run;
    } finally {
      this.sweeping = undefined;
    }
  }
}

// The linear.* RPCs the apps call.
export function linearMethods(l: LinearTasks): Record<string, (p: Record<string, unknown>) => unknown> {
  return {
    'linear.status': () => l.status(),
    'linear.options': (p) => l.options(p as { refresh?: boolean }),
    'linear.projects': (p) => l.projects(p),
    'linear.preview': (p) => l.preview(p),
    'linear.send': (p) => l.send(p),
    'linear.refresh': (p) => l.refresh(p),
    'linear.search': (p) => l.search(p),
    'linear.import': (p) => l.import(p),
    'linear.link': (p) => l.link(p),
    'linear.sync': () => l.sweep(true),
    'linear.setSync': (p) => l.setSync(p),
    'linear.unlink': (p) => l.unlink(p),
  };
}

// The Threadbear login's helper: the main login's headers.sh, asked for the
// threadbear agent's token instead (runner.json connectors.linearHeadersHelper wins).
export function linearHelper(mainHelper: string, override?: string): string | undefined {
  if (override?.trim()) return override.trim();
  return helperForAgent(mainHelper, 'threadbear');
}
export { helperHeaders };
