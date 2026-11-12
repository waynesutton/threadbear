#!/usr/bin/env node
// Builds a chart, dashboard or report Print from a short JSON spec.
//
//   node build.mjs <spec.json> <out dir>
//
// Writes <out dir>/index.html: static HTML, CSS and SVG, no script, in the
// Prints look, light or dark by the viewer's appearance. Paths in the spec
// are relative to the spec file. SKILL.md has the spec.
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { bar, column, fmt, line, pie, segmented, table, tiles } from "./charts.mjs";
import { group, numbers, readData, total } from "./data.mjs";
import { markdown } from "./md.mjs";
import { BASE_CSS, themeCss } from "./theme.mjs";
import { PROGRESS_CSS, progressBody } from "./progress-view.mjs";
import { fileURLToPath } from "node:url";

// The user's style, asked once (SKILL.md). Kept outside this folder, which
// Threadbear replaces when the skill updates; MAKE_PRINT_STYLE moves it.
export const STYLE_FILE = process.env.MAKE_PRINT_STYLE || join(process.env.XDG_CONFIG_HOME || join(homedir(), ".config"), "threadbear", "print-style.json");
// Where it was saved before make-print was built in, still read.
const OLD_STYLE_FILES = [join(dirname(fileURLToPath(import.meta.url)), "style.json"), join(homedir(), ".claude", "skills", "make-print", "style.json")];
export function savedStyle() {
  for (const file of [STYLE_FILE, ...(process.env.MAKE_PRINT_STYLE ? [] : OLD_STYLE_FILES)]) {
    try {
      if (existsSync(file)) return JSON.parse(readFileSync(file, "utf8"));
    } catch {
      return {};
    }
  }
  return {};
}

const esc = (s) => String(s ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

export function buildPage(spec, base = process.cwd()) {
  const at = (p) => resolve(base, p);
  const sources = new Map();
  const load = (d) => {
    if (!d) return null;
    if (Array.isArray(d)) return { columns: [...new Set(d.flatMap((r) => Object.keys(r)))], rows: d };
    const file = typeof d === "string" ? d : d.file;
    const key = `${file}::${d.sheet ?? ""}`;
    if (!sources.has(key)) sources.set(key, readData(at(file), { sheet: d.sheet }));
    return sources.get(key);
  };
  const main = load(spec.data);
  const need = (cols, data, what) => {
    for (const c of cols.filter(Boolean)) {
      if (!data) throw new Error(`${what} needs data: give the spec a "data" file.`);
      if (!data.columns.includes(c)) throw new Error(`${what}: no column "${c}". Columns: ${data.columns.join(", ")}`);
    }
  };

  const chartHtml = (c) => {
    const data = c.data ? load(c.data) : main;
    const what = `Chart "${c.title ?? c.type}"`;
    const agg = c.agg ?? (c.value ? "sum" : "count");
    const opts = { title: c.title, note: c.note ?? (c.value ? `${agg === "sum" ? "total" : agg} of ${c.value}` : agg === "count" ? "rows" : undefined), unit: c.unit ?? "" };
    if (c.items) return pick(c.type, c.items, opts, c);
    need([c.by, c.value, c.series], data, what);
    if (c.type === "line") {
      if (c.series) {
        const names = group(data.rows, c.series, c.value, agg).sort((a, b) => b.value - a.value).map((x) => x.label);
        const series = names.slice(0, 6).map((name) => ({ name, points: group(data.rows.filter((r) => String(r[c.series]) === name), c.by, c.value, agg).map((p) => ({ x: p.label, y: p.value })) }));
        return line(series, opts);
      }
      return line([{ name: c.value ?? "count", points: group(data.rows, c.by, c.value, agg).map((p) => ({ x: p.label, y: p.value })) }], opts);
    }
    return pick(c.type, group(data.rows, c.by, c.value, agg), opts, c);
  };
  const pick = (type, items, opts, c) => {
    if (type === "pie" || type === "donut") return pie(items, opts);
    if (type === "segmented") return segmented(items, opts);
    if (type === "column") return column(c.sort === "value" ? [...items].sort((a, b) => b.value - a.value) : items, opts);
    return bar(items, { ...opts, top: c.top ?? 12 });
  };

  const tileHtml = (list) =>
    tiles(
      list.map((t) => {
        if (t.value !== undefined && (!main || !main.columns.includes(t.value)) && !t.agg) return { label: t.label, value: String(t.value), note: t.note, tone: t.tone };
        const agg = t.agg ?? "sum";
        if (agg !== "count") need([t.value], main, `Tile "${t.label}"`);
        const v = agg === "count" ? main.rows.length : total(numbers(main.rows, t.value), agg);
        return { label: t.label, value: fmt(v, t.unit ?? ""), note: t.note, tone: t.tone };
      }),
    );

  const tableHtml = (tb) => {
    if (!tb || !main) return "";
    const cols = tb === true ? main.columns : (tb.columns ?? main.columns);
    need(cols, main, "Table");
    const rows = tb.sort ? [...main.rows].sort((a, b) => (b[tb.sort] ?? 0) - (a[tb.sort] ?? 0)) : main.rows;
    return `<section class="card">${table(cols, rows, { title: tb.title ?? "Rows", limit: tb.limit ?? 50, units: tb.units ?? {} })}</section>`;
  };

  const figure = (f) => (f ? `<section class="card figure">${readFileSync(at(f), "utf8").replace(/<\?xml[^>]*>/, "").replace(/<script[\s\S]*?<\/script>/gi, "")}</section>` : "");

  const kind = spec.kind ?? (spec.sections ? "report" : (spec.charts?.length ?? 0) > 1 || spec.tiles ? "dashboard" : "chart");
  let body;
  if (kind === "progress") {
    body = progressBody(spec);
  } else if (kind === "report") {
    body = [
      spec.tiles ? tileHtml(spec.tiles) : "",
      figure(spec.figure),
      ...(spec.sections ?? []).map((s) => `<section class="card stack">${s.heading ? `<h2 style="margin-top:0">${esc(s.heading)}</h2>` : ""}${s.markdown ? `<div class="prose">${markdown(s.markdown)}</div>` : ""}${s.chart ? chartHtml(s.chart) : ""}${s.figure ? figure(s.figure) : ""}</section>`),
      tableHtml(spec.table),
    ].join("\n");
  } else {
    const charts = (spec.charts ?? []).map((c) => `<section class="card">${chartHtml(c)}</section>`);
    body = [spec.tiles ? tileHtml(spec.tiles) : "", figure(spec.figure), charts.length > 1 ? `<div class="grid2">${charts.join("")}</div>` : charts.join(""), tableHtml(spec.table)].join("\n");
  }

  const rowsNote = main ? `${main.rows.length.toLocaleString("en-US")} rows` : "";
  const stamp = new Date().toISOString().slice(0, 16).replace("T", " ") + " UTC";
  const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${esc(spec.title)}</title>
<style>${themeCss(spec.style ?? savedStyle())}${BASE_CSS}${kind === "progress" ? PROGRESS_CSS : ""}
section+section,.tiles+section,.grid2+section,section+.grid2,.tiles+.grid2{margin-top:12px}
.grid2>section{margin-top:0!important}</style>
</head>
<body>
<main class="${kind === "report" ? "narrow" : ""}">
<header><h1>${esc(spec.title)}</h1><span class="sub">${esc(spec.subtitle ?? (kind === "progress" ? `progress · ${new Date().toLocaleString("en-US", { hour: "numeric", minute: "2-digit", month: "short", day: "numeric" })}` : [kind, rowsNote].filter(Boolean).join(" · ")))}</span></header>
${body}
<footer><span>${esc(spec.footer ?? (typeof spec.data === "string" ? `From ${spec.data.split("/").pop()}` : typeof spec.data?.file === "string" ? `From ${spec.data.file.split("/").pop()}${spec.data.sheet ? `, sheet ${spec.data.sheet}` : ""}` : ""))}</span><span>${stamp}</span></footer>
</main>
</body>
</html>
`;
  if (/<script\b/i.test(html)) throw new Error("The page must stay static: something added a <script>.");
  return { html, kind };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const [specPath, outDir] = process.argv.slice(2);
  if (!specPath || !outDir) {
    console.error("usage: node build.mjs <spec.json> <out dir>");
    process.exit(2);
  }
  try {
    const spec = JSON.parse(readFileSync(specPath, "utf8"));
    const { html, kind } = buildPage(spec, dirname(resolve(specPath)));
    mkdirSync(outDir, { recursive: true });
    writeFileSync(join(outDir, "index.html"), html);
    console.log(`${kind}: ${join(resolve(outDir), "index.html")} (${(html.length / 1024).toFixed(0)} KB)`);
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
}
