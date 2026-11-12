#!/usr/bin/env node
// Builds the bundle size Print from @convex-dev/bundle-analyzer's JSON.
//
//   node build-page.mjs <json dir> <out dir> [--title "Bundle size"] [--app "name"]
//
// <json dir> holds summary.json (`summary --json`) and, optionally,
// pkg-<name>.json files (`packages <name> --json`) for the import chains.
// <out dir> gets index.html: static HTML, CSS and SVG with no scripts, in
// the Split hero layout, following the viewer's light or dark appearance.
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { ISO_CSS, autoThemeCss, capacityFor, cratesOf, mb, scaleSvg } from "./figure.mjs";

const PELE = process.env.PELE_SKILL ?? join(homedir(), "Library/Application Support/Threadbear/skills/plugin/skills/pele");

const args = process.argv.slice(2);
const flag = (name) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};
const positional = args.filter((a, i) => !a.startsWith("--") && !args[i - 1]?.startsWith("--"));
const [jsonDir, outDir] = positional.map((p) => resolve(p));
if (!jsonDir || !outDir) {
  console.error('usage: node build-page.mjs <json dir> <out dir> [--title "Bundle size"] [--app "name"]');
  process.exit(2);
}

const summary = JSON.parse(readFileSync(join(jsonDir, "summary.json"), "utf8"));
const title = flag("--title") ?? "Bundle size";
const app = flag("--app");
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
const pct = (n, of) => (of > 0 ? (n / of) * 100 : 0);

const t = summary.totals ?? {};
const total = t.bytes ?? 0;
const code = t.codeBytes ?? 0;
const maps = t.sourceMapBytes ?? 0;
const ext = t.externalPackageBytes ?? 0;
const perPush = code + maps;
const insights = summary.insights ?? [];
const dupInsights = insights.filter((i) => i.kind === "duplicate-package");
const dupNames = [...new Set(dupInsights.map((i) => i.subject))];
const crates = cratesOf(summary);
const components = summary.components ?? [];
const componentNames = new Set(components.map((c) => c.component));
const generated = new Date().toISOString().slice(0, 16).replace("T", " ") + " UTC";

// A step as a reader knows it: a package by its name, a file by its path.
function short(display) {
  const d = display.replace(/\/dist\/.*$/, "");
  const nm = d.lastIndexOf("node_modules/");
  return nm >= 0 ? d.slice(nm + "node_modules/".length) : d;
}

// Import chains for each duplicated package (and the two biggest npm ones
// when nothing is duplicated), drawn with Pele.
async function chainDiagrams() {
  const files = existsSync(jsonDir) ? readdirSync(jsonDir).filter((f) => /^pkg-.+\.json$/.test(f)) : [];
  if (!files.length) return [];
  let render;
  try {
    ({ render } = await import(pathToFileURL(join(PELE, "node_modules/pele/dist/pele.js")).href));
  } catch {
    console.error(`Pele isn't at ${PELE}; skipping the import chains.`);
    return [];
  }
  const out = [];
  for (const f of files) {
    const pkg = JSON.parse(readFileSync(join(jsonDir, f), "utf8"));
    const ids = new Map();
    const id = (label) => {
      if (!ids.has(label)) ids.set(label, `n${ids.size}`);
      return ids.get(label);
    };
    const edges = new Set();
    const builds = new Set();
    for (const copy of pkg.copies ?? []) {
      builds.add(copy.build);
      for (const chain of (copy.chains ?? []).slice(0, 5)) {
        const steps = chain.steps ?? [];
        for (let i = 1; i < steps.length; i++) {
          const from = short(steps[i - 1].display);
          const to = i === steps.length - 1 ? copy.target : short(steps[i].display);
          if (from !== to) edges.add(`${id(from)}["${from.replace(/"/g, "'")}"] --> ${id(to)}["${to.replace(/"/g, "'")}"]`);
        }
      }
    }
    if (!edges.size) continue;
    const source = `flowchart LR\n${[...edges].slice(0, 24).map((e) => `  ${e}`).join("\n")}\n`;
    try {
      const r = render(source);
      out.push({ name: pkg.name, builds: [...builds], copies: (pkg.copies ?? []).length, svg: r.svg, source });
    } catch (e) {
      console.error(`Pele couldn't draw ${pkg.name}: ${e.message}`);
    }
  }
  return out;
}

const chains = await chainDiagrams();

const level = (i) => (i.level === "warn" ? "warn" : "info");
const insightRows = insights
  .slice(0, 6)
  .map((i) => `<li class="ins ${level(i)}"><span class="dot"></span><span><b>${esc(i.title.split(", adding")[0])}</b>${i.savedBytes ? `<em>saves up to ${mb(i.savedBytes)}</em>` : ""}</span></li>`)
  .join("");
const more = insights.length > 6 ? `<p class="more">${insights.length - 6} more. Run <code>npx @convex-dev/bundle-analyzer insights</code> for all of them.</p>` : "";

const biggest = Math.max(1, ...(summary.topPackages ?? []).map((p) => p.bytes));
const pkgRows = (summary.topPackages ?? [])
  .slice()
  .sort((a, b) => b.bytes - a.bytes)
  .slice(0, 8)
  .map((p) => {
    const dup = dupNames.includes(p.name);
    const kind = p.name === "your code" ? "own" : dup ? "dup" : componentNames.has(p.name) || p.origin === "convex" ? "cvx" : "npm";
    return `<tr><td class="pn">${esc(p.version ? `${p.name}@${p.version}` : p.name)}${dup ? '<span class="tag">twice</span>' : ""}</td><td class="bar"><i class="${kind}" style="width:${pct(p.bytes, biggest).toFixed(1)}%"></i></td><td class="num">${mb(p.bytes)}</td><td class="num dim">${pct(p.bytes, total).toFixed(1)}%</td></tr>`;
  })
  .join("");

const modRows = (summary.heaviestModules ?? [])
  .slice(0, 6)
  .map((m) => `<tr><td class="pn"><code>${esc(m.path)}</code></td><td class="dim">${esc(m.buildId)}</td><td class="num">${mb(m.loadBytes)}</td><td class="num dim">${mb(m.ownBytes)} own</td></tr>`)
  .join("");

const chainBlocks = chains
  .map((c) => `<details class="chain"><summary><b>${esc(c.name)}</b> <span class="dim">${c.copies} ${c.copies === 1 ? "copy" : "copies"} in ${esc(c.builds.join(", "))}</span></summary><div class="diagram">${c.svg}</div></details>`)
  .join("");

const fig = scaleSvg(summary);

const css = `
:root{color-scheme:dark light;--bg:#0b0d10;--card:#12161b;--line:#20262e;--text:#e6e8eb;--dim:#7d8590;--teal:#4fb3d9;--slate:#5f7a94;--amber:#e8a33d;--amberline:#3a2e1a;--own:#e6e8eb;
--pele-bg:transparent;--pele-fg:#e6e8eb;--pele-muted:#7d8590;--pele-line:#5f7a94;--pele-surface:#161b22;--pele-surface-alt:#1c222a;--pele-border:#2c343e;--pele-accent:#4fb3d9;--pele-font:-apple-system,BlinkMacSystemFont,"SF Pro Text",sans-serif;--pele-radius:6px}
@media (prefers-color-scheme: light){:root{--bg:#f5f3ee;--card:#fbfaf7;--line:#e2ded5;--text:#1d1f22;--dim:#7a766e;--teal:#3d86a3;--slate:#97a8b8;--amber:#d39634;--amberline:#ecd6ad;--own:#1d1f22;
--pele-fg:#1d1f22;--pele-muted:#7a766e;--pele-line:#97a8b8;--pele-surface:#ffffff;--pele-surface-alt:#f5f3ee;--pele-border:#e2ded5;--pele-accent:#3d86a3}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--text);font:13px/1.45 -apple-system,BlinkMacSystemFont,"SF Pro Text",system-ui,sans-serif;-webkit-font-smoothing:antialiased}
main{max-width:1080px;margin:0 auto;padding:22px}
header{display:flex;justify-content:space-between;align-items:baseline;gap:12px;flex-wrap:wrap;margin-bottom:14px}
h1{font-size:19px;margin:0;letter-spacing:-.01em}
h2{font-size:13px;margin:0 0 10px}
.dim{color:var(--dim)}
code{font:12px ui-monospace,"SF Mono",Menlo,monospace}
.card{background:var(--card);border:1px solid var(--line);border-radius:12px;padding:14px}
.hero{display:grid;grid-template-columns:1.55fr 1fr;gap:12px}
.figure{padding:10px 12px 12px;background:radial-gradient(120% 90% at 45% 35%,color-mix(in srgb,var(--card) 70%,var(--text) 4%) 0%,var(--card) 72%)}
.figure .iso{background:transparent}
.fighead,.figfoot{display:flex;justify-content:space-between;color:var(--dim);font-size:11px}
.figfoot{border-top:1px solid var(--line);padding-top:8px;margin-top:4px}
.figfoot b{color:var(--text);font-weight:600}
.tiles{display:grid;gap:10px;align-content:start}
.tile .k{color:var(--dim);font-size:11px}
.tile .v{font-size:22px;font-weight:600;letter-spacing:-.01em;font-variant-numeric:tabular-nums}
.tile.big .v{font-size:32px}
.tile.warn{border-color:var(--amberline)}
.tile.warn .v{color:var(--amber)}
.stack{display:flex;height:7px;border-radius:4px;overflow:hidden;margin-top:8px;background:var(--line)}
.stack i{display:block;height:100%}
.legend{display:flex;flex-wrap:wrap;gap:4px 12px;color:var(--dim);font-size:11px;margin-top:6px}
.legend b{display:inline-block;width:8px;height:8px;border-radius:2px;margin-right:5px;vertical-align:-1px}
.c-code{background:var(--teal)}.c-maps{background:var(--slate)}.c-ext{background:var(--amber)}
.meta{color:var(--dim);font-size:11px;padding:0 2px}
.row{display:grid;grid-template-columns:1fr 1.3fr;gap:12px;margin-top:12px}
ul{list-style:none;margin:0;padding:0;display:grid;gap:8px}
.ins{display:flex;gap:8px;align-items:flex-start;font-size:12px}
.ins b{font-weight:500}
.ins em{display:block;font-style:normal;color:var(--dim);font-size:11px}
.ins .dot{flex:none;width:7px;height:7px;border-radius:50%;margin-top:5px;background:var(--teal)}
.ins.warn .dot{background:var(--amber)}
.empty{color:var(--dim);font-size:12px;margin:0}
.more{color:var(--dim);font-size:11px;margin:10px 0 0}
table{width:100%;border-collapse:collapse;font-size:12px}
td{padding:4px 0;vertical-align:middle}
td+td{padding-left:10px}
.pn{white-space:nowrap;max-width:220px;overflow:hidden;text-overflow:ellipsis}
.mods td:first-child{width:40%}
.num{text-align:right;white-space:nowrap;font-variant-numeric:tabular-nums}
td.bar{width:42%}
td.bar i{display:block;height:6px;border-radius:3px;min-width:2px}
i.own{background:var(--own)}i.cvx{background:var(--teal)}i.npm{background:var(--slate)}i.dup{background:var(--amber)}
.tag{margin-left:6px;font-size:10px;color:var(--amber);border:1px solid var(--amberline);border-radius:4px;padding:0 4px}
.wide{margin-top:12px}
.chain{border-top:1px solid var(--line);padding:8px 0}
.chain:first-of-type{border-top:0}
.chain summary{cursor:pointer;font-size:12px}
.diagram{display:flex;justify-content:center;padding:10px 0;overflow-x:auto}
.diagram svg{max-width:100%;height:auto}
footer{color:var(--dim);font-size:11px;margin-top:14px;display:flex;justify-content:space-between;gap:12px;flex-wrap:wrap}
.br-dup .iso-top{fill:var(--amber)!important}.br-dup .iso-shade{fill:color-mix(in srgb,var(--amber) 70%,#000)!important}.br-dup .iso-fill{fill:var(--amber)!important}
@media (max-width:640px){.hero,.row{grid-template-columns:1fr}main{padding:14px}.tile.big .v{font-size:26px}.pn{max-width:150px}td.bar{width:34%}.pkgs td.num.dim{display:none}.mods td:nth-child(2){display:none}}
`;

const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${esc(title)}${app ? ` · ${esc(app)}` : ""}</title>
<style>${ISO_CSS}${autoThemeCss()}${css}</style>
</head>
<body>
<main>
<header><h1>${esc(title)}${app ? ` <span class="dim">· ${esc(app)}</span>` : ""}</h1><span class="dim">convex ${esc(summary.convexVersion ?? "?")} · ${components.length} ${components.length === 1 ? "build" : "builds"} · ${t.modules ?? "?"} modules</span></header>
<section class="hero">
  <div class="card figure iso">
    <div class="fighead"><span>Fig 1</span><span>Weighing the bundle</span></div>
    ${fig}
    <div class="figfoot"><span>Crate height is package size. Amber tags ship more than once.</span><b>${mb(total)} on a ${capacityFor(total)} MB scale</b></div>
  </div>
  <div class="tiles">
    <div class="card tile big"><div class="k">Total bundle</div><div class="v">${mb(total)}</div>
      <div class="stack"><i class="c-code" style="width:${pct(code, total).toFixed(1)}%"></i><i class="c-maps" style="width:${pct(maps, total).toFixed(1)}%"></i><i class="c-ext" style="width:${pct(ext, total).toFixed(1)}%"></i></div>
      <div class="legend"><span><b class="c-code"></b>code ${mb(code)}</span><span><b class="c-maps"></b>source maps ${mb(maps)}</span>${ext ? `<span><b class="c-ext"></b>external ${mb(ext)}</span>` : ""}</div></div>
    <div class="card tile"><div class="k">Uploaded on every push</div><div class="v">${mb(perPush)}</div><div class="k">code and source maps</div></div>
    <div class="card tile${dupNames.length ? " warn" : ""}"><div class="k">Bundled more than once</div><div class="v">${dupNames.length ? `${dupNames.length} ${dupNames.length === 1 ? "package" : "packages"}` : "None"}</div>${dupNames.length ? `<div class="k">${esc(dupNames.slice(0, 4).join(", "))}${dupNames.length > 4 ? ` and ${dupNames.length - 4} more` : ""}</div>` : ""}</div>
    <div class="meta">${crates.length} largest packages on the scale · ${insights.length} ${insights.length === 1 ? "insight" : "insights"}</div>
  </div>
</section>
<section class="row">
  <div class="card"><h2>Insights</h2>${insightRows ? `<ul>${insightRows}</ul>${more}` : '<p class="empty">Nothing to fix. No package is bundled twice, and nothing "use node" leaks into the Convex runtime.</p>'}</div>
  <div class="card"><h2>Largest packages</h2><table class="pkgs">${pkgRows}</table></div>
</section>
${modRows ? `<section class="card wide"><h2>Heaviest functions to load</h2><table class="mods">${modRows}</table></section>` : ""}
${chainBlocks ? `<section class="card wide"><h2>Why they're bundled</h2>${chainBlocks}</section>` : ""}
<footer><span>From <code>npx @convex-dev/bundle-analyzer summary --json</code>. Sizes come from the analyzer's bundler and can differ a little from a deploy.</span><span>${generated}</span></footer>
</main>
</body>
</html>
`;

if (/<script\b/i.test(html)) throw new Error("The page must stay static: something added a <script>.");
mkdirSync(outDir, { recursive: true });
writeFileSync(join(outDir, "index.html"), html);
console.log(join(outDir, "index.html"));
console.log(`${mb(total)} total, ${crates.length} crates, ${insights.length} insights, ${chains.length} import chain diagrams, ${(html.length / 1024).toFixed(0)} KB`);
