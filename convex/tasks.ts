// One private task collection per Threadbear deployment. Paired browsers write
// directly so changing a task never waits for a sleeping Mac.
import { ConvexError, v } from 'convex/values';
import { mutation, query, type QueryCtx, type MutationCtx } from './_generated/server';
import { checkPhoneToken, runnerAuth } from './lib';
import { taskFields, taskPreferenceFields, taskProject, taskStatus } from './taskFields';
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
    project: v.optional(v.union(taskProject, v.null())),
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
      status: a.status ?? 'todo',
      project: a.project ?? null,
      position: now,
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
    await ctx.db.patch(row._id, { ...patch, updatedAt: await changedAt(ctx) });
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
export const preferences = query({
  args: { auth: authField },
  returns: v.object(taskPreferenceFields),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const row = await prefRow(ctx);
    const { _id, _creationTime, name, ...prefs } = row ?? {};
    return {
      defaultView: 'board' as const,
      columnOrder: ['backlog', 'todo', 'done'] as ('backlog' | 'todo' | 'done')[],
      grouping: 'status' as const,
      sort: 'manual' as const,
      fade: 0.85,
      photoPosition: 'center' as const,
      theme: 'system' as const,
      ...prefs,
    };
  },
});
export const setPreferences = mutation({
  args: { auth: authField, ...taskPreferenceFields },
  returns: v.null(),
  handler: async (ctx, { auth, ...patch }) => {
    await authenticate(ctx, auth);
    if (patch.columnOrder && (patch.columnOrder.length !== 3 || new Set(patch.columnOrder).size !== 3)) bad('Use each status once');
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
