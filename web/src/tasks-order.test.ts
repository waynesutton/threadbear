import { test } from 'node:test';
import assert from 'node:assert/strict';
import { LIST_STATUSES, fullOrder, moveTo, groupOrder, positionAt, splitDraft, splitList } from './tasks-order.ts';

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
test('a pasted list is one task per item, indented ones as sub-tasks', () => {
  const items = splitList('## Launch\n- [ ] Write the post\n  - [x] Pick a title\n    - Draft the hook\n  with a picture\n* Ship it\n\n1. Tell Sam');
  assert.deepEqual(items, [
    { title: 'Write the post', notes: '', done: false, parent: null },
    { title: 'Pick a title', notes: '', done: true, parent: 0 },
    { title: 'Draft the hook', notes: 'with a picture', done: false, parent: 0 },
    { title: 'Ship it', notes: '', done: false, parent: null },
    { title: 'Tell Sam', notes: '', done: false, parent: null },
  ]);
});
test('text that is not a list stays one task', () => {
  assert.equal(splitList('Plan the launch\n- post\n- tweet'), null);
  assert.equal(splitList('- just one'), null);
  assert.equal(splitList('**bold**\n---'), null);
  assert.equal(splitList('Call Sam'), null);
});
