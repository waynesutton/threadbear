// Native Tasks share their validators across storage and public interfaces.
import { v } from 'convex/values';
export const taskStatus = v.union(v.literal('backlog'), v.literal('todo'), v.literal('done'));
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
  grouping: v.optional(v.union(v.literal('status'), v.literal('project'))),
  sort: v.optional(v.union(v.literal('manual'), v.literal('newest'), v.literal('oldest'), v.literal('title'))),
  backgroundColor: v.optional(v.string()),
  backgroundPhoto: v.optional(v.union(v.id('_storage'), v.null())),
  photoPosition: v.optional(v.union(v.literal('top'), v.literal('center'), v.literal('bottom'))),
  fade: v.optional(v.number()),
  theme: v.optional(v.union(v.literal('system'), v.literal('light'), v.literal('dark'))),
};
