import { test } from 'node:test';
import assert from 'node:assert/strict';
import { columnDefinitions, orderedColumns, taskColumn } from './tasks-columns.ts';
test('renamed columns keep membership and older status writes remain visible', () => {
  const columns = columnDefinitions([{ id: 'review', name: 'Review', status: 'todo' }]);
  assert.equal(taskColumn({ status: 'todo', columnId: 'review' }, columns), 'review');
  assert.equal(taskColumn({ status: 'done', columnId: 'review' }, columns), 'done');
  assert.equal(taskColumn({ status: 'todo' }, columns), 'todo');
  assert.deepEqual(orderedColumns(columns, ['missing', 'done', 'done']), ['done', 'review', 'backlog', 'todo', 'doing']);
});
