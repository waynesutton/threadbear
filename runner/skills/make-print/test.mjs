// node --test test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import { buildPage } from "./build.mjs";
import { fmt, fold } from "./charts.mjs";
import { group, parseDelimited, readData } from "./data.mjs";
import { markdown } from "./md.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const FX = join(HERE, "fixtures");
const spec = (name) => JSON.parse(readFileSync(join(FX, name), "utf8"));

test("CSV: quotes, doubled quotes, newlines in quotes, numbers with $ , and %", () => {
  const { columns, rows } = parseDelimited('Name,Amount,Note\n"Acme, Inc","$1,234.5","said ""hi""\nthen left"\nBeta,45%,\n');
  assert.deepEqual(columns, ["Name", "Amount", "Note"]);
  assert.equal(rows[0].Name, "Acme, Inc");
  assert.equal(rows[0].Amount, 1234.5);
  assert.equal(rows[0].Note, 'said "hi"\nthen left');
  assert.equal(rows[1].Amount, 45);
  assert.equal(rows[1].Note, null);
  const quoted = parseDelimited('a,b\n"1,234",x\n');
  assert.equal(quoted.rows[0].a, 1234);
});

test("XLSX and CSV read the same rows; a missing sheet names the ones there are", () => {
  const csv = readData(join(FX, "sales.csv"));
  const xlsx = readData(join(FX, "sales.xlsx"), { sheet: "Sales 2026" });
  assert.deepEqual(xlsx.columns, csv.columns);
  assert.equal(xlsx.rows.length, 180);
  assert.deepEqual(xlsx.rows[5], csv.rows[5]);
  assert.throws(() => readData(join(FX, "sales.xlsx"), { sheet: "Nope" }), /Sheets: Sales 2026/);
});

test("group and fold: totals by column, at most six slots then Other", () => {
  const { rows } = readData(join(FX, "sales.csv"));
  const byRegion = group(rows, "Region", "Revenue");
  assert.equal(byRegion.length, 6);
  assert.equal(group(rows, "Product", null, "count").find((x) => x.label === "Pro").value, 60);
  const folded = fold([1, 2, 3, 4, 5, 6, 7, 8].map((v) => ({ label: `x${v}`, value: v })));
  assert.equal(folded.length, 6);
  assert.equal(folded[5].label, "Other (3)");
  assert.equal(folded[5].value, 1 + 2 + 3);
});

test("numbers read the way people say them", () => {
  assert.equal(fmt(129000, "$"), "$129K");
  assert.equal(fmt(7435.12, "$"), "$7,435");
  assert.equal(fmt(5183), "5,183");
  assert.equal(fmt(0.5, "%"), "0.5%");
  assert.equal(fmt(2_500_000), "2.5M");
});

test("every sample builds a static page with the numbers in it", () => {
  for (const name of ["pie.json", "dashboard.json", "report.json"]) {
    const { html, kind } = buildPage(spec(name), FX);
    assert.doesNotMatch(html, /<script\b/i, name);
    assert.match(html, /prefers-color-scheme: light/, name);
    assert.match(html, /Show the data/, name);
    assert.ok(["chart", "dashboard", "report"].includes(kind), name);
  }
  const dash = buildPage(spec("dashboard.json"), FX).html;
  assert.match(dash, /\$129K/);
  assert.match(dash, /class="ln s1"/);
  assert.match(buildPage(spec("pie.json"), FX).html, /35\.8%/);
});

test("a wrong column says which columns there are", () => {
  assert.throws(() => buildPage({ title: "x", data: "sales.csv", charts: [{ type: "bar", by: "Country", value: "Revenue" }] }, FX), /no column "Country"\. Columns: Month, Region/);
});

test("report markdown is escaped and links keep only safe addresses", () => {
  const html = markdown("<img src=x onerror=alert(1)> [ok](https://x.dev) [no](javascript:alert(1))");
  assert.doesNotMatch(html, /<img/);
  assert.match(html, /<a href="https:\/\/x.dev">ok<\/a>/);
  assert.doesNotMatch(html, /javascript/);
});

test("inline rows work without a file", () => {
  const { html } = buildPage({ title: "Inline", charts: [{ type: "bar", items: [{ label: "A", value: 3 }, { label: "B", value: 1 }] }] }, FX);
  assert.match(html, />A</);
});

test("progress: steps, one doing at a time, questions with defaults, stuck, deliverables", async () => {
  const { apply } = await import("./progress.mjs");
  const t = (m) => `2026-11-10T10:${String(m).padStart(2, "0")}:00.000Z`;
  let s = apply(null, "init", ["Ship it", "Ship the thing"], t(0));
  s = apply(s, "step", ["Build", "doing"], t(1));
  s = apply(s, "step", ["Test", "doing"], t(2));
  assert.equal(s.steps.find((x) => x.title === "Build").status, "todo", "only one step is doing");
  s = apply(s, "step", ["Build", "done"], t(3));
  s = apply(s, "ask", ["Publish now?", "Wait for you"], t(4));
  s = apply(s, "answer", ["publish", "Yes"], t(5));
  assert.equal(s.questions[0].answer, "Yes");
  s = apply(s, "stuck", ["Deploy", "no key"], t(6));
  s = apply(s, "unstuck", ["dep"], t(7));
  assert.equal(s.stuck.length, 0);
  s = apply(s, "deliver", ["PR", "https://x.dev/pr/1"], t(8));
  assert.equal(s.updatedAt, t(8));
  assert.throws(() => apply(s, "step", ["X", "nope"]), /status is one of/);
  assert.throws(() => apply(s, "ask", ["No default"]), /the default/);
});

test("progress view: panels only when they have something, ages from the real clock, statuses never colour alone", async () => {
  const { progressBody, ago } = await import("./progress-view.mjs");
  const now = Date.parse("2026-11-10T12:00:00Z");
  assert.equal(ago("2026-11-10T11:58:00Z", now), "2m ago");
  assert.equal(ago("2026-11-10T09:30:00Z", now), "2h 30m ago");
  const s = { task: "T", startedAt: "2026-11-10T11:00:00Z", updatedAt: "2026-11-10T11:59:00Z", steps: [{ title: "A", status: "done", at: "2026-11-10T11:10:00Z" }, { title: "B", status: "doing", at: "2026-11-10T11:50:00Z" }], questions: [], stuck: [], deliverables: [], notes: [] };
  const html = progressBody(s, now);
  assert.match(html, /1 of 2 steps done · 50%/);
  assert.match(html, /<b>1h 0m<\/b>|<b>60m<\/b>/);
  assert.doesNotMatch(html, /Waiting on you<\/h2>/, "no questions, no panel");
  assert.doesNotMatch(html, /Stuck<\/h2>/);
  assert.match(html, /✓<\/i>Done/);
  const asked = progressBody({ ...s, questions: [{ question: "Q?", default: "D", asked: "2026-11-10T11:55:00Z" }] }, now);
  assert.match(asked, /If no answer<\/span>D/);
});

test("a saved style changes the theme, density and accent; a bad accent is ignored", async () => {
  const { themeCss } = await import("./theme.mjs");
  const dark = themeCss({ theme: "dark", density: "airy", accent: "#ff8800" });
  assert.doesNotMatch(dark, /prefers-color-scheme/);
  assert.match(dark, /--pad:20px/);
  assert.match(dark, /--accent:#ff8800/);
  assert.match(themeCss({}), /prefers-color-scheme: light/);
  assert.match(themeCss({ accent: "red;}body{x" }), /--accent:var\(--teal\)/);
  const { html } = buildPage({ kind: "progress", title: "P", task: "t", startedAt: new Date().toISOString(), updatedAt: new Date().toISOString(), steps: [], questions: [], stuck: [], deliverables: [], notes: [], style: { theme: "light" } }, FX);
  assert.match(html, /color-scheme:light/);
  assert.doesNotMatch(html, /<script/);
});

test("Convex insights: rows, a dashboard, and an all-clear report when there are none", async () => {
  const { insightsRows, insightsSpec, kindLabel, callsOf } = await import("./insights.mjs");
  const json = spec("insights.json");
  const rows = insightsRows(json);
  assert.equal(rows.length, 4);
  assert.deepEqual(rows.find((r) => r.Function === "sync.js:setPresence"), { Function: "sync.js:setPresence", What: "Conflict, retried", Severity: "Warning", Table: "runners", Calls: 2, "Last seen": "2026-11-11 03:28 UTC" });
  assert.equal(rows.length, 4);
  const err = rows[0];
  assert.equal(err.Function, "messages.js:list (agent)", "a component's function says which component");
  assert.equal(err.Severity, "Error", "errors come first");
  assert.equal(err.Calls, 4, "no occCalls: the events' calls");
  assert.equal(kindLabel("somethingNew"), "somethingNew", "an unknown kind shows as itself");
  assert.equal(callsOf({ kind: "x", recentEvents: [] }), 1);
  const s = insightsSpec(json);
  assert.equal(s.kind, "dashboard");
  const { html } = buildPage(s, FX);
  assert.match(html, /Convex insights: demo-otter-123/);
  assert.match(html, /Conflict, retried/);
  assert.match(html, /Each insight/);
  assert.match(html, /messages\.js:list \(agent\)/);
  assert.match(html, /last 72 hours/);
  assert.doesNotMatch(html, /<script/);
  // Codex's review: a rare error isn't cut off by many busier warnings.
  const busy = { deploymentName: "d", insights: [...Array.from({ length: 30 }, (_, n) => ({ kind: "occRetried", severity: "warning", functionId: `w${n}.js:f`, occCalls: 10 })), { kind: "documentsReadLimit", severity: "error", functionId: "rare.js:fails", recentEvents: [{ calls: 1 }] }] };
  const busyHtml = buildPage(insightsSpec(busy), FX).html;
  assert.match(busyHtml, /rare\.js:fails/);
  assert.match(busyHtml, /w29\.js:f/, "every insight is in the table");
  // A root function and a component's with the same path stay two.
  const twins = insightsSpec({ insights: [{ kind: "occRetried", severity: "warning", functionId: "agent/messages.js:list", occCalls: 1 }, { kind: "occRetried", severity: "warning", functionId: "messages.js:list", componentPath: "agent", occCalls: 1 }] });
  assert.equal(twins.tiles.find((t) => t.label === "Functions").value, "2");
  // Missing or broken output is an error, never "All clear".
  assert.throws(() => insightsSpec({ deploymentName: "demo", error: "Failed to load insights" }), /no "insights" list\): Failed to load insights/);
  assert.throws(() => insightsSpec({ deploymentName: "demo", insights: null }), /no "insights" list/);
  assert.throws(() => insightsSpec(null), /no "insights" list/);
  const empty = insightsSpec({ deploymentName: "calm-cat-1", insights: [] });
  assert.equal(empty.kind, "report");
  assert.match(buildPage(empty, FX).html, /No write conflicts/);
});

test("the saved style lives outside the skill's folder, so an update keeps it", async () => {
  const { mkdtempSync, writeFileSync } = await import("node:fs");
  const { tmpdir } = await import("node:os");
  const { execFileSync } = await import("node:child_process");
  const dir = mkdtempSync(join(tmpdir(), "make-print-style-"));
  const file = join(dir, "print-style.json");
  writeFileSync(file, JSON.stringify({ theme: "light", accent: "#336699" }));
  const read = (env) => execFileSync(process.execPath, ["-e", `import("${join(HERE, "build.mjs")}").then((m) => console.log(m.STYLE_FILE + "|" + JSON.stringify(m.savedStyle())))`], { env: { ...process.env, ...env } }).toString().trim();
  assert.equal(read({ MAKE_PRINT_STYLE: file }), `${file}|{"theme":"light","accent":"#336699"}`);
  assert.equal(read({ MAKE_PRINT_STYLE: "", XDG_CONFIG_HOME: dir, HOME: dir }).split("|")[0], join(dir, "threadbear", "print-style.json"));
  assert.ok(!read({ MAKE_PRINT_STYLE: "", XDG_CONFIG_HOME: dir, HOME: dir }).split("|")[0].startsWith(HERE), "not in the skill's folder");
});

test("Convex API map: every function, gaps first, and a failed command stops", async () => {
  const { apiRows, apiSpec, argNames } = await import("./apimap.mjs");
  const json = spec("functions.json");
  const rows = apiRows(json);
  assert.equal(rows.length, json.functions.length, "every function, routes included");
  assert.deepEqual(rows[0], { Function: "legacy.js:save", File: "legacy.js", Type: "Mutation", Access: "Public", Takes: "any", Checks: "No args or returns validator" }, "a public function with no validators comes first");
  const messages = rows.find((r) => r.Function === "phone.js:messages");
  assert.match(messages.Takes, /threadId/);
  assert.match(messages.Takes, /limit\?/, "optional args are marked");
  assert.equal(messages.Checks, "Validated");
  assert.ok(rows.some((r) => r.Type === "HTTP" && r.Access === "Public route" && /^GET \//.test(r.Function)));
  assert.equal(rows.at(-1).Access, "Internal", "internal ones come last");
  assert.equal(argNames({ type: "object", value: {} }), "none");
  const s = apiSpec(json);
  assert.equal(s.tiles.find((t) => t.label === "Missing validators").value, "1");
  const { html } = buildPage(s, FX);
  assert.match(html, /Convex API: demo-otter-123/);
  assert.match(html, /legacy\.js:save/);
  assert.match(html, /Every function, gaps first/);
  assert.doesNotMatch(html, /<script/);
  assert.throws(() => apiSpec({ error: "Not logged in" }), /no "functions" list\): Not logged in/);
  assert.throws(() => apiSpec(null), /no "functions" list/);
  assert.equal(apiSpec({ url: "https://x.convex.cloud", functions: [] }).kind, "report");
});
