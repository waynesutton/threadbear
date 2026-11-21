import { test } from 'node:test';
import assert from 'node:assert/strict';
import { LinearTasks, LinearError, describe as describeIssue, issueKey, issueRow, linearCaller, linearHelper, marker, picturesOf, priorityNumber, waitingFor, webLinks, type CallResult } from '../../src/linear-tasks.ts';
import type { NativeTask } from '../../src/tasks.ts';

// docs/protocol.md, "Tasks and Linear". Shapes are Linear's MCP answers
// through the Connectors gateway (20 Nov 2026), with made-up data.

const TEAM = { id: 'team-1', key: 'ENG', name: 'Engineering', displayName: 'Engineering' };
const issue = (over: Record<string, unknown> = {}) => ({
  id: 'ENG-7',
  uuid: '11111111-2222-3333-4444-555555555555',
  title: 'Fix the thing',
  url: 'https://linear.app/acme/issue/ENG-7/fix-the-thing',
  priority: { value: 2, name: 'High' },
  dueDate: '2026-12-01',
  status: 'Todo',
  statusType: 'unstarted',
  updatedAt: '2026-11-20T10:00:00.000Z',
  team: 'Engineering',
  teamId: 'team-1',
  projectId: 'proj-uuid',
  project: 'Launch',
  ...over,
});
const task = (over: Partial<NativeTask> = {}): NativeTask => ({
  id: 'task-1',
  title: 'Fix the thing',
  notes: 'It breaks on Tuesdays.',
  status: 'todo',
  project: null,
  position: 1,
  createdAt: 1,
  updatedAt: 1,
  deleted: false,
  priority: 'high',
  dueDate: '2026-12-01',
  links: [
    { id: 'l1', kind: 'link', title: 'Spec', url: 'https://example.com/spec' },
    { id: 'p1', kind: 'print', title: 'A print', printId: 'pg_1' },
    { id: 'image-a', kind: 'image', title: 'shot.png', storageId: 'st_a', imageUrl: 'https://convex.example/img/a' },
    { id: 'image-b', kind: 'image', title: 'two.png', storageId: 'st_b', imageUrl: 'https://convex.example/img/b' },
  ],
  ...over,
});

type Handler = (args: Record<string, unknown>) => unknown;
function harness(o: { tools?: Record<string, Handler>; tasks?: NativeTask[]; connected?: boolean; api?: number } = {}) {
  const calls: { name: string; args: Record<string, unknown> }[] = [];
  const backend: { op: string; args: Record<string, unknown> }[] = [];
  const rows = new Map((o.tasks ?? [task(), task({ id: 'sub-1', title: 'First step', status: 'done', parentId: 'task-1', position: 1 }), task({ id: 'sub-2', title: 'Second step', parentId: 'task-1', position: 2 })]).map((t) => [t.id, t]));
  let sweepClaim: { since: number | null } | null = { since: null };
  const tools: Record<string, Handler> = {
    linear__get_workspace: () => ({ id: 'ws', name: 'Acme', url: 'https://linear.app/acme' }),
    linear__list_teams: () => ({ teams: [TEAM], hasNextPage: false }),
    linear__list_projects: () => ({ projects: [{ id: 'P-ENG-1', uuid: 'proj-uuid', name: 'Launch', status: { name: 'Building', type: 'started' } }, { id: 'P-ENG-2', uuid: 'old-uuid', name: 'Old', status: { name: 'Done', type: 'completed' } }], hasNextPage: false }),
    linear__list_issues: () => ({ issues: [], hasNextPage: false }),
    linear__get_issue: () => issue(),
    linear__list_comments: () => ({ comments: [{ id: 'c1', body: 'Looks good', author: { id: 'u', name: 'Sam' }, createdAt: '2026-11-20T11:00:00Z' }], hasNextPage: false }),
    linear__prepare_file_upload: (a) => ({ assetUrl: `https://uploads.linear.app/ws/${String(a.filename)}`, uploadRequest: { method: 'PUT', url: 'https://storage.googleapis.com/signed', headers: { 'content-type': String(a.contentType), 'x-goog-content-length-range': `${a.size},${a.size}` } } }),
    linear__save_issue: () => ({ id: 'ENG-7', url: 'https://linear.app/acme/issue/ENG-7/fix-the-thing' }),
    ...o.tools,
  };
  const call = async (name: string, args: Record<string, unknown>): Promise<CallResult> => {
    calls.push({ name, args });
    const h = tools[name];
    if (!h) throw new Error(`no tool ${name}`);
    const r = h(args);
    if (r && typeof r === 'object' && 'waiting' in (r as object)) return r as CallResult;
    if (r instanceof Error) throw r;
    return { text: JSON.stringify(r) };
  };
  const puts: { url: string; headers: Record<string, string>; size: number }[] = [];
  const fakeFetch = (async (url: string | URL, init?: RequestInit) => {
    const u = String(url);
    if (init?.method === 'PUT') {
      puts.push({ url: u, headers: init.headers as Record<string, string>, size: (init.body as Uint8Array).byteLength });
      return new Response(null, { status: 200 });
    }
    if (u.includes('/img/b') && o.tools?.failImage) return new Response('nope', { status: 500 });
    return new Response(new Uint8Array([1, 2, 3, 4]), { status: 200, headers: { 'content-type': 'image/png' } });
  }) as typeof fetch;
  const l = new LinearTasks({
    call: () => call,
    connected: () => o.connected ?? true,
    apiVersion: () => o.api ?? 105,
    tasksOn: () => true,
    tasks: async (op, args) => {
      if (op === 'get') return rows.get(String(args?.id)) ?? null;
      if (op === 'list') return { items: [...rows.values()], next: null };
      throw new Error(op);
    },
    linear: async (op, args = {}) => {
      backend.push({ op, args });
      if (op === 'claim') return { uploads: [] };
      if (op === 'linkedIssues') return [];
      if (op === 'synced') return [{ id: 'task-1', issueId: issue().uuid, identifier: 'ENG-7', teamId: 'team-1' }];
      if (op === 'claimSweep') return sweepClaim;
      if (op === 'apply') return { changed: ['changed status to Done'] };
      return null;
    },
    fetch: fakeFetch,
  });
  return { l, calls, backend, puts, rows, setClaim: (c: typeof sweepClaim) => (sweepClaim = c) };
}
const settled = async (h: ReturnType<typeof harness>) => {
  for (let i = 0; i < 50 && !h.backend.some((b) => b.op === 'settle'); i++) await new Promise((r) => setTimeout(r, 5));
  return h.backend.find((b) => b.op === 'settle')!;
};

test('the description carries the notes, sub-tasks as a checklist, pictures and the marker', () => {
  const d = describeIssue(task(), [{ title: 'First step', status: 'done' }, { title: 'Second step', status: 'todo' }], [{ title: 'shot [1].png', url: 'https://uploads.linear.app/x' }]);
  assert.equal(
    d,
    'It breaks on Tuesdays.\n\n**Sub-tasks**\n- [x] First step\n- [ ] Second step\n\n![shot 1.png](https://uploads.linear.app/x)\n\n---\nSent from Threadbear · Threadbear task task-1',
  );
  assert.equal(describeIssue(task({ notes: '  ' }), []), '---\nSent from Threadbear · Threadbear task task-1');
});

test('only web links go to Linear, and pictures are their own list', () => {
  assert.deepEqual(webLinks(task().links), [{ url: 'https://example.com/spec', title: 'Spec' }]);
  assert.deepEqual(picturesOf(task().links).map((p) => p.storageId), ['st_a', 'st_b']);
  assert.deepEqual(webLinks([{ id: 'linear-issue', kind: 'link', title: 'ENG-7 in Linear', url: 'https://linear.app/x' }]), []);
});

test('priorities, issue keys, rows and held calls are read the way Linear writes them', () => {
  assert.deepEqual(['urgent', 'high', 'normal', 'low', null].map((p) => priorityNumber(p as never)), [1, 2, 3, 4, 0]);
  assert.equal(issueKey('eng-12'), 'ENG-12');
  assert.equal(issueKey('https://linear.app/acme/issue/MKT-825/guide-for-hosting'), 'MKT-825');
  assert.equal(issueKey('fix the thing'), undefined);
  const row = issueRow(issue())!;
  assert.equal(row.issueId, issue().uuid);
  assert.equal(row.identifier, 'ENG-7');
  assert.equal(row.priority, 2);
  assert.equal(row.projectId, 'proj-uuid');
  assert.equal(issueRow({ id: 'ENG-7' }), undefined);
  assert.deepEqual(waitingFor('Waiting for Wayne to approve this at http://127.0.0.1:8795/approvals/ap_abc123 (it expires in 24 hours).'), { id: 'ap_abc123', url: 'http://127.0.0.1:8795/approvals/ap_abc123' });
  assert.equal(waitingFor('{"id":"ENG-7"}'), undefined);
  assert.ok(new LinearError('{"error":"invalid_request","message":"Could not find referenced Issue."}').notFound);
});

test('the Threadbear login is the main helper asked for the threadbear agent', () => {
  assert.equal(linearHelper('/Users/x/.config/connectors/headers.sh claude'), '/Users/x/.config/connectors/headers.sh threadbear');
  assert.equal(linearHelper('/Users/x/.config/connectors/headers.sh claude', '/opt/mine.sh'), '/opt/mine.sh');
  assert.equal(linearHelper('/opt/other-helper'), undefined);
});

test('Linear is offered only when it is connected and the backend is new enough', async () => {
  assert.equal(harness({ connected: false }).l.status().ready, false);
  assert.match(harness({ api: 104 }).l.status().why!, /Update the Threadbear backend/);
  assert.deepEqual(harness().l.status(), { ready: true });
  await assert.rejects(harness({ connected: false }).l.send({ taskId: 'task-1', teamId: 'team-1' }), /Linear isn’t connected/);
});

test('a send claims the task, uploads the ticked pictures, then makes one issue', async () => {
  const h = harness();
  const r = await h.l.send({ taskId: 'task-1', teamId: 'team-1', projectId: 'proj-uuid', imageIds: ['st_a'], sync: true });
  assert.equal(r.started, true);
  const s = await settled(h);
  assert.equal(h.backend[0]!.op, 'claim');
  assert.equal(h.backend[0]!.args.projectName, 'Launch');
  // One picture, sent with Linear's signed headers as given.
  assert.equal(h.puts.length, 1);
  assert.equal(h.puts[0]!.headers['x-goog-content-length-range'], '4,4');
  assert.ok(h.backend.some((b) => b.op === 'uploaded' && b.args.storageId === 'st_a'));
  const save = h.calls.find((c) => c.name === 'linear__save_issue')!;
  assert.equal(save.args.id, undefined);
  assert.equal(save.args.team, 'team-1');
  assert.equal(save.args.project, 'proj-uuid');
  assert.equal(save.args.priority, 2);
  assert.equal(save.args.dueDate, '2026-12-01');
  assert.deepEqual(save.args.links, [{ url: 'https://example.com/spec', title: 'Spec' }]);
  assert.match(String(save.args.description), /!\[shot\.png\]\(https:\/\/uploads\.linear\.app\/ws\/shot\.png\)/);
  assert.match(String(save.args.description), /- \[x\] First step\n- \[ \] Second step/);
  assert.doesNotMatch(String(save.args.description), /convex\.example/);
  assert.equal((s.args.issue as { identifier: string }).identifier, 'ENG-7');
  assert.equal(s.args.found, undefined);
});

test('a project that left the team is refused before anything is claimed', async () => {
  const h = harness();
  await assert.rejects(h.l.send({ taskId: 'task-1', teamId: 'team-1', projectId: 'old-uuid' }), /isn’t open in this team/);
  await assert.rejects(h.l.send({ taskId: 'task-1', teamId: 'nope' }), /team isn’t in your Linear workspace/);
  assert.equal(h.backend.length, 0);
});

test('done, deleted and already linked tasks are not sent', async () => {
  await assert.rejects(harness({ tasks: [task({ status: 'done' })] }).l.send({ taskId: 'task-1', teamId: 'team-1' }), /done/);
  await assert.rejects(harness({ tasks: [task({ deleted: true })] }).l.send({ taskId: 'task-1', teamId: 'team-1' }), /no longer exists/);
  await assert.rejects(harness({ tasks: [task({ linear: { state: 'linked', identifier: 'ENG-7', requestId: 'r', claimedAt: 1, teamId: 't', teamName: 'T', sync: true } })] }).l.send({ taskId: 'task-1', teamId: 'team-1' }), /Already in Linear as ENG-7/);
});

test('a send that already made its issue links that issue instead of making another', async () => {
  const h = harness({
    tools: {
      // Fuzzy search: one unrelated hit, then the real one.
      linear__list_issues: () => ({ issues: [{ id: 'ENG-1', uuid: 'u1' }, { id: 'ENG-7', uuid: issue().uuid }] }),
      linear__get_issue: (a) => (a.id === 'ENG-1' ? issue({ id: 'ENG-1', uuid: 'u1', description: 'Something else' }) : issue({ description: `Notes\n\n---\nSent from Threadbear · ${marker('task-1')}` })),
    },
  });
  await h.l.send({ taskId: 'task-1', teamId: 'team-1' });
  const s = await settled(h);
  assert.equal(s.args.found, true);
  assert.equal((s.args.issue as { identifier: string }).identifier, 'ENG-7');
  assert.ok(!h.calls.some((c) => c.name === 'linear__save_issue'));
});

test('one picture that fails is left out and the issue is still made', async () => {
  const h = harness({ tools: { failImage: () => null } });
  await h.l.send({ taskId: 'task-1', teamId: 'team-1', imageIds: ['st_a', 'st_b'] });
  const s = await settled(h);
  assert.ok(s.args.issue);
  assert.equal(h.puts.length, 1);
});

test('a create held for approval waits on it, then settles', async () => {
  let held = 2;
  const h = harness({
    tools: {
      linear__save_issue: () => ({ waiting: { id: 'ap_1', url: 'http://127.0.0.1:8795/approvals/ap_1' } }),
      connectors__await_approval: () => (held-- > 0 ? { waiting: { id: 'ap_1', url: 'http://127.0.0.1:8795/approvals/ap_1' } } : { id: 'ENG-7' }),
    },
  });
  await h.l.send({ taskId: 'task-1', teamId: 'team-1' });
  const s = await settled(h);
  assert.ok(h.backend.some((b) => b.op === 'waiting' && b.args.url === 'http://127.0.0.1:8795/approvals/ap_1'));
  assert.equal(h.calls.filter((c) => c.name === 'connectors__await_approval').length, 3);
  assert.equal((s.args.issue as { identifier: string }).identifier, 'ENG-7');
});

test('an error during a send settles the claim as failed', async () => {
  const h = harness({ tools: { linear__save_issue: () => new LinearError('{"message":"Team is archived"}') } });
  await h.l.send({ taskId: 'task-1', teamId: 'team-1' });
  const s = await settled(h);
  assert.equal(s.args.error, 'Team is archived');
});

test('refresh copies the issue and its comments, and a gone issue stops sync', async () => {
  const linked = task({ linear: { state: 'linked', identifier: 'ENG-7', issueId: issue().uuid, requestId: 'r', claimedAt: 1, teamId: 'team-1', teamName: 'E', sync: true } });
  const h = harness({ tasks: [linked] });
  await h.l.refresh({ taskId: 'task-1' });
  const apply = h.backend.find((b) => b.op === 'apply')!;
  assert.equal((apply.args.issue as { statusType: string }).statusType, 'unstarted');
  assert.deepEqual(apply.args.comments, [{ id: 'c1', body: 'Looks good', author: 'Sam', createdAt: '2026-11-20T11:00:00Z' }]);
  const gone = harness({ tasks: [linked], tools: { linear__get_issue: () => new LinearError('{"message":"Could not find referenced Issue."}') } });
  assert.deepEqual(await gone.l.refresh({ taskId: 'task-1' }), { changed: [], missing: true });
  assert.ok(gone.backend.some((b) => b.op === 'missing'));
});

test('the sweep asks each team once what changed, and reads only those issues', async () => {
  const h = harness({ tools: { linear__list_issues: () => ({ issues: [{ id: 'ENG-7', uuid: issue().uuid, updatedAt: 'x' }, { id: 'ENG-9', uuid: 'other' }] }) } });
  h.setClaim({ since: Date.parse('2026-11-20T09:00:00Z') });
  const r = await h.l.sweep();
  assert.deepEqual(r, { checked: 1, changed: 1 });
  const list = h.calls.filter((c) => c.name === 'linear__list_issues');
  assert.equal(list.length, 1);
  assert.equal(list[0]!.args.updatedAt, '2026-11-20T08:59:00.000Z');
  assert.equal(h.calls.filter((c) => c.name === 'linear__get_issue').length, 1);
});

test('the sweep does nothing between intervals, or with no synced tasks', async () => {
  const h = harness();
  h.setClaim(null);
  assert.equal(await h.l.sweep(), undefined);
  assert.equal(h.calls.length, 0);
  const none = harness();
  const linear = none.l as unknown as { deps: { linear: (op: string) => Promise<unknown> } };
  const real = linear.deps.linear;
  linear.deps.linear = async (op: string) => (op === 'synced' ? [] : real(op));
  assert.deepEqual(await none.l.sweep(), { checked: 0, changed: 0 });
  assert.ok(!none.backend.some((b) => b.op === 'claimSweep'));
  assert.equal(none.calls.length, 0);
});

test('search marks issues that already have a task, and a pasted key goes straight to it', async () => {
  const h = harness({ tools: { linear__list_issues: () => ({ issues: [issue(), issue({ id: 'ENG-8', uuid: 'done-uuid', statusType: 'completed' })], hasNextPage: true, cursor: 'c2' }) } });
  const r = await h.l.search({ query: 'thing' });
  assert.equal(r.issues.length, 1);
  assert.equal(r.cursor, 'c2');
  assert.equal(h.calls.at(-1)!.args.assignee, 'me');
  const k = harness();
  const byKey = await k.l.search({ query: 'https://linear.app/acme/issue/ENG-7/x' });
  assert.equal(byKey.issues[0]!.identifier, 'ENG-7');
  assert.ok(!k.calls.some((c) => c.name === 'linear__list_issues'));
});

test('import brings the issue, its description and comments, and sub-issues when asked', async () => {
  const h = harness({ tools: { linear__list_issues: () => ({ issues: [issue({ id: 'ENG-8', uuid: 'child-uuid', title: 'Child' })] }), linear__get_issue: (a) => (a.id === 'ENG-8' ? issue({ id: 'ENG-8', uuid: 'child-uuid', title: 'Child', description: 'kid' }) : issue({ description: 'From Linear' })) } });
  await h.l.import({ issues: ['ENG-7'], project: null, includeSubIssues: true });
  const imp = h.backend.find((b) => b.op === 'importIssues')!;
  const items = imp.args.issues as { issue: { identifier: string }; description: string; parentIssueId?: string; comments: unknown[] }[];
  assert.deepEqual(items.map((i) => i.issue.identifier), ['ENG-7', 'ENG-8']);
  assert.equal(items[0]!.description, 'From Linear');
  assert.equal(items[1]!.parentIssueId, issue().uuid);
  assert.equal(items[0]!.comments.length, 1);
  assert.equal(imp.args.sync, true);
});

test('link takes a key or URL and never writes to Linear', async () => {
  const h = harness();
  assert.deepEqual(await h.l.link({ taskId: 'task-1', issue: 'eng-7' }), { identifier: 'ENG-7' });
  assert.ok(h.backend.some((b) => b.op === 'link'));
  assert.ok(!h.calls.some((c) => c.name === 'linear__save_issue'));
  await assert.rejects(h.l.link({ taskId: 'task-1', issue: 'not a key' }), /Paste an issue key/);
});

test('the caller falls back on the main login when the Threadbear one is missing', async () => {
  const seen: string[] = [];
  const call = linearCaller({
    mcpUrl: 'http://127.0.0.1:8795/mcp',
    headers: async () => {
      throw new Error('no item');
    },
    fallback: async () => ({ Authorization: 'Bearer main' }),
    fetch: (async (_u: unknown, init?: RequestInit) => {
      seen.push((init?.headers as Record<string, string>).Authorization!);
      return new Response(JSON.stringify({ jsonrpc: '2.0', id: 1, result: { content: [{ type: 'text', text: 'Waiting for Wayne to approve this at http://127.0.0.1:8795/approvals/ap_9.' }] } }));
    }) as typeof fetch,
  });
  const r = await call('linear__save_issue', {});
  assert.deepEqual(r, { waiting: { id: 'ap_9', url: 'http://127.0.0.1:8795/approvals/ap_9' } });
  assert.deepEqual(seen, ['Bearer main']);
});
