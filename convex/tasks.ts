// One private task collection per Threadbear deployment. Paired browsers write
// directly so changing a task never waits for a sleeping Mac.
import { ConvexError, v } from 'convex/values';
import { internalMutation, mutation, query, type QueryCtx, type MutationCtx } from './_generated/server';
import { internal } from './_generated/api';
import { checkPhoneToken, runnerAuth } from './lib';
import { mainRunner, queue } from './phone';
import { BUILDING, columnIds, defaultColumns, defaultSectionOrder, fullColumnOrder, taskStatuses, listStatuses, type TaskColumn, type TaskStatus, taskActivityKind, taskAuthor, taskFields, taskLink, taskLinkOut, taskPreferenceFields, taskPriority, taskProject, taskStatus } from './taskFields';
import type { Infer } from 'convex/values';
import type { Id } from './_generated/dataModel';

export const authField = v.union(v.object({ token: v.string() }), v.object({ key: v.string() }));
export type Auth = Infer<typeof authField>;
type Ctx = QueryCtx | MutationCtx;
export async function authenticate(ctx: Ctx, auth: Auth) {
  if ('token' in auth) await checkPhoneToken(ctx, auth.token);
  else await runnerAuth(ctx, auth.key, 'nativeTasks');
}
export function bad(message: string): never {
  throw new ConvexError({ kind: 'badArgs', message });
}
export function text(value: string, max: number, name: string) {
  if (value.length > max) bad(`${name} is too long`);
  return value;
}
export function title(value: string) {
  const clean = text(value.trim(), 500, 'Title');
  if (!clean) bad('Give the task a title');
  return clean;
}
export async function changedAt(ctx: Ctx): Promise<number> {
  const last = await ctx.db.query('tasks').withIndex('by_updated').order('desc').first();
  return Math.max(Date.now(), (last?.updatedAt ?? 0) + 1);
}
export const find = (ctx: Ctx, id: string) =>
  ctx.db
    .query('tasks')
    .withIndex('by_task_id', (q) => q.eq('id', id))
    .unique();
export async function projectExists(ctx: Ctx, project: Infer<typeof taskProject> | null | undefined) {
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
function dueTime(value: string | null | undefined) {
  if (value === undefined || value === null) return value;
  if (!/^([01]\d|2[0-3]):[0-5]\d$/.test(value)) bad('Use a due time like 09:30');
  return value;
}
const timeLabel = (value: string) => {
  const [h, m] = value.split(':').map(Number);
  return `${((h! + 11) % 12) + 1}:${String(m).padStart(2, '0')} ${h! < 12 ? 'AM' : 'PM'}`;
};
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
      if (prefs) await ctx.db.patch("taskPreferences", prefs._id, { ownerName: name });
      else await ctx.db.insert('taskPreferences', { name: 'tasks', ownerName: name });
    }
    return { kind: given.kind, name };
  }
  return { kind: 'user', name: prefs?.ownerName ?? 'You' };
}
export async function log(ctx: MutationCtx, taskId: string, author: Author, body: string, kind: 'event' | 'comment' = 'event') {
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
  const file = await ctx.db.system.get("_storage", storageId);
  if (!file || file.size > 10 * 1024 * 1024 || !IMAGE_TYPES.includes(file.contentType ?? '')) bad('Choose a PNG, JPEG, WebP or GIF under 10 MB');
}
// Images a links change dropped go from storage too.
async function dropImages(ctx: MutationCtx, before: Link[] | undefined, after: Link[]) {
  const kept = new Set(after.map((l) => l.storageId));
  for (const l of before ?? []) if (l.storageId && !kept.has(l.storageId)) await ctx.storage.delete(l.storageId);
}
async function columnName(ctx: Ctx, columnId: string) {
  return columnsOf(await prefRow(ctx)).find((c) => c.id === columnId)?.name ?? STATUS_NAMES[columnId] ?? columnId;
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
    const board = await boardOf(ctx);
    const items = await Promise.all(
      rows.slice(0, limit).map(async ({ _id, _creationTime, ...row }) => {
        place(board, row);
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
    // API 88.
    dueTime: v.optional(v.union(v.string(), v.null())),
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
      ...(a.dueDate && a.dueTime ? { dueTime: dueTime(a.dueTime) } : {}),
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
    // API 88. A time needs a date; clearing the date clears the time.
    dueTime: v.optional(v.union(v.string(), v.null())),
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
    dueTime(patch.dueTime);
    if (patch.dueDate === null) patch.dueTime = null;
    if (patch.dueTime && !(patch.dueDate ?? row.dueDate)) bad('Set a due date before a time');
    const parentChanged = patch.parentId !== undefined && (patch.parentId ?? null) !== (row.parentId ?? null);
    const parent = parentChanged ? await checkParent(ctx, row.id, patch.parentId) : null;
    if (patch.links) patch.links = await cleanLinks(ctx, patch.links, row.links);
    const column = patch.columnId !== undefined || patch.status !== undefined ? await columnFields(ctx, patch.columnId, patch.status, row) : undefined;
    const was = { ...row };
    await ctx.db.patch("tasks", row._id, { ...patch, ...column, updatedAt: await changedAt(ctx) });
    if (patch.links) await dropImages(ctx, was.links, patch.links);
    // The activity: what changed, in words, by whom.
    const events: [string, string][] = [];
    if (column && column.columnId !== (was.columnId ?? was.status)) events.push([was.id, `moved to ${await columnName(ctx, column.columnId)}`]);
    if (patch.priority !== undefined && (patch.priority ?? null) !== (was.priority ?? null))
      events.push([was.id, patch.priority ? `set priority to ${PRIORITY_NAMES[patch.priority]}` : 'removed the priority']);
    if (patch.dueDate !== undefined && (patch.dueDate ?? null) !== (was.dueDate ?? null))
      events.push([was.id, patch.dueDate ? `set the due date to ${dayLabel(patch.dueDate)}` : 'removed the due date']);
    if (patch.dueTime !== undefined && patch.dueDate !== null && (patch.dueTime ?? null) !== (was.dueTime ?? null))
      events.push([was.id, patch.dueTime ? `set the due time to ${timeLabel(patch.dueTime)}` : 'removed the due time']);
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
  await ctx.db.patch("tasks", row._id, { deleted: value, deletedWithParent: undefined, ...(orphan ? { parentId: null } : {}), updatedAt });
  if (child) return null;
  const children = await ctx.db
    .query('tasks')
    .withIndex('by_parent', (q) => q.eq('parentId', row.id).eq('deleted', !value))
    .take(500);
  for (const child of children) {
    if (value) await ctx.db.patch("tasks", child._id, { deleted: true, deletedWithParent: true, updatedAt });
    else if (child.deletedWithParent) await ctx.db.patch("tasks", child._id, { deleted: false, deletedWithParent: undefined, updatedAt });
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
export const prefRow = (ctx: Ctx) =>
  ctx.db
    .query('taskPreferences')
    .withIndex('by_name', (q) => q.eq('name', 'tasks'))
    .unique();
type Prefs = Awaited<ReturnType<typeof prefRow>>;
type Board = { columns: TaskColumn[]; order: string[] };
const columnsOf = (row: Prefs): TaskColumn[] => row?.columns ?? defaultColumns;
// Before API 89 a saved board had only the four statuses' order to go on.
function boardOrder(row: Prefs, columns = columnsOf(row)) {
  return columnIds(row?.boardColumnOrder, columns, row?.columns ? fullColumnOrder(row.columnOrder) : defaultColumns.map((c) => c.id));
}
function sectionOrder(row: Prefs, columns = columnsOf(row)) {
  return columnIds(row?.sectionOrder, columns, row?.columns ? fullColumnOrder(row.listOrder, listStatuses) : defaultSectionOrder);
}
export async function boardOf(ctx: Ctx): Promise<Board> {
  const row = await prefRow(ctx);
  const columns = columnsOf(row);
  return { columns, order: boardOrder(row, columns) };
}
const building = (columns: TaskColumn[]): TaskColumn => columns.find((c) => c.id === BUILDING) ?? defaultColumns.find((c) => c.id === BUILDING)!;
// Where a status goes when no column is named: its built-in column if it's
// still there, else the first on the board with that status, else Building.
export function columnForStatus({ columns, order }: Board, status: TaskStatus): TaskColumn {
  return columns.find((c) => c.id === status) ?? order.map((id) => columns.find((c) => c.id === id)).find((c) => c?.status === status) ?? building(columns);
}
// A task whose column was deleted shows where its tasks are being moved, so
// no client ever draws one in a column it can't find.
function place(board: Board, row: { columnId?: string; status: TaskStatus }) {
  if (board.columns.some((c) => c.id === (row.columnId ?? row.status))) return;
  const c = columnForStatus(board, row.status);
  row.columnId = c.id;
  row.status = c.status;
}
// columnId may be an id or, for agents, a column's name. With only a status,
// a task already in a column with that status stays there.
export async function columnFields(ctx: Ctx, columnId: string | undefined, status: TaskStatus | undefined, current?: { columnId?: string; status: TaskStatus }) {
  if (columnId === '') bad('Choose a known column');
  const board = await boardOf(ctx);
  const named = columnId?.trim().toLowerCase();
  const column = columnId === undefined ? undefined : (board.columns.find((c) => c.id === columnId) ?? board.columns.find((c) => c.name.toLowerCase() === named));
  if (column) {
    if (status && status !== column.status) bad('Column and status disagree');
    return { status: column.status, columnId: column.id };
  }
  if (columnId !== undefined) {
    // A deleted built-in column, from an older client: its status still says where to go.
    if (!taskStatuses.includes(columnId as TaskStatus)) bad('That column is no longer available');
    if (status && status !== columnId) bad('Column and status disagree');
    status = columnId as TaskStatus;
  }
  const want = status ?? 'todo';
  const here = current && board.columns.find((c) => c.id === (current.columnId ?? current.status));
  const next = here && here.status === want ? here : columnForStatus(board, want);
  return { status: next.status, columnId: next.id };
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
      // API 89: always the full set, each column once, so clients draw these as given.
      columns: columnsOf(row),
      boardColumnOrder: boardOrder(row),
      sectionOrder: sectionOrder(row),
    };
  },
});
export const setPreferences = mutation({
  args: { auth: authField, ...taskPreferenceFields },
  returns: v.null(),
  handler: async (ctx, { auth, ...patch }) => {
    await authenticate(ctx, auth);
    const existing = columnsOf(await prefRow(ctx));
    if (patch.columns) {
      if (patch.columns.length < 1 || patch.columns.length > 32) bad('Use between 1 and 32 columns');
      const ids = new Set<string>();
      for (const c of patch.columns) {
        if (!/^[a-zA-Z0-9_-]{1,80}$/.test(c.id) || ids.has(c.id)) bad('Use a unique column id');
        ids.add(c.id);
        c.name = c.name.trim();
        if (!c.name || c.name.length > 80) bad('Use a column name under 81 characters');
        if (taskStatuses.includes(c.id as TaskStatus) && c.status !== c.id) bad('Keep built-in completion behavior');
      }
      if (!ids.has(BUILDING)) bad(`Keep the ${building(existing).name} column: new tasks go there`);
      // Deleting goes through deleteColumn, which moves the column's tasks first.
      if (existing.some((c) => !ids.has(c.id))) bad('Delete a column with Delete column, so its tasks move');
      if (existing.some((c) => patch.columns!.find((next) => next.id === c.id)?.status !== c.status)) bad('Keep the column completion behavior');
    }
    const definitions = patch.columns ?? existing;
    for (const field of ['boardColumnOrder', 'sectionOrder'] as const) {
      const order = patch[field];
      if (!order) continue;
      if (order.length > 32 || new Set(order).size !== order.length) bad('Use each known column once');
      // An older client may still list a deleted built-in column; leave it out.
      const ids = new Set(definitions.map((c) => c.id));
      if (order.some((id) => !ids.has(id) && !taskStatuses.includes(id as TaskStatus))) bad('Use each known column once');
      patch[field] = order.filter((id) => ids.has(id));
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
    if (patch.reminders) {
      patch.reminders.to = patch.reminders.to.trim();
      if (patch.reminders.to.length > 200) bad('Use an address or channel under 201 characters');
      if (!Number.isInteger(patch.reminders.lead) || patch.reminders.lead < 0 || patch.reminders.lead > 1440) bad('Remind between 0 and 1440 minutes early');
    }
    if (patch.linear) {
      if (patch.linear.defaults.length > 200 || patch.linear.defaults.some((d) => d.projectKey.length > 300 || d.teamId.length > 100 || (d.projectId?.length ?? 0) > 100)) bad('Too many Linear defaults');
      // lastSweepAt belongs to the runner's sweep; a settings save keeps it.
      const was = (await prefRow(ctx))?.linear;
      patch.linear = { ...patch.linear, ...(was?.lastSweepAt !== undefined ? { lastSweepAt: Math.max(was.lastSweepAt, patch.linear.lastSweepAt ?? 0) } : {}) };
    }
    if (patch.fade !== undefined && (!Number.isFinite(patch.fade) || patch.fade < 0 || patch.fade > 1)) bad('Fade must be between 0 and 1');
    if (patch.backgroundColor !== undefined && patch.backgroundColor !== '' && !/^#[0-9a-f]{6}$/i.test(patch.backgroundColor)) bad('Choose a hex color');
    if (patch.backgroundPhoto) {
      const file = await ctx.db.system.get("_storage", patch.backgroundPhoto);
      if (!file || file.size > 10 * 1024 * 1024 || !['image/jpeg', 'image/png', 'image/webp', 'image/gif'].includes(file.contentType ?? '')) bad('Choose a PNG, JPEG, WebP or GIF under 10 MB');
    }
    const row = await prefRow(ctx);
    if (row) await ctx.db.patch("taskPreferences", row._id, patch);
    else await ctx.db.insert('taskPreferences', { name: 'tasks', ...patch });
    return null;
  },
});

// Upload URLs and photo links are issued only to authenticated account clients.
// API 89. Any column but Building can go. Its tasks move to the first column
// left with the same status (so done stays done), else to Building.
export const deleteColumn = mutation({
  args: { auth: authField, id: v.string() },
  // null when it was already gone.
  returns: v.union(v.object({ movedTo: v.string() }), v.null()),
  handler: async (ctx, { auth, id }) => {
    await authenticate(ctx, auth);
    const row = await prefRow(ctx);
    const columns = columnsOf(row);
    if (id === BUILDING) bad(`${building(columns).name} can be renamed but not deleted: new tasks go there`);
    const gone = columns.find((c) => c.id === id);
    const rest = columns.filter((c) => c.id !== id);
    const board = { columns: rest, order: boardOrder(row, rest) };
    if (!gone) return null;
    const target = columnForStatus(board, gone.status);
    const prefs = { columns: rest, boardColumnOrder: board.order, sectionOrder: sectionOrder(row, rest) };
    if (row) await ctx.db.patch("taskPreferences", row._id, prefs);
    else await ctx.db.insert('taskPreferences', { name: 'tasks', ...prefs });
    await moveTasks(ctx, id, target);
    return { movedTo: target.id };
  },
});
const MOVE_BATCH = 200;
// Deleted tasks move too, so restoring one never brings back a missing column.
async function moveTasks(ctx: MutationCtx, from: string, to: TaskColumn) {
  const rows = await ctx.db.query('tasks').withIndex('by_column_status', (q) => q.eq('columnId', from)).take(MOVE_BATCH);
  // Rows from before columns name only their status.
  const legacy = taskStatuses.includes(from as TaskStatus) && rows.length < MOVE_BATCH
    ? await ctx.db.query('tasks').withIndex('by_column_status', (q) => q.eq('columnId', undefined).eq('status', from as TaskStatus)).take(MOVE_BATCH - rows.length)
    : [];
  let updatedAt = await changedAt(ctx);
  for (const t of [...rows, ...legacy]) await ctx.db.patch("tasks", t._id, { columnId: to.id, status: to.status, updatedAt: updatedAt++ });
  if (rows.length + legacy.length === MOVE_BATCH) await ctx.scheduler.runAfter(0, internal.tasks.moveColumnTasks, { from, to: to.id });
}
export const moveColumnTasks = internalMutation({
  args: { from: v.string(), to: v.string() },
  returns: v.null(),
  handler: async (ctx, { from, to }) => {
    const columns = columnsOf(await prefRow(ctx));
    // Added back since, so the move is over.
    if (columns.some((c) => c.id === from)) return null;
    await moveTasks(ctx, from, columns.find((c) => c.id === to) ?? building(columns));
    return null;
  },
});
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

// At most this many of a task's pictures go to its thread, as a phone's message takes.
const TASK_THREAD_IMAGES = 6;
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
    // The task's pictures reach the agent as pictures, after the words. They
    // go as taskImages, not images: a finished command deletes its images'
    // files, and these belong to the task.
    const taskImages: { url: string }[] = [];
    for (const l of row.links ?? []) {
      if (l.kind !== 'image' || !l.storageId || taskImages.length >= TASK_THREAD_IMAGES) continue;
      const url = await ctx.storage.getUrl(l.storageId);
      if (url) taskImages.push({ url });
    }
    const pictures = taskImages.length ? '\n\nPictures attached to this task:\n' + taskImages.map((_, i) => `[Picture ${i + 1}]`).join('\n') : '';
    const commandId = await queue(ctx, runnerId, 'create', undefined, {
      text: row.title + (row.notes ? '\n\n' + row.notes : '') + pictures,
      ...(taskImages.length ? { taskImages } : {}),
      title: row.title,
      ...(project ? { projectIds: [project.id], ...(project.folders[0] ? { cwd: project.folders[0] } : {}) } : {}),
    });
    await ctx.db.patch("tasks", row._id, { threadCommandId: commandId, updatedAt: await changedAt(ctx) });
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
    place(await boardOf(ctx), task);
    return expose(ctx, task);
  },
});
// A task's comments and events, oldest first (API 87).
const activityItem = v.object({ id: v.string(), taskId: v.string(), kind: taskActivityKind, body: v.string(), author: taskAuthor, createdAt: v.number(), source: v.optional(v.literal('linear')), externalId: v.optional(v.string()) });
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
    await ctx.db.patch("tasks", row._id, { commentCount: (row.commentCount ?? 0) + 1, updatedAt: await changedAt(ctx) });
    return item;
  },
});
export const removeComment = mutation({
  args: { auth: authField, id: v.string(), commentId: v.string() },
  returns: v.null(),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const itemId = ctx.db.normalizeId('taskActivity', a.commentId);
    const item = itemId ? await ctx.db.get("taskActivity", itemId) : null;
    if (!item || item.taskId !== a.id || item.kind !== 'comment') return null;
    // Comments copied from Linear stay as long as the issue has them.
    if (item.source === 'linear') bad('That comment is from Linear: remove it there');
    await ctx.db.delete("taskActivity", item._id);
    const row = await find(ctx, a.id);
    if (row) await ctx.db.patch("tasks", row._id, { commentCount: Math.max(0, (row.commentCount ?? 1) - 1), updatedAt: await changedAt(ctx) });
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
    await ctx.db.patch("tasks", row._id, { links: [...links, link], updatedAt: await changedAt(ctx) });
    return link;
  },
});

// API 88: the one Mac that gets here first for a due date and time sends its
// reminder; every other one (and the same Mac after a restart) is told no.
export const claimReminder = mutation({
  args: { auth: authField, id: v.string(), key: v.string() },
  returns: v.boolean(),
  handler: async (ctx, a) => {
    await authenticate(ctx, a.auth);
    const row = await find(ctx, a.id);
    if (!row || row.deleted || row.status === 'done' || !row.dueDate || !row.dueTime) return false;
    if (a.key !== `${row.dueDate}T${row.dueTime}` || row.remindedFor === a.key) return false;
    await ctx.db.patch("tasks", row._id, { remindedFor: a.key });
    return true;
  },
});
