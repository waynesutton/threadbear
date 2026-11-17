import assert from 'node:assert/strict';
import test from 'node:test';
import { completionHeading } from './completion.ts';

test('the task count includes pending and blocked work across the request', () => {
  assert.equal(completionHeading({ tasks: [
    { id: 'build', title: 'Build', status: 'completed' },
    { id: 'test', title: 'Test', status: 'blocked' },
    { id: 'land', title: 'Land', status: 'pending' },
  ] }), '1 of 3 tasks completed');
});

test('all completed is explicit and an empty or missing list makes no completion claim', () => {
  assert.equal(completionHeading({ tasks: [{ id: 'build', title: 'Build', status: 'completed' }] }), '1 task completed');
  assert.equal(completionHeading({ tasks: [] }), undefined);
  assert.equal(completionHeading(undefined), undefined);
});
