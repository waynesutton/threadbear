import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { darkOnly, paletteFrom, PALETTES } from './theme.ts';

test('an unknown or missing saved palette is the stock look', () => {
  assert.equal(paletteFrom(null), 'stock');
  assert.equal(paletteFrom(''), 'stock');
  assert.equal(paletteFrom('gone'), 'stock');
  for (const p of PALETTES) assert.equal(paletteFrom(p), p);
});

test('only Squall is dark only', () => {
  assert.deepEqual(PALETTES.filter(darkOnly), ['squall']);
});

// Squall has to win over data-theme="light": its block sets a dark
// color-scheme with the same weight as the light rule, so it must come later.
test('every palette has its colours in styles.css, after the light and dark rules', () => {
  const css = readFileSync(new URL('./styles.css', import.meta.url), 'utf8');
  const light = css.indexOf(":root[data-theme='light']");
  assert.ok(light > 0);
  for (const p of PALETTES.filter((p) => p !== 'stock')) {
    const at = css.indexOf(`:root[data-palette='${p}']`);
    assert.ok(at > light, `${p} comes after the light rule`);
    const block = css.slice(at, css.indexOf('}', at));
    for (const v of ['--bg', '--side', '--text', '--accent', '--code']) assert.ok(block.includes(`${v}:`), `${p} sets ${v}`);
  }
  const squall = css.slice(css.indexOf(":root[data-palette='squall']"));
  assert.match(squall.slice(0, squall.indexOf('}')), /color-scheme: dark/);
});
