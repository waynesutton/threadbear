// Plan board screens (src/board-frames.ts, experiments/planBoardMockups.md):
// the policy first in every stored file, scripts and active SVG refused,
// size caps, loopback routes only, versions kept, and the tools only with
// both planBoard and planBoardMockups on.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { makeHarness, tmpDir } from '../helpers/fakes.ts';
import { Boards, BoardError, type BoardTools } from '../../src/boards.ts';
import { BoardFrames, FRAME_POLICY, MAX_FRAME_VERSIONS, loopbackRoute, passiveHtml, withPolicy } from '../../src/board-frames.ts';
import { buildThreadbearTools } from '../../src/threadbear-mcp.ts';

const TID = '5533c870-aaaa-bbbb-cccc-000000000001';
const PLAN = 'flowchart LR\n summary[GET /api/summary] --> kpi[KPI cards]';
const PAGE = '<!doctype html><html><head><title>KPI</title><style>.a{color:red}</style></head><body><div class="a">KPI</div></body></html>';

function setup() {
  const boards = new Boards(path.join(tmpDir(), 'boards'));
  boards.update(TID, { mermaid: PLAN });
  return { boards, frames: new BoardFrames(boards) };
}

test('frames: the policy is the first thing in <head>, with scripts off', () => {
  const out = withPolicy(PAGE);
  assert.ok(out.indexOf('Content-Security-Policy') < out.indexOf('<title>'));
  assert.ok(out.includes("script-src 'none'"));
  assert.match(withPolicy('<p>bare</p>'), /^<!doctype html><html><head><meta http-equiv="Content-Security-Policy"/);
  assert.ok(FRAME_POLICY.startsWith("default-src 'none'"));
});

test('frames: scripts, handlers, outside CSS and active SVG are refused', () => {
  assert.throws(() => passiveHtml('<script>alert(1)</script>'), /<script>/);
  assert.throws(() => passiveHtml('<img src=x onerror="x()">'), /event attribute/);
  assert.throws(() => passiveHtml('<a href="javascript:x()">x</a>'), /javascript:/);
  assert.throws(() => passiveHtml('<iframe src="https://x"></iframe>'), /embeds/);
  assert.throws(() => passiveHtml('<link rel="stylesheet" href="https://cdn.tailwindcss.com/x.css">'), /network/);
  assert.throws(() => passiveHtml('<svg><animate attributeName="x"/></svg>'), /animates/);
  assert.throws(() => passiveHtml('<svg><rect fill="url(https://x/y)"/></svg>'), /outside url/);
  assert.throws(() => passiveHtml('<svg><use href="https://x/s.svg#a"/></svg>'), /links outside/);
  passiveHtml('<svg><rect fill="url(#g)"/><use href="#a"/></svg>');
  passiveHtml(PAGE);
});

test('frames: over 2 MB is refused', () => {
  assert.throws(() => passiveHtml(`<p>${'x'.repeat(2 * 1024 * 1024)}</p>`), /under 2 MB/);
});

test('frames: a route is the dev server on this Mac, nothing else', () => {
  assert.equal(loopbackRoute('http://localhost:5173/__mockups/kpi?v=b'), 'http://localhost:5173/__mockups/kpi?v=b');
  assert.ok(loopbackRoute('http://127.0.0.1:3000/x'));
  assert.throws(() => loopbackRoute('https://example.com:443/x'), /localhost or 127\.0\.0\.1/);
  assert.throws(() => loopbackRoute('http://localhost/x'), /port/);
  assert.throws(() => loopbackRoute('file:///etc/passwd'), /http page/);
  assert.throws(() => loopbackRoute('http://u:p@localhost:5173/'), /sign-in/);
});

test('frames: three takes at most, versions kept and capped, a pick marks one', () => {
  const { frames } = setup();
  const a = frames.write(TID, { node: 'kpi', frame: 'row', variant: 'A', title: 'Row', html: PAGE });
  frames.write(TID, { node: 'kpi', frame: 'row', variant: 'B', title: 'Row', html: PAGE });
  assert.throws(() => frames.write(TID, { node: 'kpi', frame: 'row', variant: 'D', title: 'Row', html: PAGE }), /A, B or C/);
  assert.throws(() => frames.write(TID, { node: 'ghost', frame: 'row', title: 'x', html: PAGE }), /no box/);
  assert.throws(() => frames.write(TID, { node: 'kpi', frame: 'row', title: 'x' }), /one of them/);
  for (let i = 0; i < 11; i++) frames.write(TID, { node: 'kpi', frame: 'row', variant: 'A', title: 'Row', html: PAGE });
  const list = frames.list(TID, 'kpi');
  const ra = list.find((f) => f.id === a.id)!;
  assert.equal(ra.version, 12);
  assert.equal(ra.changed, true);
  const kept = fs.readdirSync(path.dirname(path.dirname(ra.path!)));
  assert.equal(kept.length, MAX_FRAME_VERSIONS);
  assert.ok(fs.readFileSync(ra.path!, 'utf8').includes('Content-Security-Policy'));
  frames.pick(TID, list.find((f) => f.variant === 'B')!.id);
  assert.deepEqual(frames.list(TID, 'kpi').map((f) => [f.variant, Boolean(f.picked)]), [['A', false], ['B', true]]);
  frames.seen(TID);
  assert.equal(frames.list(TID, 'kpi')[0].changed, false);
});

test('frames: a script is refused and nothing is written', () => {
  const { frames, boards } = setup();
  assert.throws(() => frames.write(TID, { node: 'kpi', frame: 'row', title: 'x', html: '<script>x</script>' }), BoardError);
  assert.equal(fs.existsSync(path.join(boards.dir, TID, 'frames')), false);
});

test('frames: the tools only with planBoard and planBoardMockups both on', () => {
  const h = makeHarness();
  const { thread } = h.runner.create({ text: 'plan', accountId: 'claude-3' });
  const boards = new Boards(path.join(h.home, 'boards'));
  const control: BoardTools = { read: (id) => boards.get(id), write: (id, u) => boards.update(id, u), mustSend: (id, n) => boards.mustSend(id, n), linkThread: () => {}, statusOf: () => undefined };
  const frames = new BoardFrames(boards);
  const names = (b: boolean, f: boolean) => buildThreadbearTools(h.runner, thread.id, Date.now, undefined, undefined, undefined, { boards: () => (b ? control : undefined), frames: () => (f ? frames : undefined) }).map((t) => t.name);
  assert.ok(names(true, true).includes('update_frame') && names(true, true).includes('read_frames'));
  assert.ok(!names(true, false).includes('update_frame'), 'mockups off');
  assert.ok(!names(false, true).includes('update_frame'), 'planBoard off');
  assert.ok(!names(false, false).includes('read_frames'));
});
