#!/usr/bin/env node
// One step from a Convex app to a finished bundle size page folder:
//
//   node report.mjs <app dir> <out dir> [--app "name"] [--title "Bundle size"]
//
// Runs @convex-dev/bundle-analyzer (pinned) in <app dir>, saves its JSON in
// <out dir>/../<name>-json, fetches the import chains of every duplicated
// package (at most 4), and builds <out dir>/index.html. Prints the out dir
// and a one-line summary to use as the page's summary.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const ANALYZER = "@convex-dev/bundle-analyzer@0.1.0";
const HERE = dirname(fileURLToPath(import.meta.url));

const args = process.argv.slice(2);
const flag = (name) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};
const positional = args.filter((a, i) => !a.startsWith("--") && !args[i - 1]?.startsWith("--"));
const [appDir, outDir] = positional.map((p) => resolve(p));
if (!appDir || !outDir) {
  console.error('usage: node report.mjs <app dir> <out dir> [--app "name"] [--title "Bundle size"]');
  process.exit(2);
}
if (!existsSync(join(appDir, "convex"))) {
  console.error(`${appDir} has no convex/ folder. Point this at the Convex app's root.`);
  process.exit(2);
}
if (!existsSync(join(appDir, "convex/_generated"))) {
  console.error(`${appDir}/convex/_generated is missing. Run \`npx convex codegen\` there first (ask the user before you do).`);
  process.exit(3);
}

const jsonDir = join(dirname(outDir), `${basename(outDir)}-json`);
mkdirSync(jsonDir, { recursive: true });

const analyze = (...cmd) => {
  try {
    return execFileSync("npx", ["-y", ANALYZER, ...cmd, "--json"], { cwd: appDir, encoding: "utf8", maxBuffer: 64 * 1024 * 1024, stdio: ["ignore", "pipe", "pipe"] });
  } catch (e) {
    console.error(`bundle-analyzer ${cmd.join(" ")} failed:\n${e.stderr ?? e.message}`);
    process.exit(4);
  }
};

const summaryText = analyze("summary");
writeFileSync(join(jsonDir, "summary.json"), summaryText);
const summary = JSON.parse(summaryText);
const dups = [...new Set((summary.insights ?? []).filter((i) => i.kind === "duplicate-package").map((i) => i.subject))].slice(0, 4);
for (const name of dups) {
  const safe = name.replace(/[^a-z0-9@._-]+/gi, "_");
  writeFileSync(join(jsonDir, `pkg-${safe}.json`), analyze("packages", name));
}

const build = ["build-page.mjs", jsonDir, outDir];
for (const f of ["--app", "--title"]) if (flag(f)) build.push(f, flag(f));
execFileSync(process.execPath, build, { cwd: HERE, stdio: ["ignore", "inherit", "inherit"] });

const t = summary.totals ?? {};
const mb = (b) => (b >= 1e6 ? `${(b / 1e6).toFixed(2)} MB` : `${(b / 1e3).toFixed(1)} KB`);
const line = `${mb(t.bytes ?? 0)} total, ${mb((t.codeBytes ?? 0) + (t.sourceMapBytes ?? 0))} uploaded per push. ${dups.length ? `${dups.length} ${dups.length === 1 ? "package" : "packages"} bundled more than once (${dups.join(", ")}).` : "Nothing bundled twice."} ${(summary.insights ?? []).length} insights.`;
writeFileSync(join(jsonDir, "summary-line.txt"), line + "\n");
console.log(`summary: ${line}`);
console.log(`dir: ${outDir}`);

