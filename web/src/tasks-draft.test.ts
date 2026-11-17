import { test } from 'node:test';
import assert from 'node:assert/strict';
import { taskDraftChanges, type TaskDraft } from './tasks-draft.ts';
test('saving a title edit preserves a concurrent status, note and project edit', () => {
  const original: TaskDraft = { title: 'Draft', notes: 'Notes', status: 'todo', project: { runnerId: 'mac', id: 'launch' } };
  const edited = { ...original, title: 'Final', project: { id: 'launch', runnerId: 'mac' } };
  const changedElsewhere = { ...original, status: 'done', notes: 'New notes', project: null };
  const patch = taskDraftChanges(original, edited);
  assert.deepEqual(patch, { title: 'Final' });
  assert.deepEqual({ ...changedElsewhere, ...patch }, { ...changedElsewhere, title: 'Final' });
});
test('unassigning a project and clearing notes are explicit edits', () => {
  const original: TaskDraft = { title: 'Draft', notes: 'Notes', status: 'todo', project: { runnerId: 'mac', id: 'launch' } };
  assert.deepEqual(taskDraftChanges(original, { ...original, notes: '', project: null }), { notes: '', project: null });
  assert.deepEqual(taskDraftChanges(original, original), {});
});
