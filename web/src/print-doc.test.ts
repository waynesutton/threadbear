import { test } from 'node:test';
import assert from 'node:assert/strict';
import { printDoc, PRINT_POLICY } from './print-doc.ts';

test('the policy goes in front of the print, after its doctype', () => {
  const out = printDoc('<!doctype html>\n<html><head><title>x</title></head><body>hi</body></html>');
  assert.ok(out.startsWith('<!doctype html><meta http-equiv="Content-Security-Policy"'));
  assert.ok(out.indexOf(PRINT_POLICY) < out.indexOf('<html>'));
  assert.ok(out.includes('<base target="_blank">'));
  assert.ok(printDoc('<p>bare</p>').startsWith('<meta http-equiv="Content-Security-Policy"'));
});

test('the policy allows nothing from the network and no scripts', () => {
  assert.match(PRINT_POLICY, /default-src 'none'/);
  assert.doesNotMatch(PRINT_POLICY, /script-src|connect-src|https?:/);
});

const PRINT = '<!doctype html>\n<html lang="en"><head><style>:root{--bg:#000}@media (prefers-color-scheme: light){:root{--bg:#fff}}@media (prefers-color-scheme:dark) and (max-width:600px){p{x:y}}</style></head><body>hi</body></html>';

test('the system look leaves the print as it was made', () => {
  assert.equal(printDoc(PRINT).includes('@media (prefers-color-scheme: light)'), true);
  assert.doesNotMatch(printDoc(PRINT), /data-theme/);
});

test('a forced look settles the colour scheme query in the CSS, for any browser and old prints too', () => {
  const light = printDoc(PRINT, 'light');
  assert.match(light, /<html data-theme="light" lang="en">/);
  assert.match(light, /@media \(min-width: 0px\)\{:root\{--bg:#fff\}\}/, 'the light rules always apply');
  assert.match(light, /@media \(max-width: -1px\) and \(max-width:600px\)/, 'the dark ones never do');
  const dark = printDoc(PRINT, 'dark');
  assert.match(dark, /<html data-theme="dark"/);
  assert.match(dark, /@media \(max-width: -1px\)\{:root\{--bg:#fff\}\}/);
  assert.doesNotMatch(dark, /prefers-color-scheme/);
  assert.ok(dark.startsWith('<!doctype html><meta http-equiv="Content-Security-Policy"'), 'the policy still comes first');
});
