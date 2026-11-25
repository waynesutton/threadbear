import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createFastLaneLookup, type FastLaneLookupOptions } from '../../src/fast-lane.ts';
import { parseSearch, readWriter, writerPrompt, type FastLaneChoice } from '../../src/fast-lane-models.ts';

const found = { status: 200, body: { results: [
  { title: 'npm', url: 'https://npm.example/convex', date: '2026-11-18', snippet: '1.51.0' },
  { title: 'Old', url: 'https://old.example', publish_date: null, excerpts: ['1.49.0'] },
  { title: 'No text', url: 'https://empty.example' },
] } };
const exaAnswer = { answer: 'From Exa.', citations: [{ title: 'E', url: 'https://e.example' }] };

// endpoint id to what treg says; a string is an error.
function lane(choice: FastLaneChoice, replies: Record<string, unknown>, writers: FastLaneLookupOptions['writers']) {
  const calls: string[] = [];
  const lookup = createFastLaneLookup({
    connectors: { mcpUrl: 'http://127.0.0.1:8795/mcp', headers: async () => ({}) },
    choice: () => choice,
    writers,
    fetch: (async (_url: unknown, init?: RequestInit) => {
      const id = (JSON.parse(String(init?.body)) as { params: { arguments: { endpoint_id: string } } }).params.arguments.endpoint_id;
      calls.push(id);
      const r = replies[id];
      const result = typeof r === 'string' ? { isError: true, content: [{ type: 'text', text: r }] } : { content: [{ type: 'text', text: JSON.stringify(r ?? {}) }] };
      return new Response(JSON.stringify({ jsonrpc: '2.0', id: 1, result }));
    }) as typeof fetch,
  });
  return { lookup, calls };
}
const says = (text: string) => async () => text;
const fails = async (): Promise<string> => { throw new Error('Free usage limit reached.'); };

test('fast-lane: Exa answers by default and no model is asked', async () => {
  const { lookup, calls } = lane('exa', { 'exa.web.answer': exaAnswer }, { hy4: fails, haiku: fails });
  const r = await lookup('q');
  assert.equal(r.source, 'Exa');
  assert.deepEqual(calls, ['exa.web.answer']);
});

test('fast-lane: a model writes from the search, with only the links it cited', async () => {
  let prompt = '';
  const { lookup, calls } = lane('hy4', { 'perplexity.web.search.fast': found }, { hy4: async (p) => { prompt = p.user; return 'It is 1.51.0 [1].'; } });
  const r = await lookup('latest convex');
  assert.equal(r.source, 'Hy4 Flash Free');
  assert.deepEqual(r.citations, [{ title: 'npm', url: 'https://npm.example/convex', numbers: [1], publishedDate: '2026-11-18' }]);
  assert.match(prompt, /Question: latest convex[\s\S]*\[2\] Old/);
  assert.deepEqual(calls, ['perplexity.web.search.fast']);
});

test('fast-lane: the second search steps in, then Exa, then the last model on the same search', async () => {
  const a = lane('hy4', { 'perplexity.web.search.fast': 'boom', 'parallel.web.search': found }, { hy4: says('ok [1]') });
  assert.equal((await a.lookup('q')).source, 'Hy4 Flash Free');
  assert.deepEqual(a.calls, ['perplexity.web.search.fast', 'parallel.web.search']);

  const b = lane('hy4', { 'perplexity.web.search.fast': found, 'exa.web.answer': exaAnswer }, { hy4: fails, haiku: says('x') });
  assert.equal((await b.lookup('q')).source, "Exa (Hy4 Flash Free didn't answer)");

  const c = lane('hy4', { 'perplexity.web.search.fast': found, 'exa.web.answer': 'down' }, { hy4: fails, haiku: says('Haiku says [1]') });
  assert.equal((await c.lookup('q')).source, "Claude Haiku (Hy4 Flash Free didn't answer)");
  assert.equal(c.calls.filter((x) => x === 'perplexity.web.search.fast').length, 1);

  const d = lane('exa', { 'perplexity.web.search.fast': found, 'exa.web.answer': 'down' }, { hy4: says('Hy4 says [1]') });
  assert.equal((await d.lookup('q')).source, "Hy4 Flash Free (Exa didn't answer)");
});

test('fast-lane: everything failing gives the first error, and a signed-out treg is called once', async () => {
  const a = lane('hy4', { 'perplexity.web.search.fast': found, 'exa.web.answer': 'down' }, { hy4: fails });
  await assert.rejects(a.lookup('q'), /Free usage limit/);
  const b = lane('hy4', { 'perplexity.web.search.fast': 'Unknown tool: treg__call' }, { hy4: says('x'), haiku: says('x') });
  await assert.rejects(b.lookup('q'), /Sign in to it in Connectors/);
  assert.equal(b.calls.length, 1);
});

test('fast-lane: Stop ends it without asking the backups', async () => {
  const ac = new AbortController();
  const { lookup, calls } = lane('hy4', { 'perplexity.web.search.fast': found, 'exa.web.answer': exaAnswer }, { hy4: () => { ac.abort(); return new Promise(() => undefined); } });
  await assert.rejects(lookup('q', { signal: ac.signal }));
  assert.ok(!calls.includes('exa.web.answer'));
});

test('fast-lane: search parsing, the prompt and the model answer', () => {
  const s = parseSearch(JSON.stringify(found));
  assert.equal(s.length, 2);
  assert.deepEqual(parseSearch('not json'), []);
  assert.match(writerPrompt('q', s, '2026-11-25').system, /Today is 2026-11-25/);
  assert.equal(readWriter('![x](https://evil.example)', s).answer, '[x](https://evil.example)');
  assert.equal(readWriter('No marks', s).citations.length, 2);
  assert.throws(() => readWriter('  ', s), /empty/);
});
