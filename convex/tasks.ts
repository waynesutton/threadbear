// One private task collection per Threadbear deployment. Paired browsers write
// directly so changing a task never waits for a sleeping Mac.
import { ConvexError, v } from 'convex/values';
import { mutation, query, type QueryCtx, type MutationCtx } from './_generated/server';
import { checkPhoneToken, runnerAuth } from './lib';
import { mainRunner, queue } from './phone';
import { fullColumnOrder, taskStatuses, listStatuses, taskFields, taskPreferenceFields, taskProject, taskStatus } from './taskFields';
import type { Infer } from 'convex/values';

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
function expose(row: Infer<typeof taskFieldsValidator>) {
  return row;
}
const taskFieldsValidator = v.object(taskFields);

export const list = query({
  args: {
    auth: authField,
    after: v.optional(v.string()),
    limit: v.optional(v.number()),
  },
  returns: v.object({
    items: v.array(taskFieldsValidator),
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
        return expose(row);
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
  },
  returns: v.string(),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    if (!/^[\w-]{1,100}$/.test(a.id)) bad('Invalid task id');
    if (await find(ctx, a.id)) return a.id;
    await projectExists(ctx, a.project);
    const now = await changedAt(ctx);
    await ctx.db.insert('tasks', {
      id: a.id,
      title: title(a.title),
      notes: text(a.notes ?? '', 20000, 'Notes'),
      ...(await columnFields(ctx, a.columnId, a.status)),
      project: a.project ?? null,
      position: a.position !== undefined && Number.isFinite(a.position) ? a.position : now,
      createdAt: now,
      updatedAt: now,
      deleted: false,
    });
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
  },
  returns: v.null(),
  handler: async (ctx, { auth, id, ...patch }) => {
    await authenticate(ctx, auth);
    const row = await find(ctx, id);
    if (!row || row.deleted) bad('Task no longer exists');
    await projectExists(ctx, patch.project);
    if (patch.title !== undefined) patch.title = title(patch.title);
    if (patch.notes !== undefined) text(patch.notes, 20000, 'Notes');
    if (patch.position !== undefined && !Number.isFinite(patch.position)) bad('Invalid position');
    const column = patch.columnId !== undefined || patch.status !== undefined ? await columnFields(ctx, patch.columnId, patch.status) : {};
    await ctx.db.patch(row._id, { ...patch, ...column, updatedAt: await changedAt(ctx) });
    return null;
  },
});
async function deleted(ctx: MutationCtx, auth: Auth, id: string, value: boolean) {
  await authenticate(ctx, auth);
  const row = await find(ctx, id);
  if (row && row.deleted !== value)
    await ctx.db.patch(row._id, {
      deleted: value,
      updatedAt: await changedAt(ctx),
    });
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
