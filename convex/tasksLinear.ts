// API 105: Native Tasks and Linear. The runner talks to Linear (through the
// Connectors gateway, as its own login); this file keeps what it learned on
// the task, and is the one place that decides what a Linear change does here.
//
// - Sending a task makes one issue. claim is the lock: a double click, a
//   second Mac or a retry after a crash can't make two. settle only takes the
//   answer for the claim it belongs to.
// - One Linear issue belongs to at most one task (by_linear_issue).
// - Sync only ever goes from Linear to here, and only for tasks with sync on.
//   A field changes only when it changed in Linear since the last sync, so a
//   local edit stays until Linear changes that same field. Notes never change.
import { v, type Infer } from 'convex/values';
import { mutation, query, type MutationCtx } from './_generated/server';
import type { Doc } from './_generated/dataModel';
import { taskLinear, taskProject, type TaskStatus } from './taskFields';
import { authField, authenticate, bad, boardOf, changedAt, columnFields, find, log, prefRow, projectExists, text, title, type Auth } from './tasks';

type Linear = Infer<typeof taskLinear>;
const LINEAR = { kind: 'agent' as const, name: 'Linear' };
const STALE_CLAIM_MS = 10 * 60 * 1000;
const LINK_ID = 'linear-issue';
const SYNC_MS = { '1h': 3600_000, '6h': 6 * 3600_000, '12h': 12 * 3600_000, '1d': 24 * 3600_000 } as const;

// What the runner read from Linear about one issue.
export const issueSnap = v.object({
  issueId: v.string(), // Linear's uuid
  identifier: v.string(), // ENG-123
  url: v.string(),
  title: v.string(),
  priority: v.number(), // 0 none, 1 urgent, 2 high, 3 medium, 4 low
  dueDate: v.union(v.string(), v.null()),
  status: v.string(),
  statusType: v.string(), // triage, backlog, unstarted, started, completed, canceled
  updatedAt: v.string(),
});
type Snap = Infer<typeof issueSnap>;
const commentSnap = v.object({ id: v.string(), body: v.string(), author: v.string(), createdAt: v.string() });
type Comment = Infer<typeof commentSnap>;

async function runnerOnly(ctx: MutationCtx, auth: Auth) {
  await authenticate(ctx, auth);
  if (!('key' in auth)) bad('Only the Mac can do that');
}
async function live(ctx: MutationCtx, id: string) {
  const row = await find(ctx, id);
  if (!row || row.deleted) bad('Task no longer exists');
  return row;
}
function priorityOf(n: number): Doc<'tasks'>['priority'] {
  return n === 1 ? 'urgent' : n === 2 ? 'high' : n === 3 ? 'normal' : n === 4 ? 'low' : null;
}
function cleanDue(value: string | null): string | null {
  return value && /^\d{4}-\d{2}-\d{2}$/.test(value) ? value : null;
}
// Linear's state types to a board column. Canceled goes to a Canceled
// column when the board has one (it's done for completion), else Done.
async function placeFor(ctx: MutationCtx, statusType: string, current?: { columnId?: string; status: TaskStatus }) {
  const status: TaskStatus = statusType === 'completed' || statusType === 'canceled' ? 'done' : statusType === 'started' ? 'doing' : statusType === 'unstarted' ? 'todo' : 'backlog';
  if (statusType === 'canceled') {
    const board = await boardOf(ctx);
    const canceled = board.columns.find((c) => c.id === 'canceled' && c.status === 'done');
    if (canceled) return { status: 'done' as TaskStatus, columnId: canceled.id };
  }
  return columnFields(ctx, undefined, status, current);
}
async function owner(ctx: MutationCtx, issueId: string) {
  return ctx.db
    .query('tasks')
    .withIndex('by_linear_issue', (q) => q.eq('linear.issueId', issueId))
    .first();
}
const last = (s: Snap): NonNullable<Linear['last']> => ({ title: s.title, priority: s.priority, dueDate: cleanDue(s.dueDate), statusType: s.statusType });
function issueFields(s: Snap) {
  return { issueId: s.issueId, identifier: s.identifier, url: s.url, status: s.status, statusType: s.statusType, issueUpdatedAt: s.updatedAt, syncedAt: Date.now(), last: last(s) };
}
// The issue shows in the task's Links too, so every client (the web, older
// Macs) can open it without knowing about the linear field.
function withLink(links: Doc<'tasks'>['links'], s: Snap) {
  const rest = (links ?? []).filter((l) => l.id !== LINK_ID);
  if (rest.length >= 50) return links;
  return [...rest, { id: LINK_ID, kind: 'link' as const, title: `${s.identifier} in Linear`, url: s.url }];
}
async function addComments(ctx: MutationCtx, row: Doc<'tasks'>, comments: Comment[]) {
  let added = 0;
  for (const c of comments.slice(0, 100)) {
    if (!c.id || c.id.length > 100) continue;
    const seen = await ctx.db
      .query('taskActivity')
      .withIndex('by_external', (q) => q.eq('externalId', c.id))
      .first();
    if (seen) continue;
    const body = c.body.trim().slice(0, 10000);
    if (!body) continue;
    const at = Date.parse(c.createdAt);
    await ctx.db.insert('taskActivity', {
      taskId: row.id,
      kind: 'comment',
      body,
      author: { kind: 'user', name: `${c.author.trim().slice(0, 60) || 'Someone'} in Linear` },
      createdAt: Number.isFinite(at) ? at : Date.now(),
      source: 'linear',
      externalId: c.id,
    });
    added++;
  }
  return added;
}

// The lock for a send. Returns the pictures an earlier try already uploaded.
export const claim = mutation({
  args: {
    auth: authField,
    id: v.string(),
    requestId: v.string(),
    teamId: v.string(),
    teamName: v.string(),
    projectId: v.optional(v.string()),
    projectName: v.optional(v.string()),
    sync: v.boolean(),
  },
  returns: v.object({ uploads: v.array(v.object({ storageId: v.id('_storage'), assetUrl: v.string() })) }),
  handler: async (ctx, a) => {
    await runnerOnly(ctx, a.auth);
    const row = await live(ctx, a.id);
    if (row.status === 'done') bad('This task is done: reopen it to send it to Linear');
    if (!/^[\w-]{8,100}$/.test(a.requestId)) bad('Invalid request');
    const was = row.linear;
    if (was?.state === 'linked') bad(`Already in Linear as ${was.identifier ?? 'an issue'}`);
    if (was?.state === 'creating' && was.requestId !== a.requestId && Date.now() - was.claimedAt < STALE_CLAIM_MS) bad('This task is already being sent to Linear');
    const linear: Linear = {
      state: 'creating',
      requestId: a.requestId,
      claimedAt: Date.now(),
      teamId: text(a.teamId, 100, 'Team'),
      teamName: text(a.teamName, 200, 'Team name'),
      ...(a.projectId ? { projectId: text(a.projectId, 100, 'Project'), projectName: text(a.projectName ?? '', 200, 'Project name') } : {}),
      sync: a.sync,
      ...(was?.uploads ? { uploads: was.uploads } : {}),
    };
    await ctx.db.patch('tasks', row._id, { linear, updatedAt: await changedAt(ctx) });
    return { uploads: was?.uploads ?? [] };
  },
});

async function mine(ctx: MutationCtx, id: string, requestId: string) {
  const row = await find(ctx, id);
  if (!row?.linear || row.linear.requestId !== requestId || row.linear.state !== 'creating') return null;
  return row as Doc<'tasks'> & { linear: Linear };
}
export const uploaded = mutation({
  args: { auth: authField, id: v.string(), requestId: v.string(), storageId: v.id('_storage'), assetUrl: v.string() },
  returns: v.null(),
  handler: async (ctx, a) => {
    await runnerOnly(ctx, a.auth);
    const row = await mine(ctx, a.id, a.requestId);
    if (!row || !/^https:\/\/\S{1,2000}$/.test(a.assetUrl)) return null;
    const uploads = [...(row.linear.uploads ?? []).filter((u) => u.storageId !== a.storageId), { storageId: a.storageId, assetUrl: a.assetUrl }].slice(-20);
    await ctx.db.patch('tasks', row._id, { linear: { ...row.linear, uploads } });
    return null;
  },
});
export const waiting = mutation({
  args: { auth: authField, id: v.string(), requestId: v.string(), url: v.string() },
  returns: v.null(),
  handler: async (ctx, a) => {
    await runnerOnly(ctx, a.auth);
    const row = await mine(ctx, a.id, a.requestId);
    if (!row || !/^https?:\/\/\S{1,2000}$/.test(a.url)) return null;
    await ctx.db.patch('tasks', row._id, { linear: { ...row.linear, waitingUrl: a.url, claimedAt: Date.now() }, updatedAt: await changedAt(ctx) });
    return null;
  },
});

// The answer to a send: the issue it made (or found), or why it failed.
export const settle = mutation({
  args: { auth: authField, id: v.string(), requestId: v.string(), issue: v.optional(issueSnap), error: v.optional(v.string()), found: v.optional(v.boolean()) },
  returns: v.null(),
  handler: async (ctx, a) => {
    await runnerOnly(ctx, a.auth);
    const row = await mine(ctx, a.id, a.requestId);
    if (!row) return null;
    const { waitingUrl: _w, error: _e, ...base } = row.linear;
    const now = await changedAt(ctx);
    if (!a.issue) {
      await ctx.db.patch('tasks', row._id, { linear: { ...base, state: 'failed', error: text((a.error ?? 'Linear gave an error').slice(0, 500), 500, 'Error') }, updatedAt: now });
      return null;
    }
    const other = await owner(ctx, a.issue.issueId);
    if (other && other.id !== row.id) {
      await ctx.db.patch('tasks', row._id, { linear: { ...base, state: 'failed', error: `${a.issue.identifier} already belongs to another task` }, updatedAt: now });
      return null;
    }
    await ctx.db.patch('tasks', row._id, { linear: { ...base, state: 'linked', ...issueFields(a.issue) }, links: withLink(row.links, a.issue), updatedAt: now });
    await log(ctx, row.id, LINEAR, a.found ? `linked to ${a.issue.identifier}, which an earlier send had made` : `sent to Linear as ${a.issue.identifier}`);
    return null;
  },
});

// Links a task to an issue that already exists. Nothing changes in Linear,
// and the task keeps its own title, notes and fields until Linear changes one.
export const link = mutation({
  args: { auth: authField, id: v.string(), issue: issueSnap, teamId: v.string(), teamName: v.string(), projectId: v.optional(v.string()), projectName: v.optional(v.string()), sync: v.boolean() },
  returns: v.null(),
  handler: async (ctx, a) => {
    await runnerOnly(ctx, a.auth);
    const row = await live(ctx, a.id);
    if (row.linear?.state === 'linked') bad(`Already linked to ${row.linear.identifier ?? 'an issue'}`);
    if (row.linear?.state === 'creating' && Date.now() - row.linear.claimedAt < STALE_CLAIM_MS) bad('This task is being sent to Linear');
    const other = await owner(ctx, a.issue.issueId);
    if (other && other.id !== row.id && !other.deleted) bad(`${a.issue.identifier} is already linked to “${other.title}”`);
    const linear: Linear = {
      state: 'linked',
      requestId: `link-${a.issue.issueId}`.slice(0, 100),
      claimedAt: Date.now(),
      teamId: a.teamId,
      teamName: a.teamName,
      ...(a.projectId ? { projectId: a.projectId, projectName: a.projectName ?? '' } : {}),
      sync: a.sync,
      ...issueFields(a.issue),
    };
    await ctx.db.patch('tasks', row._id, { linear, links: withLink(row.links, a.issue), updatedAt: await changedAt(ctx) });
    await log(ctx, row.id, LINEAR, `linked to ${a.issue.identifier}`);
    return null;
  },
});

// Picked issues become tasks, already linked. Parents come before their
// sub-issues in the list; a sub-issue whose parent isn't here stands alone.
export const importIssues = mutation({
  args: {
    auth: authField,
    project: v.union(taskProject, v.null()),
    sync: v.boolean(),
    issues: v.array(
      v.object({
        issue: issueSnap,
        description: v.string(),
        parentIssueId: v.optional(v.string()),
        teamId: v.string(),
        teamName: v.string(),
        projectId: v.optional(v.string()),
        projectName: v.optional(v.string()),
        comments: v.array(commentSnap),
      }),
    ),
  },
  returns: v.object({ created: v.array(v.string()), skipped: v.array(v.string()) }),
  handler: async (ctx, a) => {
    await runnerOnly(ctx, a.auth);
    if (a.issues.length > 50) bad('Add at most 50 issues at a time');
    await projectExists(ctx, a.project);
    const created: string[] = [];
    const skipped: string[] = [];
    for (const item of a.issues) {
      const s = item.issue;
      const id = `linear-${s.issueId.replace(/[^\w-]/g, '').slice(0, 80)}`;
      const had = await owner(ctx, s.issueId);
      if ((had && !had.deleted) || (await find(ctx, id))) {
        skipped.push(s.identifier);
        continue;
      }
      const parentId = item.parentIssueId ? (await owner(ctx, item.parentIssueId))?.id : undefined;
      const parent = parentId ? await find(ctx, parentId) : null;
      const now = await changedAt(ctx);
      const priority = priorityOf(s.priority);
      const due = cleanDue(s.dueDate);
      const linear: Linear = {
        state: 'linked',
        requestId: `import-${s.issueId}`.slice(0, 100),
        claimedAt: Date.now(),
        teamId: item.teamId,
        teamName: item.teamName,
        ...(item.projectId ? { projectId: item.projectId, projectName: item.projectName ?? '' } : {}),
        sync: a.sync,
        ...issueFields(s),
      };
      const _id = await ctx.db.insert('tasks', {
        id,
        title: title(s.title.slice(0, 500)),
        notes: item.description.slice(0, 20000),
        ...(await placeFor(ctx, s.statusType)),
        project: a.project,
        position: now,
        createdAt: now,
        updatedAt: now,
        deleted: false,
        ...(priority ? { priority } : {}),
        ...(due ? { dueDate: due } : {}),
        ...(parent && !parent.deleted && !parent.parentId ? { parentId: parent.id } : {}),
        links: withLink(undefined, s),
        createdBy: LINEAR,
        linear,
      });
      await log(ctx, id, LINEAR, `added from ${s.identifier}`);
      const row = await ctx.db.get('tasks', _id);
      const added = row ? await addComments(ctx, row, item.comments) : 0;
      if (row && added) await ctx.db.patch('tasks', row._id, { commentCount: added });
      created.push(s.identifier);
    }
    return { created, skipped };
  },
});

// What Linear says now. Always refreshes the chip; with sync on, copies what
// changed in Linear since last time onto the task.
export const apply = mutation({
  args: { auth: authField, id: v.string(), issue: issueSnap, comments: v.optional(v.array(commentSnap)) },
  returns: v.object({ changed: v.array(v.string()) }),
  handler: async (ctx, a) => {
    await runnerOnly(ctx, a.auth);
    const row = await find(ctx, a.id);
    const was = row?.linear;
    if (!row || row.deleted || !was || was.state !== 'linked' || was.issueId !== a.issue.issueId) return { changed: [] };
    const s = a.issue;
    const fresh = !was.issueUpdatedAt || s.updatedAt > was.issueUpdatedAt;
    const changed: string[] = [];
    const patch: Partial<Doc<'tasks'>> = {};
    if (fresh && was.sync && was.last) {
      const before = was.last;
      if (s.title !== before.title && s.title.trim()) {
        patch.title = title(s.title.slice(0, 500));
        changed.push(`renamed this to ${patch.title}`);
      }
      if (s.priority !== before.priority) {
        patch.priority = priorityOf(s.priority);
        changed.push(patch.priority ? `set priority to ${patch.priority[0]!.toUpperCase()}${patch.priority.slice(1)}` : 'removed the priority');
      }
      const due = cleanDue(s.dueDate);
      if (due !== before.dueDate) {
        patch.dueDate = due;
        if (!due) patch.dueTime = null;
        changed.push(due ? `set the due date to ${due}` : 'removed the due date');
      }
      if (s.statusType !== before.statusType) {
        Object.assign(patch, await placeFor(ctx, s.statusType, row));
        changed.push(s.statusType === 'canceled' ? 'canceled this in Linear' : `changed status to ${s.status}`);
      }
    }
    const { missing: _m, ...base } = was;
    const linear: Linear = fresh ? { ...base, ...issueFields(s) } : { ...base, syncedAt: Date.now() };
    const added = was.sync && a.comments ? await addComments(ctx, row, a.comments) : 0;
    await ctx.db.patch('tasks', row._id, {
      ...patch,
      linear,
      ...(fresh ? { links: withLink(row.links, s) } : {}),
      ...(added ? { commentCount: (row.commentCount ?? 0) + added } : {}),
      updatedAt: await changedAt(ctx),
    });
    for (const body of changed) await log(ctx, row.id, LINEAR, body);
    if (added) changed.push(`${added} comment${added === 1 ? '' : 's'}`);
    return { changed };
  },
});

// The issue is gone (deleted, or this login can't see it): stop syncing.
export const missing = mutation({
  args: { auth: authField, id: v.string(), issueId: v.string() },
  returns: v.null(),
  handler: async (ctx, a) => {
    await runnerOnly(ctx, a.auth);
    const row = await find(ctx, a.id);
    if (!row?.linear || row.linear.issueId !== a.issueId || row.linear.missing) return null;
    await ctx.db.patch('tasks', row._id, { linear: { ...row.linear, missing: true, sync: false, syncedAt: Date.now() }, updatedAt: await changedAt(ctx) });
    await log(ctx, row.id, LINEAR, `couldn't find ${row.linear.identifier ?? 'the issue'} in Linear, so stopped syncing`);
    return null;
  },
});

export const setSync = mutation({
  args: { auth: authField, id: v.string(), sync: v.boolean() },
  returns: v.null(),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const row = await live(ctx, a.id);
    if (!row.linear || row.linear.state !== 'linked' || row.linear.sync === a.sync) return null;
    // Turning sync back on starts from what Linear says next, not from old changes.
    await ctx.db.patch('tasks', row._id, { linear: { ...row.linear, sync: a.sync, ...(a.sync ? { missing: undefined } : {}) }, updatedAt: await changedAt(ctx) });
    return null;
  },
});

// Forgets the issue here. Never touches Linear.
export const unlink = mutation({
  args: { auth: authField, id: v.string() },
  returns: v.null(),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const row = await live(ctx, a.id);
    if (!row.linear) return null;
    if (row.linear.state === 'creating' && Date.now() - row.linear.claimedAt < STALE_CLAIM_MS && !row.linear.waitingUrl) bad('Wait for the send to finish first');
    const name = row.linear.identifier;
    await ctx.db.patch('tasks', row._id, { linear: undefined, links: (row.links ?? []).filter((l) => l.id !== LINK_ID), updatedAt: await changedAt(ctx) });
    if (name) await log(ctx, row.id, LINEAR, `unlinked from ${name}`);
    return null;
  },
});

// The tasks a sync looks at: linked, sync on, not deleted.
export const synced = query({
  args: { auth: authField },
  returns: v.array(v.object({ id: v.string(), issueId: v.string(), identifier: v.string(), teamId: v.string(), issueUpdatedAt: v.optional(v.string()) })),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const rows = await ctx.db
      .query('tasks')
      .withIndex('by_linear_sync', (q) => q.eq('linear.sync', true).eq('deleted', false))
      .take(500);
    return rows.flatMap((r) =>
      r.linear?.state === 'linked' && r.linear.issueId ? [{ id: r.id, issueId: r.linear.issueId, identifier: r.linear.identifier ?? '', teamId: r.linear.teamId, ...(r.linear.issueUpdatedAt ? { issueUpdatedAt: r.linear.issueUpdatedAt } : {}) }] : [],
    );
  },
});

// Which of these issues already have a task, for the picker's "In Tasks".
export const linkedIssues = query({
  args: { auth: authField, issueIds: v.array(v.string()) },
  returns: v.array(v.object({ issueId: v.string(), taskId: v.string(), title: v.string() })),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    if (a.issueIds.length > 250) bad('Ask about at most 250 issues');
    const out = [];
    for (const issueId of a.issueIds) {
      const row = await ctx.db
        .query('tasks')
        .withIndex('by_linear_issue', (q) => q.eq('linear.issueId', issueId))
        .first();
      if (row && !row.deleted) out.push({ issueId, taskId: row.id, title: row.title });
    }
    return out;
  },
});

// One Mac runs each sync: the first to ask after the interval gets the time
// the last one ran (to ask Linear what changed since), everyone else null.
export const claimSweep = mutation({
  args: { auth: authField, force: v.optional(v.boolean()) },
  returns: v.union(v.object({ since: v.union(v.number(), v.null()) }), v.null()),
  handler: async (ctx, a) => {
    await runnerOnly(ctx, a.auth);
    const prefs = await prefRow(ctx);
    const settings = prefs?.linear ?? { syncEvery: '1h' as const, defaults: [] };
    const now = Date.now();
    const lastAt = settings.lastSweepAt;
    if (!a.force && lastAt !== undefined && now - lastAt < SYNC_MS[settings.syncEvery]) return null;
    // Two forced sweeps a minute apart are one sweep.
    if (a.force && lastAt !== undefined && now - lastAt < 60_000) return null;
    const linear = { ...settings, lastSweepAt: now };
    if (prefs) await ctx.db.patch('taskPreferences', prefs._id, { linear });
    else await ctx.db.insert('taskPreferences', { name: 'tasks', linear });
    return { since: lastAt ?? null };
  },
});

