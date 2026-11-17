// Send only edits made in this form, so another device's unrelated changes survive.
export type TaskDraft = {
  title: string;
  notes: string;
  columnId?: string;
  status: 'backlog' | 'todo' | 'doing' | 'done';
  project: { id: string; runnerId: string } | null;
};
export function taskDraftChanges(base: TaskDraft, next: TaskDraft): Partial<TaskDraft> {
  const patch: Partial<TaskDraft> = {};
  if (base.title !== next.title) patch.title = next.title;
  if (base.notes !== next.notes) patch.notes = next.notes;
  if (base.columnId !== next.columnId) patch.columnId = next.columnId;
  if (base.status !== next.status) patch.status = next.status;
  if (base.project?.id !== next.project?.id || base.project?.runnerId !== next.project?.runnerId) patch.project = next.project;
  return patch;
}
