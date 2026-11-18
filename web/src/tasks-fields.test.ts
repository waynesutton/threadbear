import { test } from 'node:test';
import assert from 'node:assert/strict';
import { cleanUrl, dueLabel, monthGrid, overdue, priorityName, quickDues, shiftMonth, subTaskProgress } from './tasks-fields.ts';

test('due shortcuts: end of this week is the coming Sunday, or today on a Sunday', () => {
  const wednesday = new Date(2026, 10, 18);
  assert.deepEqual(quickDues(wednesday).map((q) => q.day), ['2026-11-18', '2026-11-19', '2026-11-22', '2026-11-25']);
  const sunday = new Date(2026, 10, 22);
  assert.equal(quickDues(sunday)[2]!.day, '2026-11-22');
  // Month and year ends roll over.
  assert.deepEqual(quickDues(new Date(2026, 11, 31)).map((q) => q.day), ['2026-12-31', '2027-01-01', '2027-01-03', '2027-01-07']);
});

test('due labels read as words near today and as dates otherwise', () => {
  const now = new Date(2026, 10, 18);
  assert.equal(dueLabel('2026-11-18', now), 'Today');
  assert.equal(dueLabel('2026-11-19', now), 'Tomorrow');
  assert.equal(dueLabel('2026-11-17', now), 'Yesterday');
  assert.equal(dueLabel('2026-12-02', now), 'Dec 2');
  assert.equal(dueLabel('2027-01-02', now), 'Jan 2, 2027');
  assert.equal(overdue('2026-11-17', now), true);
  assert.equal(overdue('2026-11-18', now), false);
});

test('the month grid starts on a Monday and holds the whole month', () => {
  const grid = monthGrid('2026-11');
  assert.equal(grid.length, 42);
  assert.equal(grid[0]!.day, '2026-10-26');
  assert.equal(grid.filter((d) => d.inMonth).length, 30);
  assert.equal(shiftMonth('2026-12', 1), '2027-01');
  assert.equal(shiftMonth('2026-01', -1), '2025-12');
});

test('priority names, sub-task progress and links', () => {
  assert.equal(priorityName(null), 'No priority');
  assert.equal(priorityName('urgent'), 'Urgent');
  assert.equal(subTaskProgress([]), null);
  assert.equal(subTaskProgress([{ status: 'done' }, { status: 'todo' }]), '1/2');
  assert.equal(cleanUrl('example.com/a'), 'https://example.com/a');
  assert.equal(cleanUrl('http://x.test'), 'http://x.test');
  assert.equal(cleanUrl('javascript:alert(1)'), null);
  assert.equal(cleanUrl('not a link'), null);
});
