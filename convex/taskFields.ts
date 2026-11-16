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
export const taskFields = {
  id: v.string(),
  title: v.string(),
  notes: v.string(),
  status: taskStatus,
  project: v.union(taskProject, v.null()),
  position: v.number(),
  createdAt: v.number(),
  updatedAt: v.number(),
  deleted: v.boolean(),
};
export const taskPreferenceFields = {
  defaultView: v.optional(taskView),
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
};
