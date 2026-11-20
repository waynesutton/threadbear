import { test } from 'node:test';
import assert from 'node:assert/strict';
import { columnDefinitions, deleteTarget, orderedColumns, taskColumn } from './tasks-columns.ts';
test('renamed columns keep membership and older status writes remain visible', () => {
  const columns = columnDefinitions([...columnDefinitions(), { id: 'review', name: 'Review', status: 'todo' }]);
  assert.equal(taskColumn({ status: 'todo', columnId: 'review' }, columns), 'review');
  assert.equal(taskColumn({ status: 'done', columnId: 'review' }, columns), 'done');
  assert.equal(taskColumn({ status: 'todo' }, columns), 'todo');
  assert.deepEqual(orderedColumns(columns, ['missing', 'done', 'done']), ['done', 'backlog', 'todo', 'doing', 'review']);
});

test('a deleted column stays gone and its tasks go where the backend sends them', () => {
  const columns = [
    { id: 'backlog', name: 'Backlog', status: 'backlog' },
    { id: 'todo', name: 'Building', status: 'todo' },
    { id: 'observing', name: 'Observing', status: 'doing' },
    { id: 'done', name: 'Done', status: 'done' },
    { id: 'canceled', name: 'Canceled', status: 'done' },
  ] as const;
  const defs = columnDefinitions([...columns]);
  assert.ok(!defs.some(c => c.id === 'doing'));
  const order = ['backlog', 'todo', 'observing', 'done', 'canceled'];
  assert.equal(deleteTarget('canceled', defs, order), 'done');
  assert.equal(deleteTarget('done', defs, order), 'canceled');
  assert.equal(deleteTarget('observing', defs, order), 'todo');
  // An older row in a deleted Doing column shows in the first doing column.
  assert.equal(taskColumn({ status: 'doing' }, defs), 'observing');
});
