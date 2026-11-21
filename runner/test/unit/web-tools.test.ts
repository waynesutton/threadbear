import { test } from 'node:test';
import assert from 'node:assert/strict';
import { toolKind, toolTitle } from '../../src/tools.ts';

// Web search and reads through an MCP server read as web lines, like
// WebSearch and WebFetch, whether installed directly or through Connectors.
const PREFIXES = ['mcp__connectors__firecrawl__', 'mcp__firecrawl__'];
const EXA = ['mcp__connectors__exa__', 'mcp__exa__'];

const cases: [string[], string, Record<string, unknown>, string][] = [
  [PREFIXES, 'firecrawl_search', { query: 'convex agents' }, 'Search the web for convex agents'],
  [EXA, 'web_search_exa', { query: 'gpui' }, 'Search the web for gpui'],
  [EXA, 'web_search_advanced_exa', { query: 'bun 2' }, 'Search the web for bun 2'],
  [PREFIXES, 'firecrawl_scrape', { url: 'https://docs.convex.dev' }, 'Read https://docs.convex.dev'],
  [EXA, 'web_fetch_exa', { urls: ['https://a.dev', 'https://b.dev'] }, 'Read https://a.dev'],
  [PREFIXES, 'firecrawl_map', { url: 'https://convex.dev' }, 'Map https://convex.dev'],
];

for (const [prefixes, tool, input, title] of cases) {
  for (const prefix of prefixes) {
    test(`${prefix}${tool} is a web line`, () => {
      assert.equal(toolKind(`${prefix}${tool}`), 'web');
      assert.equal(toolTitle(`${prefix}${tool}`, input, '/tmp'), title);
    });
  }
}

test('a treg web search or extract reads as web, its kind stays mcp', () => {
  for (const name of ['mcp__connectors__treg__call', 'mcp__treg__call']) {
    assert.equal(toolKind(name), 'mcp');
    assert.equal(toolTitle(name, { endpoint_id: 'web.brave.search', params: { q: 'threadbear' } }, '/tmp'), 'Search the web for threadbear');
    assert.equal(toolTitle(name, { endpoint_id: 'exa.web.answer', body: { query: 'what is convex' } }, '/tmp'), 'Search the web for what is convex');
    assert.equal(toolTitle(name, { endpoint_id: 'tavily.web.extract', params: { urls: ['https://x.dev'] } }, '/tmp'), 'Read https://x.dev');
  }
});

test('a treg call that is not web stays generic', () => {
  assert.equal(toolTitle('mcp__connectors__treg__call', { endpoint_id: 'dataforseo.serp.google', params: { q: 'x' } }, '/tmp'), 'connectors: treg__call');
  assert.equal(toolTitle('mcp__connectors__treg__call', { endpoint_id: 'render/v1/services' }, '/tmp'), 'connectors: treg__call');
});

test('other MCP tools and threadbear tools are unaffected', () => {
  assert.equal(toolKind('mcp__connectors__firecrawl__firecrawl_crawl'), 'mcp');
  assert.equal(toolKind('mcp__threadbear__firecrawl_search'), 'mcp');
  assert.equal(toolTitle('mcp__threadbear__pin', { title: 'Plan' }, '/tmp'), 'Pin Plan');
  assert.equal(toolKind('mcp__threadbear__pin'), 'mcp');
});
