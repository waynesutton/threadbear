// node --test test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { capacityFor, cratesOf, scaleSvg } from "./figure.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const sample = JSON.parse(readFileSync(join(HERE, "fixtures/sample/summary.json"), "utf8"));

const build = (jsonDir) => {
  const out = mkdtempSync(join(tmpdir(), "br-out-"));
  execFileSync(process.execPath, [join(HERE, "build-page.mjs"), jsonDir, out, "--app", "test"], { stdio: "pipe" });
  return readFileSync(join(out, "index.html"), "utf8");
};

test("capacity is the next round figure with headroom", () => {
  assert.equal(capacityFor(1.28e6), 2);
  assert.equal(capacityFor(20.22e6), 25);
  assert.equal(capacityFor(0.3e6), 0.5);
  assert.equal(capacityFor(1.9e6), 5);
});

test("crates: biggest first, at most 8, duplicates flagged by version or insight", () => {
  const crates = cratesOf(sample);
  assert.equal(crates.length, 8);
  assert.equal(crates[0].name, "convex");
  assert.ok(crates.every((c, i) => i === 0 || crates[i - 1].bytes >= c.bytes));
  assert.deepEqual(crates.filter((c) => c.dup).map((c) => c.label), ["iconv-lite@0.7.3", "iconv-lite@0.6.3"]);
});

test("the figure names every crate and the reading in its label", () => {
  const svg = scaleSvg(sample);
  assert.match(svg, /reads 20\.22 MB on a 25 MB scale/);
  assert.match(svg, /bundled more than once: iconv-lite/);
  assert.equal((svg.match(/class="br-dup"/g) ?? []).length, 2);
});

test("the page is static and carries the analyzer's numbers", () => {
  const html = build(join(HERE, "fixtures/sample"));
  assert.doesNotMatch(html, /<script\b/i);
  assert.match(html, /20\.22 MB/);
  assert.match(html, /17\.36 MB/);
  assert.match(html, /5 packages/);
  assert.match(html, /prefers-color-scheme: light/);
});

test("a clean bundle says so instead of listing nothing", () => {
  const dir = mkdtempSync(join(tmpdir(), "br-json-"));
  const clean = { ...sample, insights: [], topPackages: sample.topPackages.filter((p) => p.name !== "iconv-lite"), totals: { ...sample.totals, externalPackageBytes: 0 } };
  writeFileSync(join(dir, "summary.json"), JSON.stringify(clean));
  const html = build(dir);
  assert.match(html, /Nothing to fix/);
  assert.match(html, />None</);
  assert.doesNotMatch(html, /external 0/);
});

test("duplicated packages get an import chain diagram when their detail JSON is there", () => {
  const dir = mkdtempSync(join(tmpdir(), "br-json-"));
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "summary.json"), JSON.stringify(sample));
  const step = (display) => ({ path: display, display, specifier: null });
  writeFileSync(
    join(dir, "pkg-iconv-lite.json"),
    JSON.stringify({
      name: "iconv-lite",
      copies: [
        { build: "app:node", target: "iconv-lite@0.7.3", chains: [{ entryPoint: "convex/mail.ts", steps: [step("convex/mail.ts"), step("../node_modules/nodemailer/dist/index.js"), step("iconv-lite@0.7.3/lib/index.js")] }] },
        { build: "app:node", target: "iconv-lite@0.6.3", chains: [{ entryPoint: "convex/feed.ts", steps: [step("convex/feed.ts"), step("iconv-lite@0.6.3/lib/index.js")] }] },
      ],
    }),
  );
  const html = build(dir);
  assert.match(html, /Why they're bundled/);
  assert.match(html, /<details class="chain">/);
  assert.match(html, /nodemailer/);
  assert.doesNotMatch(html, /node_modules\//);
});
