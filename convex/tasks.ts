// One private task collection per Threadbear deployment. Paired browsers write
// directly so changing a task never waits for a sleeping Mac.
import { ConvexError, v } from 'convex/values';
import { mutation, query, type QueryCtx, type MutationCtx } from './_generated/server';
import { checkPhoneToken, runnerAuth } from './lib';
import { mainRunner, queue } from './phone';
import { fullColumnOrder, taskStatuses, listStatuses, taskActivityKind, taskAuthor, taskFields, taskLink, taskLinkOut, taskPreferenceFields, taskPriority, taskProject, taskStatus } from './taskFields';
import type { Infer } from 'convex/values';
import type { Id } from './_generated/dataModel';

const authField = v.union(v.object({ token: v.string() }), v.object({ key: v.string() }));
type Auth = Infer<typeof authField>;
type Ctx = QueryCtx | MutationCtx;
async function authenticate(ctx: Ctx, auth: Auth) {
  if ('token' in auth) await checkPhoneToken(ctx, auth.token);
  else await runnerAuth(ctx, auth.key, 'nativeTasks');
}
function bad(message: string): never {
  throw new ConvexError({ kind: 'badArgs', message });
}
function text(value: string, max: number, name: string) {
  if (value.length > max) bad(`${name} is too long`);
  return value;
}
function title(value: string) {
  const clean = text(value.trim(), 500, 'Title');
  if (!clean) bad('Give the task a title');
  return clean;
}
async function changedAt(ctx: Ctx): Promise<number> {
  const last = await ctx.db.query('tasks').withIndex('by_updated').order('desc').first();
  return Math.max(Date.now(), (last?.updatedAt ?? 0) + 1);
}
const find = (ctx: Ctx, id: string) =>
  ctx.db
    .query('tasks')
    .withIndex('by_task_id', (q) => q.eq('id', id))
    .unique();
async function projectExists(ctx: Ctx, project: Infer<typeof taskProject> | null | undefined) {
  if (
    project &&
    !(await ctx.db
      .query('projects')
      .withIndex('by_runner_id', (q) => q.eq('runnerId', project.runnerId).eq('id', project.id))
      .unique())
  )
    bad('That project is no longer available');
}
const taskFieldsValidator = v.object(taskFields);
// What clients see: image links carry their URL, and deletedWithParent stays inside.
const { deletedWithParent: _deletedWithParent, links: _links, ...listedFields } = taskFields;
const taskOut = v.object({ ...listedFields, links: v.optional(v.array(taskLinkOut)) });
async function expose(ctx: Ctx, row: Infer<typeof taskFieldsValidator>): Promise<Infer<typeof taskOut>> {
  const { deletedWithParent: _hidden, links, ...rest } = row;
  if (!links) return rest;
  return {
    ...rest,
    links: await Promise.all(
      links.map(async (l) => {
        const url = l.kind === 'image' && l.storageId ? await ctx.storage.getUrl(l.storageId) : null;
        return url ? { ...l, imageUrl: url } : l;
      }),
    ),
  };
}

// ---- API 87: priority, due date, sub-tasks, links, comments and activity.
type Author = Infer<typeof taskAuthor>;
type Link = Infer<typeof taskLink>;
const PRIORITY_NAMES = { urgent: 'Urgent', high: 'High', normal: 'Normal', low: 'Low' } as const;
const STATUS_NAMES: Record<string, string> = { backlog: 'Backlog', todo: 'Todo', doing: 'Doing', done: 'Done' };
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const IMAGE_TYPES = ['image/jpeg', 'image/png', 'image/webp', 'image/gif'];
const MAX_LINKS = 50;
function dueDate(value: string | null | undefined) {
  if (value === undefined || value === null) return value;
  const day = new Date(`${value}T00:00:00Z`);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value) || Number.isNaN(day.getTime()) || day.toISOString().slice(0, 10) !== value) bad('Use a due date like 2026-11-20');
  return value;
}
const dayLabel = (value: string) => {
  const [y, m, d] = value.split('-').map(Number);
  return `${MONTHS[m! - 1]} ${d}, ${y}`;
};
// Who did it. A runner key says (the Mac app as the user, an agent's tool as
// the agent); a browser is always the user, by the name the runner last gave.
async function actor(ctx: MutationCtx, auth: Auth, given: Author | undefined): Promise<Author> {
  const prefs = await prefRow(ctx);
  if ('key' in auth && given) {
    const name = given.name.trim().slice(0, 80) || (given.kind === 'agent' ? 'Agent' : 'You');
    if (given.kind === 'user' && name !== prefs?.ownerName) {
      if (prefs) await ctx.db.patch(prefs._id, { ownerName: name });
      else await ctx.db.insert('taskPreferences', { name: 'tasks', ownerName: name });
    }
    return { kind: given.kind, name };
  }
  return { kind: 'user', name: prefs?.ownerName ?? 'You' };
}
async function log(ctx: MutationCtx, taskId: string, author: Author, body: string, kind: 'event' | 'comment' = 'event') {
  const createdAt = Date.now();
  const _id = await ctx.db.insert('taskActivity', { taskId, kind, body, author, createdAt });
  return { id: _id as string, taskId, kind, body, author, createdAt };
}
async function checkParent(ctx: Ctx, id: string, parentId: string | null | undefined) {
  if (parentId === undefined || parentId === null) return null;
  if (parentId === id) bad('A task cannot be its own sub-task');
  const parent = await find(ctx, parentId);
  if (!parent || parent.deleted) bad('That parent task no longer exists');
  if (parent.parentId) bad('A sub-task cannot have sub-tasks');
  const child = await ctx.db
    .query('tasks')
    .withIndex('by_parent', (q) => q.eq('parentId', id).eq('deleted', false))
    .first();
  if (child) bad('A task with sub-tasks cannot become a sub-task');
  return parent;
}
async function cleanLinks(ctx: Ctx, links: Link[], before: Link[] = []): Promise<Link[]> {
  if (links.length > MAX_LINKS) bad(`Use at most ${MAX_LINKS} links`);
  const known = new Set(before.map((l) => l.storageId).filter(Boolean));
  const ids = new Set<string>();
  const out: Link[] = [];
  for (const l of links) {
    if (!/^[\w-]{1,100}$/.test(l.id) || ids.has(l.id)) bad('Use a unique link id');
    ids.add(l.id);
    if (l.url !== undefined && (l.url.length > 2000 || !/^https?:\/\/\S+$/i.test(l.url))) bad('Use an http or https link');
    const link: Link = { id: l.id, kind: l.kind, title: '' };
    if (l.url) link.url = l.url;
    if (l.kind === 'link' && !l.url) bad('Give the link a URL');
    if (l.kind === 'print') {
      if (!l.printId || l.printId.length > 200) bad('Choose a print');
      link.printId = l.printId;
    }
    if (l.kind === 'thread') {
      if (!l.thread || l.thread.id.length > 200 || l.thread.runnerId.length > 200) bad('Choose a thread');
      link.thread = { runnerId: l.thread.runnerId, id: l.thread.id };
    }
    if (l.kind === 'image') {
      if (!l.storageId) bad('Choose an image');
      if (!known.has(l.storageId)) await checkImage(ctx, l.storageId);
      link.storageId = l.storageId;
    }
    link.title = text(l.title.trim(), 300, 'Link title') || l.url || { link: 'Link', print: 'Print', image: 'Image', thread: 'Thread' }[l.kind];
    out.push(link);
  }
  return out;
}
async function checkImage(ctx: Ctx, storageId: Id<'_storage'>) {
  const file = await ctx.db.system.get(storageId);
  if (!file || file.size > 10 * 1024 * 1024 || !IMAGE_TYPES.includes(file.contentType ?? '')) bad('Choose a PNG, JPEG, WebP or GIF under 10 MB');
}
// Images a links change dropped go from storage too.
async function dropImages(ctx: MutationCtx, before: Link[] | undefined, after: Link[]) {
  const kept = new Set(after.map((l) => l.storageId));
  for (const l of before ?? []) if (l.storageId && !kept.has(l.storageId)) await ctx.storage.delete(l.storageId);
}
async function columnName(ctx: Ctx, columnId: string) {
  return (await prefRow(ctx))?.columns?.find((c) => c.id === columnId)?.name ?? STATUS_NAMES[columnId] ?? columnId;
}
async function projectTitle(ctx: Ctx, project: Infer<typeof taskProject>) {
  const row = await ctx.db
    .query('projects')
    .withIndex('by_runner_id', (q) => q.eq('runnerId', project.runnerId).eq('id', project.id))
    .unique();
  return row?.title ?? 'a project';
}

export const list = query({
  args: {
    auth: authField,
    after: v.optional(v.string()),
    limit: v.optional(v.number()),
  },
  returns: v.object({
    items: v.array(taskOut),
    next: v.union(v.string(), v.null()),
  }),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    if (a.limit !== undefined && !Number.isFinite(a.limit)) bad('Invalid page size');
    const limit = Math.min(250, Math.max(1, Math.floor(a.limit ?? 100)));
    const rows = await ctx.db
      .query('tasks')
      .withIndex('by_deleted_id', (q) => q.eq('deleted', false).gt('id', a.after ?? ''))
      .take(limit + 1);
    const items = await Promise.all(
      rows.slice(0, limit).map(async ({ _id, _creationTime, ...row }) => {
        // A removed project does not take its tasks with it.
        if (
          row.project &&
          !(await ctx.db
            .query('projects')
            .withIndex('by_runner_id', (q) => q.eq('runnerId', row.project!.runnerId).eq('id', row.project!.id))
            .unique())
        )
          row.project = null;
        return expose(ctx, row);
      }),
    );
    return {
      items,
      next: rows.length > limit ? items[items.length - 1]!.id : null,
    };
  },
});
export const create = mutation({
  args: {
    auth: authField,
    id: v.string(),
    title: v.string(),
    notes: v.optional(v.string()),
    status: v.optional(taskStatus),
    columnId: v.optional(v.string()),
    project: v.optional(v.union(taskProject, v.null())),
    // Where it sorts; left out, it goes last. API 84.
    position: v.optional(v.number()),
    // API 87.
    priority: v.optional(v.union(taskPriority, v.null())),
    dueDate: v.optional(v.union(v.string(), v.null())),
    parentId: v.optional(v.union(v.string(), v.null())),
    links: v.optional(v.array(taskLink)),
    author: v.optional(taskAuthor),
  },
  returns: v.string(),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    if (!/^[\w-]{1,100}$/.test(a.id)) bad('Invalid task id');
    if (await find(ctx, a.id)) return a.id;
    await projectExists(ctx, a.project);
    const parent = await checkParent(ctx, a.id, a.parentId);
    const links = a.links ? await cleanLinks(ctx, a.links) : undefined;
    const author = await actor(ctx, a.auth, a.author);
    const now = await changedAt(ctx);
    const clean = title(a.title);
    await ctx.db.insert('tasks', {
      id: a.id,
      title: clean,
      notes: text(a.notes ?? '', 20000, 'Notes'),
      ...(await columnFields(ctx, a.columnId, a.status)),
      project: a.project ?? null,
      position: a.position !== undefined && Number.isFinite(a.position) ? a.position : now,
      createdAt: now,
      updatedAt: now,
      deleted: false,
      ...(a.priority ? { priority: a.priority } : {}),
      ...(a.dueDate ? { dueDate: dueDate(a.dueDate) } : {}),
      ...(parent ? { parentId: parent.id } : {}),
      ...(links?.length ? { links } : {}),
      createdBy: author,
    });
    await log(ctx, a.id, author, 'created the task');
    if (parent) await log(ctx, parent.id, author, `added sub-task ${clean}`);
    return a.id;
  },
});
export const update = mutation({
  args: {
    auth: authField,
    id: v.string(),
    title: v.optional(v.string()),
    notes: v.optional(v.string()),
    status: v.optional(taskStatus),
    columnId: v.optional(v.string()),
    project: v.optional(v.union(taskProject, v.null())),
    position: v.optional(v.number()),
    // API 87. links replaces the whole array.
    priority: v.optional(v.union(taskPriority, v.null())),
    dueDate: v.optional(v.union(v.string(), v.null())),
    parentId: v.optional(v.union(v.string(), v.null())),
    links: v.optional(v.array(taskLink)),
    author: v.optional(taskAuthor),
  },
  returns: v.null(),
  handler: async (ctx, { auth, id, author: given, ...patch }) => {
    await authenticate(ctx, auth);
    const row = await find(ctx, id);
    if (!row || row.deleted) bad('Task no longer exists');
    await projectExists(ctx, patch.project);
    if (patch.title !== undefined) patch.title = title(patch.title);
    if (patch.notes !== undefined) text(patch.notes, 20000, 'Notes');
    if (patch.position !== undefined && !Number.isFinite(patch.position)) bad('Invalid position');
    dueDate(patch.dueDate);
    const parentChanged = patch.parentId !== undefined && (patch.parentId ?? null) !== (row.parentId ?? null);
    const parent = parentChanged ? await checkParent(ctx, row.id, patch.parentId) : null;
    if (patch.links) patch.links = await cleanLinks(ctx, patch.links, row.links);
    const column = patch.columnId !== undefined || patch.status !== undefined ? await columnFields(ctx, patch.columnId, patch.status) : undefined;
    const was = { ...row };
    await ctx.db.patch(row._id, { ...patch, ...column, updatedAt: await changedAt(ctx) });
    if (patch.links) await dropImages(ctx, was.links, patch.links);
    // The activity: what changed, in words, by whom.
    const events: [string, string][] = [];
    if (column && column.columnId !== (was.columnId ?? was.status)) events.push([was.id, `moved to ${await columnName(ctx, column.columnId)}`]);
    if (patch.priority !== undefined && (patch.priority ?? null) !== (was.priority ?? null))
      events.push([was.id, patch.priority ? `set priority to ${PRIORITY_NAMES[patch.priority]}` : 'removed the priority']);
    if (patch.dueDate !== undefined && (patch.dueDate ?? null) !== (was.dueDate ?? null))
      events.push([was.id, patch.dueDate ? `set the due date to ${dayLabel(patch.dueDate)}` : 'removed the due date']);
    if (patch.project !== undefined && (patch.project?.runnerId ?? '') + '/' + (patch.project?.id ?? '') !== (was.project?.runnerId ?? '') + '/' + (was.project?.id ?? ''))
      events.push([was.id, patch.project ? `moved to project ${await projectTitle(ctx, patch.project)}` : 'removed the project']);
    if (parentChanged) {
      const title = patch.title ?? was.title;
      const old = was.parentId ? await find(ctx, was.parentId) : null;
      if (old) events.push([was.id, `removed this from ${old.title}`], [old.id, `removed sub-task ${title}`]);
      if (parent) events.push([was.id, `made this a sub-task of ${parent.title}`], [parent.id, `added sub-task ${title}`]);
    }
    if (events.length) {
      const author = await actor(ctx, auth, given);
      for (const [taskId, body] of events) await log(ctx, taskId, author, body);
    }
    return null;
  },
});
// Deleting a task deletes its sub-tasks, and restoring it brings back those
// that went with it. A sub-task restored without its parent stands alone.
async function deleted(ctx: MutationCtx, auth: Auth, id: string, value: boolean) {
  await authenticate(ctx, auth);
  const row = await find(ctx, id);
  if (!row || row.deleted === value) return null;
  const updatedAt = await changedAt(ctx);
  const parent = !value && row.parentId ? await find(ctx, row.parentId) : null;
  const orphan = !value && row.parentId && (!parent || parent.deleted);
  const child = Boolean(row.parentId);
  await ctx.db.patch(row._id, { deleted: value, deletedWithParent: undefined, ...(orphan ? { parentId: null } : {}), updatedAt });
  if (child) return null;
  const children = await ctx.db
    .query('tasks')
    .withIndex('by_parent', (q) => q.eq('parentId', row.id).eq('deleted', !value))
    .take(500);
  for (const child of children) {
    if (value) await ctx.db.patch(child._id, { deleted: true, deletedWithParent: true, updatedAt });
    else if (child.deletedWithParent) await ctx.db.patch(child._id, { deleted: false, deletedWithParent: undefined, updatedAt });
  }
  return null;
}
export const remove = mutation({
  args: { auth: authField, id: v.string() },
  returns: v.null(),
  handler: (ctx, a) => deleted(ctx, a.auth, a.id, true),
});
export const restore = mutation({
  args: { auth: authField, id: v.string() },
  returns: v.null(),
  handler: (ctx, a) => deleted(ctx, a.auth, a.id, false),
});
const prefRow = (ctx: Ctx) =>
  ctx.db
    .query('taskPreferences')
    .withIndex('by_name', (q) => q.eq('name', 'tasks'))
    .unique();
async function columnFields(ctx: Ctx, columnId: string | undefined, status: Infer<typeof taskStatus> | undefined) {
  if (columnId === '') bad('Choose a known column');
  if (columnId === undefined || taskStatuses.includes(columnId as Infer<typeof taskStatus>)) {
    const canonical = (columnId ?? status ?? 'todo') as Infer<typeof taskStatus>;
    if (columnId && status && status !== canonical) bad('Column and status disagree');
    return { status: canonical, columnId: canonical };
  }
  const column = (await prefRow(ctx))?.columns?.find(c => c.id === columnId);
  if (!column) bad('That column is no longer available');
  if (status && status !== column.status) bad('Column and status disagree');
  return { status: column.status, columnId: column.id };
}
export const preferences = query({
  args: { auth: authField },
  returns: v.object(taskPreferenceFields),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const row = await prefRow(ctx);
    const { _id, _creationTime, name, ...prefs } = row ?? {};
    return {
      defaultView: 'list' as const,
      grouping: 'status' as const,
      sort: 'manual' as const,
      fade: 0.85,
      photoPosition: 'center' as const,
      theme: 'system' as const,
      ...prefs,
      columnOrder: fullColumnOrder(row?.columnOrder),
      listOrder: fullColumnOrder(row?.listOrder, listStatuses),
    };
  },
});
export const setPreferences = mutation({
  args: { auth: authField, ...taskPreferenceFields },
  returns: v.null(),
  handler: async (ctx, { auth, ...patch }) => {
    await authenticate(ctx, auth);
    if (patch.columns) {
      if (patch.columns.length < 4 || patch.columns.length > 32) bad('Use between 4 and 32 columns');
      const ids = new Set<string>();
      for (const c of patch.columns) {
        if (!/^[a-zA-Z0-9_-]{1,80}$/.test(c.id) || ids.has(c.id)) bad('Use a unique column id');
        ids.add(c.id);
        c.name = c.name.trim();
        if (!c.name || c.name.length > 80) bad('Use a column name under 81 characters');
        if (taskStatuses.includes(c.id as Infer<typeof taskStatus>) && c.status !== c.id) bad('Keep built-in completion behavior');
      }
      if (taskStatuses.some(id => !ids.has(id))) bad('Keep the four built-in columns');
      const existing = await prefRow(ctx);
      if (existing?.columns?.some(c => !ids.has(c.id))) bad('Removing columns is not supported');
      if (existing?.columns?.some(c => patch.columns!.find(next => next.id === c.id)?.status !== c.status)) bad('Keep the column completion behavior');
    }
    for (const order of [patch.boardColumnOrder, patch.sectionOrder]) {
      if (!order) continue;
      const definitions = patch.columns ?? (await prefRow(ctx))?.columns;
      const ids = new Set<string>(definitions?.map(c => c.id) ?? taskStatuses);
      if (order.length > 32 || new Set(order).size !== order.length || order.some(id => !ids.has(id))) bad('Use each known column once');
    }
    if (patch.columnOrder) {
      if (new Set(patch.columnOrder).size !== patch.columnOrder.length) bad('Use each status once');
      patch.columnOrder = fullColumnOrder(patch.columnOrder);
    }
    if (patch.listOrder) {
      if (new Set(patch.listOrder).size !== patch.listOrder.length) bad('Use each status once');
      patch.listOrder = fullColumnOrder(patch.listOrder, listStatuses);
    }
    if (patch.projectOrder && (patch.projectOrder.length > 1000 || patch.projectOrder.some((k) => k.length > 300) || new Set(patch.projectOrder).size !== patch.projectOrder.length)) bad('Use each project once');
    if (patch.fade !== undefined && (!Number.isFinite(patch.fade) || patch.fade < 0 || patch.fade > 1)) bad('Fade must be between 0 and 1');
    if (patch.backgroundColor !== undefined && patch.backgroundColor !== '' && !/^#[0-9a-f]{6}$/i.test(patch.backgroundColor)) bad('Choose a hex color');
    if (patch.backgroundPhoto) {
      const file = await ctx.db.system.get(patch.backgroundPhoto);
      if (!file || file.size > 10 * 1024 * 1024 || !['image/jpeg', 'image/png', 'image/webp', 'image/gif'].includes(file.contentType ?? '')) bad('Choose a PNG, JPEG, WebP or GIF under 10 MB');
    }
    const row = await prefRow(ctx);
    if (row) await ctx.db.patch(row._id, patch);
    else await ctx.db.insert('taskPreferences', { name: 'tasks', ...patch });
    return null;
  },
});

// Upload URLs and photo links are issued only to authenticated account clients.
export const uploadPhoto = mutation({
  args: { auth: authField },
  returns: v.string(),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    return await ctx.storage.generateUploadUrl();
  },
});
export const photo = query({
  args: { auth: authField },
  returns: v.union(v.string(), v.null()),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const prefs = await prefRow(ctx);
    return prefs?.backgroundPhoto ? await ctx.storage.getUrl(prefs.backgroundPhoto) : null;
  },
});

// Small invalidation subscription; clients then refresh their loaded pages.
export const revision = query({
  args: { auth: authField },
  returns: v.union(v.number(), v.null()),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const row = await ctx.db.query('tasks').withIndex('by_updated').order('desc').first();
    return row?.updatedAt ?? null;
  },
});

export const projects = query({
  args: { auth: authField },
  returns: v.array(v.object({ runnerId: v.string(), id: v.string(), title: v.string() })),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const rows = await ctx.db.query('projects').take(1000);
    return rows.filter((p) => !p.archived).map(({ runnerId, id, title }) => ({ runnerId, id, title }));
  },
});
export const count = query({
  args: { auth: authField, project: taskProject },
  returns: v.object({ count: v.number(), more: v.boolean() }),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const rows = await ctx.db
      .query('tasks')
      .withIndex('by_project_deleted', (q) => q.eq('project.runnerId', a.project.runnerId).eq('project.id', a.project.id).eq('deleted', false))
      .take(1001);
    return { count: Math.min(rows.length, 1000), more: rows.length > 1000 };
  },
});

// Queue through the same runner command path as the website's new-thread flow.
// Keeping the command on the task makes retries (including a lost reply) harmless.
export const startThread = mutation({
  args: { auth: authField, id: v.string() },
  returns: v.id('commands'),
  handler: async (ctx, { auth, id }) => {
    await authenticate(ctx, auth);
    // Shared task access does not grant restricted runner keys remote command access.
    if ('key' in auth) await runnerAuth(ctx, auth.key, 'queueCommand');
    const row = await find(ctx, id);
    if (!row || row.deleted) bad('Task no longer exists');
    if (row.threadCommandId) return row.threadCommandId;
    const project = row.project ? await ctx.db.query('projects').withIndex('by_runner_id', q => q.eq('runnerId', row.project!.runnerId).eq('id', row.project!.id)).unique() : null;
    if (row.project && !project) bad('That project is no longer available');
    const runnerId = project?.runnerId ?? await mainRunner(ctx);
    const commandId = await queue(ctx, runnerId, 'create', undefined, {
      text: row.title + (row.notes ? '\n\n' + row.notes : ''),
      title: row.title,
      ...(project ? { projectIds: [project.id], ...(project.folders[0] ? { cwd: project.folders[0] } : {}) } : {}),
    });
    await ctx.db.patch(row._id, { threadCommandId: commandId, updatedAt: await changedAt(ctx) });
    return commandId;
  },
});

// One task by id, deleted or not, or null (API 87).
export const get = query({
  args: { auth: authField, id: v.string() },
  returns: v.union(taskOut, v.null()),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const row = await find(ctx, a.id);
    if (!row) return null;
    const { _id, _creationTime, ...task } = row;
    return expose(ctx, task);
  },
});
// A task's comments and events, oldest first (API 87).
const activityItem = v.object({ id: v.string(), taskId: v.string(), kind: taskActivityKind, body: v.string(), author: taskAuthor, createdAt: v.number() });
export const activity = query({
  args: { auth: authField, id: v.string() },
  returns: v.object({ items: v.array(activityItem) }),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const rows = await ctx.db
      .query('taskActivity')
      .withIndex('by_task', (q) => q.eq('taskId', a.id))
      .order('desc')
      .take(300);
    return { items: rows.reverse().map(({ _id, _creationTime, ...r }) => ({ id: _id as string, ...r })) };
  },
});
export const comment = mutation({
  args: { auth: authField, id: v.string(), body: v.string(), author: v.optional(taskAuthor) },
  returns: activityItem,
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const row = await find(ctx, a.id);
    if (!row || row.deleted) bad('Task no longer exists');
    const body = text(a.body.trim(), 10000, 'Comment');
    if (!body) bad('Write a comment first');
    const item = await log(ctx, row.id, await actor(ctx, a.auth, a.author), body, 'comment');
    await ctx.db.patch(row._id, { commentCount: (row.commentCount ?? 0) + 1, updatedAt: await changedAt(ctx) });
    return item;
  },
});
export const removeComment = mutation({
  args: { auth: authField, id: v.string(), commentId: v.string() },
  returns: v.null(),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const itemId = ctx.db.normalizeId('taskActivity', a.commentId);
    const item = itemId ? await ctx.db.get(itemId) : null;
    if (!item || item.taskId !== a.id || item.kind !== 'comment') return null;
    await ctx.db.delete(item._id);
    const row = await find(ctx, a.id);
    if (row) await ctx.db.patch(row._id, { commentCount: Math.max(0, (row.commentCount ?? 1) - 1), updatedAt: await changedAt(ctx) });
    return null;
  },
});
// Appends an uploaded picture (from uploadPhoto's URL) as an image link.
// Repeating it with the same file returns the same link.
export const addImage = mutation({
  args: { auth: authField, id: v.string(), storageId: v.id('_storage'), title: v.optional(v.string()) },
  returns: taskLink,
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const row = await find(ctx, a.id);
    if (!row || row.deleted) bad('Task no longer exists');
    const links = row.links ?? [];
    const had = links.find((l) => l.storageId === a.storageId);
    if (had) return had;
    if (links.length >= MAX_LINKS) bad(`Use at most ${MAX_LINKS} links`);
    await checkImage(ctx, a.storageId);
    const link: Link = { id: `image-${String(a.storageId).replace(/[^\w-]/g, '').slice(0, 90)}`, kind: 'image', title: text((a.title ?? '').trim(), 300, 'Image name') || 'Image', storageId: a.storageId };
    await ctx.db.patch(row._id, { links: [...links, link], updatedAt: await changedAt(ctx) });
    return link;
  },
});
