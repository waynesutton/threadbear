import { test } from 'node:test';
import assert from 'node:assert/strict';
import { LIST_STATUSES, fullOrder, moveTo, groupOrder, positionAt, splitDraft } from './tasks-order.ts';

test('a column order saved before Doing gets Doing back after Todo', () => {
  assert.deepEqual(fullOrder(['done', 'todo', 'backlog']), ['done', 'todo', 'doing', 'backlog']);
  assert.deepEqual(fullOrder(undefined), ['backlog', 'todo', 'doing', 'done']);
  assert.deepEqual(fullOrder(['doing', 'done', 'todo', 'backlog']), ['doing', 'done', 'todo', 'backlog']);
});
test('project groups keep the saved order, add new ones after, and No project goes last', () => {
  assert.deepEqual(groupOrder(['b', 'a'], ['a', 'none', 'c', 'b']), ['b', 'a', 'c', 'none']);
  assert.deepEqual(groupOrder(['gone', 'a'], ['a']), ['a']);
  assert.deepEqual(groupOrder(['b', 'a'], ['a', 'b'], 'a'), ['a']);
});
test('a dropped task lands between its neighbours', () => {
  const peers = [{ id: 'a', position: 100 }, { id: 'b', position: 200 }];
  assert.equal(positionAt(peers, 'b'), 150);
  assert.equal(positionAt(peers, 'a'), -900);
  assert.equal(positionAt(peers), 1200);
});
test('the first typed line is the title and the rest is the note', () => {
  assert.deepEqual(splitDraft('Call Sam\nabout the beta\n'), { title: 'Call Sam', notes: 'about the beta' });
  assert.deepEqual(splitDraft('\n  Ship it  '), { title: 'Ship it', notes: '' });
  assert.deepEqual(splitDraft('   \n '), { title: '', notes: '' });
});
test('the list runs most active first unless it has its own order', () => {
  assert.deepEqual(fullOrder(undefined, LIST_STATUSES), ['doing', 'todo', 'backlog', 'done']);
});
test('a dragged column lands where it was dropped, whichever way it went', () => {
  const cols = ['backlog', 'todo', 'doing', 'done'];
  assert.deepEqual(moveTo(cols, 'backlog', 'doing'), ['todo', 'doing', 'backlog', 'done']);
  assert.deepEqual(moveTo(cols, 'done', 'todo'), ['backlog', 'done', 'todo', 'doing']);
  assert.deepEqual(moveTo(cols, 'todo', 'todo'), cols);
});
