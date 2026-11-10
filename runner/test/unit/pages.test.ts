import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { MAX_HTML_BYTES, MAX_VERSIONS, PageError, Pages } from '../../src/pages.ts';
import type { ThreadProject } from '../../src/protocol.ts';

const home = () => fs.mkdtempSync(path.join(os.tmpdir(), 'tb-pages-'));
const proj = (id: string, pinned: boolean): ThreadProject => ({ id, title: id, by: 'agent', at: '2026-11-10T00:00:00.000Z', ...(pinned ? { pinned } : {}) });
const page = (body: string) => `<!doctype html><title>t</title><body>${body}</body>`;

test('create writes v1, update bumps the version in place, and both survive a reload', () => {
  const h = home();
  let t = Date.parse('2026-11-10T00:00:00Z');
  const pages = new Pages(h, () => t);
  const p = pages.create({ title: 'Bundle size', summary: '20.22 MB', ownerThread: 'th1', html: page('one'), source: 'bundle-analyzer summary' });
  assert.equal(p.id, 'pg_bundle_size');
  assert.equal(p.version, 1);
  assert.match(fs.readFileSync(pages.file(p.id), 'utf8'), /one/);
  t += 1000;
  const u = pages.update(p.id, { html: page('two'), summary: '19.8 MB' });
  assert.equal(u.version, 2);
  assert.equal(u.summary, '19.8 MB');
  assert.match(fs.readFileSync(pages.file(p.id), 'utf8'), /two/);
  assert.match(fs.readFileSync(pages.file(p.id, 1), 'utf8'), /one/);
  const again = new Pages(h);
  assert.equal(again.get(p.id)?.version, 2);
  assert.equal(again.list({ source: 'bundle-analyzer summary' }).length, 1);
});

test('a title-only update keeps the version', () => {
  const pages = new Pages(home());
  const p = pages.create({ title: 'Progress', summary: '', ownerThread: 'th1', html: page('x') });
  assert.equal(pages.update(p.id, { title: 'Render progress' }).version, 1);
});

test('only the newest versions are kept', () => {
  const pages = new Pages(home());
  const p = pages.create({ title: 'P', summary: '', ownerThread: 'th1', html: page('0') });
  for (let i = 1; i <= MAX_VERSIONS + 2; i++) pages.update(p.id, { html: page(String(i)) });
  const newest = MAX_VERSIONS + 3;
  assert.ok(fs.existsSync(pages.file(p.id, newest)));
  assert.ok(fs.existsSync(pages.file(p.id, newest - MAX_VERSIONS + 1)));
  assert.ok(!fs.existsSync(pages.file(p.id, newest - MAX_VERSIONS)));
  assert.ok(!fs.existsSync(pages.file(p.id, 1)));
});

test('static pages refuse scripts; live pages keep them', () => {
  const pages = new Pages(home());
  assert.throws(() => pages.create({ title: 'S', summary: '', ownerThread: 't', html: page('<SCRIPT>1</script>') }), PageError);
  assert.equal(pages.list().length, 0);
  const live = pages.create({ title: 'L', summary: '', ownerThread: 't', html: page('<script>1</script>'), live: true });
  assert.equal(live.live, true);
});

test('size caps and the html-or-dir rule', () => {
  const pages = new Pages(home());
  assert.throws(() => pages.create({ title: 'Big', summary: '', ownerThread: 't', html: 'x'.repeat(MAX_HTML_BYTES + 1) }), /most is/);
  assert.throws(() => pages.create({ title: 'None', summary: '', ownerThread: 't' }), /one of them/);
  assert.throws(() => pages.create({ title: '  ', summary: '', ownerThread: 't', html: page('x') }), /title/);
});

test('a folder page copies its assets and refuses links, scripts and data files', () => {
  const pages = new Pages(home());
  const src = fs.mkdtempSync(path.join(os.tmpdir(), 'tb-page-src-'));
  fs.writeFileSync(path.join(src, 'index.html'), page('<img src="fig.svg">'));
  fs.writeFileSync(path.join(src, 'fig.svg'), '<svg/>');
  fs.mkdirSync(path.join(src, 'clip'));
  fs.writeFileSync(path.join(src, 'clip', 'clip.mp4'), Buffer.alloc(10));
  const p = pages.create({ title: 'Folder', summary: '', ownerThread: 't', dir: src });
  const v = path.dirname(pages.file(p.id));
  assert.ok(fs.existsSync(path.join(v, 'fig.svg')));
  assert.ok(fs.existsSync(path.join(v, 'clip', 'clip.mp4')));

  fs.writeFileSync(path.join(src, 'data.json'), '{}');
  assert.throws(() => pages.update(p.id, { dir: src }), /data\.json/);
  fs.rmSync(path.join(src, 'data.json'));
  fs.writeFileSync(path.join(src, 'app.js'), '');
  assert.throws(() => pages.update(p.id, { dir: src }), /app\.js/);
  fs.rmSync(path.join(src, 'app.js'));
  fs.symlinkSync('/etc/hosts', path.join(src, 'hosts.html'));
  assert.throws(() => pages.update(p.id, { dir: src }), /link/);
  assert.equal(pages.get(p.id)?.version, 1);
});

test('projects: one pinned project is inherited, none or several are not, [] means none', () => {
  const pages = new Pages(home());
  const one = pages.create({ title: 'A', summary: '', ownerThread: 't', html: page('a'), ownerProjects: [proj('p_tb', true), proj('p_misc', false)] });
  assert.deepEqual(one.projects.map((p) => p.id), ['p_tb']);
  const two = pages.create({ title: 'B', summary: '', ownerThread: 't', html: page('b'), ownerProjects: [proj('p_tb', true), proj('p_poddle', true)] });
  assert.deepEqual(two.projects, []);
  const guessed = pages.create({ title: 'C', summary: '', ownerThread: 't', html: page('c'), ownerProjects: [proj('p_tb', false)] });
  assert.deepEqual(guessed.projects, []);
  const none = pages.create({ title: 'D', summary: '', ownerThread: 't', html: page('d'), ownerProjects: [proj('p_tb', true)], projects: [] });
  assert.deepEqual(none.projects, []);
  const asked = pages.create({ title: 'E', summary: '', ownerThread: 't', html: page('e'), projects: ['p_poddle', 'p_tb'] });
  assert.deepEqual(asked.projects.map((p) => p.id), ['p_poddle', 'p_tb']);
});

test('setProjects adds and removes, and list filters by project, thread and none', () => {
  const pages = new Pages(home());
  const a = pages.create({ title: 'A', summary: '', ownerThread: 't1', html: page('a'), projects: [] });
  pages.create({ title: 'B', summary: '', ownerThread: 't2', html: page('b'), projects: ['p_tb'] });
  pages.setProjects(a.id, { add: ['p_tb', 'p_poddle'] });
  assert.equal(pages.list({ project: 'p_poddle' }).length, 1);
  assert.equal(pages.list({ project: 'p_tb' }).length, 2);
  pages.setProjects(a.id, { remove: ['p_tb', 'p_poddle'] });
  assert.deepEqual(pages.list({ none: true }).map((p) => p.title), ['A']);
  assert.deepEqual(pages.list({ thread: 't2' }).map((p) => p.title), ['B']);
});

// The tools, against a fake runner that knows one thread pinned to a project.
type Tool = { name: string; handler: (a: never, e: never) => Promise<{ content: { text: string }[]; isError?: boolean }> };
const call = async (tools: Tool[], name: string, args: Record<string, unknown>) => {
  const r = await tools.find((t) => t.name === name)!.handler(args as never, {} as never);
  return { text: r.content[0]!.text, err: r.isError === true };
};

test('page tools: off without the experiment; create inherits the pinned project and update keeps one page', async () => {
  const { buildThreadbearTools } = await import('../../src/threadbear-mcp.ts');
  const h = home();
  const cwd = fs.mkdtempSync(path.join(os.tmpdir(), 'tb-page-cwd-'));
  const thread = { id: 'th1', title: 'Bundle', cwd, projects: [proj('p_tb', true)] };
  const control = { list: async () => [thread], get: async () => ({ ...thread, permissionMode: 'full' }) } as never;
  const projects = { find: async (q: string) => (q === 'Threadbear' || q === 'p_tb' ? { id: 'p_tb', title: 'Threadbear' } : undefined) } as never;
  const off = buildThreadbearTools(control, 'th1', Date.now, undefined, projects) as unknown as Tool[];
  assert.ok(!off.some((t) => t.name === 'create_page'));

  const store = new Pages(h);
  const tools = buildThreadbearTools(control, 'th1', Date.now, undefined, projects, undefined, { pages: () => store }) as unknown as Tool[];
  const made = await call(tools, 'create_page', { title: 'Bundle size', summary: '20.22 MB', html: page('v1'), source: 'bundle-analyzer summary' });
  assert.equal(made.err, false, made.text);
  assert.match(made.text, /pg_bundle_size {2}v1 .*in p_tb/);
  const bad = await call(tools, 'create_page', { title: 'X', summary: '', html: page('<script></script>') });
  assert.equal(bad.err, true);

  fs.mkdirSync(path.join(cwd, 'out'));
  fs.writeFileSync(path.join(cwd, 'out', 'index.html'), page('v2'));
  const up = await call(tools, 'update_page', { id: 'pg_bundle', dir: 'out' });
  assert.match(up.text, /v2/);
  assert.match(fs.readFileSync(store.file('pg_bundle_size'), 'utf8'), /v2/);

  assert.match((await call(tools, 'list_pages', { project: 'Threadbear' })).text, /pg_bundle_size/);
  assert.equal((await call(tools, 'list_pages', { none: true })).text, 'No pages.');
  await call(tools, 'remove_page_from_project', { id: 'pg_bundle_size', project: 'Threadbear' });
  assert.match((await call(tools, 'read_page', { id: 'pg_bundle_size' })).text, /no project[\s\S]*file: .*v2\/index\.html/);
});

test('pages.* methods: empty while off, list, get with file and versions, setProjects as Mike, and a change notifies', async () => {
  const { pageMethods } = await import('../../src/pages.ts');
  const offM = pageMethods(() => undefined);
  assert.deepEqual(offM['pages.list']!({}), { pages: [] });
  assert.throws(() => offM['pages.get']!({ pageId: 'x' }), /Pages is off/);

  const seen: number[] = [];
  const store = new Pages(home(), Date.now, (list) => seen.push(list.length));
  const m = pageMethods(() => store);
  const p = store.create({ title: 'Bundle size', summary: 's', ownerThread: 't1', html: page('1'), projects: [] });
  store.update(p.id, { html: page('2') });
  assert.deepEqual(seen, [1, 1]);
  const got = m['pages.get']!({ pageId: 'pg_bundle' }) as { page: { version: number }; file: string; versions: number[] };
  assert.equal(got.page.version, 2);
  assert.deepEqual(got.versions, [1, 2]);
  assert.match(fs.readFileSync(got.file, 'utf8'), /2/);
  const moved = m['pages.setProjects']!({ pageId: p.id, add: ['p_tb'] }) as { page: { projects: { id: string; by: string }[] } };
  assert.deepEqual(moved.page.projects.map((x) => [x.id, x.by]), [['p_tb', 'mike']]);
  assert.equal((m['pages.list']!({ project: 'p_tb' }) as { pages: unknown[] }).pages.length, 1);
  assert.equal((m['pages.list']!({ none: true }) as { pages: unknown[] }).pages.length, 0);
  assert.throws(() => m['pages.get']!({ pageId: 'nope' }), /No page nope/);
  assert.throws(() => m['pages.setProjects']!({ pageId: p.id, add: 'p_tb' }), /list of strings/);
});
