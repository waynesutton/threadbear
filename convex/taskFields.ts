// Native Tasks share their validators across storage and public interfaces.
import { v } from 'convex/values';
export const taskStatus = v.union(v.literal('backlog'), v.literal('todo'), v.literal('doing'), v.literal('done'));
// Board order. Rows saved before Doing existed hold three; preferences adds it back.
export const taskStatuses = ['backlog', 'todo', 'doing', 'done'] as const;
export type TaskStatus = (typeof taskStatuses)[number];
export const listStatuses: TaskStatus[] = ['doing', 'todo', 'backlog', 'done'];
export function fullColumnOrder(order: readonly TaskStatus[] | undefined, fallback: readonly TaskStatus[] = taskStatuses): TaskStatus[] {
  const out = (order ?? fallback).filter((s, i, all) => taskStatuses.includes(s) && all.indexOf(s) === i);
  for (const s of taskStatuses) if (!out.includes(s)) out.splice(s === 'doing' ? Math.max(0, out.indexOf('todo') + 1) : out.length, 0, s);
  return out;
}
export const taskView = v.union(v.literal('board'), v.literal('list'));
export const taskProject = v.object({ runnerId: v.string(), id: v.string() });
// API 87: Linear-style issue fields. All optional, so rows and clients from before keep working.
export const taskPriority = v.union(v.literal('urgent'), v.literal('high'), v.literal('normal'), v.literal('low'));
export const taskAuthor = v.object({ kind: v.union(v.literal('user'), v.literal('agent')), name: v.string() });
const linkFields = {
  id: v.string(),
  kind: v.union(v.literal('link'), v.literal('print'), v.literal('image'), v.literal('thread')),
  title: v.string(),
  url: v.optional(v.string()),
  printId: v.optional(v.string()),
  thread: v.optional(v.object({ runnerId: v.string(), id: v.string() })),
  storageId: v.optional(v.id('_storage')),
};
export const taskLink = v.object(linkFields);
// What list returns: an image link also carries its storage URL.
export const taskLinkOut = v.object({ ...linkFields, imageUrl: v.optional(v.string()) });
export const taskActivityKind = v.union(v.literal('comment'), v.literal('event'));
export const taskActivityFields = {
  taskId: v.string(),
  kind: taskActivityKind,
  body: v.string(),
  author: taskAuthor,
  createdAt: v.number(),
};
export const taskFields = {
  id: v.string(),
  title: v.string(),
  notes: v.string(),
  status: taskStatus,
  columnId: v.optional(v.string()),
  threadCommandId: v.optional(v.id('commands')),
  thread: v.optional(v.object({ runnerId: v.string(), id: v.string() })),
  threadError: v.optional(v.string()),
  project: v.union(taskProject, v.null()),
  position: v.number(),
  createdAt: v.number(),
  updatedAt: v.number(),
  deleted: v.boolean(),
  priority: v.optional(v.union(taskPriority, v.null())),
  // A calendar date, YYYY-MM-DD, with no time zone.
  dueDate: v.optional(v.union(v.string(), v.null())),
  // One level deep: a sub-task of that task's id.
  parentId: v.optional(v.union(v.string(), v.null())),
  links: v.optional(v.array(taskLink)),
  commentCount: v.optional(v.number()),
  createdBy: v.optional(taskAuthor),
  // Deleted along with its parent, so restoring the parent brings it back. Never listed.
  deletedWithParent: v.optional(v.boolean()),
};
export const taskPreferenceFields = {
  defaultView: v.optional(taskView),
  boardColumnOrder: v.optional(v.array(v.string())),
  sectionOrder: v.optional(v.array(v.string())),
  columns: v.optional(v.array(v.object({ id: v.string(), name: v.string(), status: taskStatus }))),
  columnOrder: v.optional(v.array(taskStatus)),
  // The list's own section order: most active first, unlike the board.
  listOrder: v.optional(v.array(taskStatus)),
  // Project groups' order, as runnerId/id keys, with 'none' for tasks without one.
  projectOrder: v.optional(v.array(v.string())),
  grouping: v.optional(v.union(v.literal('status'), v.literal('project'))),
  sort: v.optional(v.union(v.literal('manual'), v.literal('newest'), v.literal('oldest'), v.literal('title'))),
  backgroundColor: v.optional(v.string()),
  backgroundPhoto: v.optional(v.union(v.id('_storage'), v.null())),
  photoPosition: v.optional(v.union(v.literal('top'), v.literal('center'), v.literal('bottom'))),
  fade: v.optional(v.number()),
  theme: v.optional(v.union(v.literal('system'), v.literal('light'), v.literal('dark'))),
  // API 87: the owner's first name, as the runner last gave it, for activity a browser writes.
  ownerName: v.optional(v.string()),
};
