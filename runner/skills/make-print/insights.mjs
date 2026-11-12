// A Convex insights Print: `npx convex insights --details --json` turned
// into a make-print dashboard spec, then built like any other.
//
//   npx convex insights --details --json > insights.json   (add --prod for production)
//   node <this skill's folder>/insights.mjs insights.json <out dir>
//
// Insights cover the last 72 hours: write conflicts (OCC) and functions near
// or over their read limits (https://docs.convex.dev/cli/reference/insights).
// Only the occRetried shape has been seen in real output; the other kinds are
// read by their kind and severity, with their count found as best it can be.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildPage } from "./build.mjs";

const KINDS = {
  occRetried: "Conflict, retried",
  occFailedPermanently: "Conflict, failed",
  bytesReadLimit: "Over byte limit",
  bytesReadThreshold: "Near byte limit",
  documentsReadLimit: "Over doc limit",
  documentsReadThreshold: "Near doc limit",
};

export const kindLabel = (kind) => KINDS[kind] ?? String(kind ?? "Insight");

// How many calls an insight counts: occCalls for a write conflict, else the
// first number whose name ends in "calls" or "count", else its events' calls.
export function callsOf(insight) {
  if (typeof insight.occCalls === "number") return insight.occCalls;
  for (const [k, v] of Object.entries(insight)) if (typeof v === "number" && /(calls|count)$/i.test(k)) return v;
  const events = Array.isArray(insight.recentEvents) ? insight.recentEvents : [];
  const sum = events.reduce((n, e) => n + (typeof e.calls === "number" ? e.calls : 0), 0);
  return sum || 1;
}

const lastSeen = (insight) => {
  const times = (insight.recentEvents ?? []).map((e) => Date.parse(e.timestamp)).filter(Number.isFinite);
  if (!times.length) return "";
  return new Date(Math.max(...times)).toISOString().slice(0, 16).replace("T", " ") + " UTC";
};

// The insights array, or an error saying why there isn't one: a missing or
// broken result must never read as "all clear".
export function insightsList(json) {
  if (!json || typeof json !== "object" || !Array.isArray(json.insights)) {
    const why = json && typeof json === "object" && json.error ? `: ${json.error}` : "";
    throw new Error(`This isn't npx convex insights --json output (no "insights" list)${why}. Run it again and check it worked before making the print.`);
  }
  return json.insights;
}

// A component's function says which component in brackets after it, so
// root "agent/messages.js:list" and component agent's "messages.js:list"
// stay two functions. Function ids never contain " (".
export const functionName = (i) => {
  const id = String(i.functionId ?? "unknown");
  return i.componentPath ? `${id} (${i.componentPath})` : id;
};

// Errors first, then the most calls, so the table leads with what's failing.
const worstFirst = (a, b) => (a.Severity === "Error" ? 0 : 1) - (b.Severity === "Error" ? 0 : 1) || b.Calls - a.Calls;

export function insightsRows(json) {
  return insightsList(json).map((i) => ({
    Function: functionName(i),
    What: kindLabel(i.kind),
    Severity: i.severity === "error" ? "Error" : "Warning",
    Table: i.occTableName || "",
    Calls: callsOf(i),
    "Last seen": lastSeen(i),
  })).sort(worstFirst);
}

export function insightsSpec(json, { title } = {}) {
  const rows = insightsRows(json);
  const deployment = json.deploymentName ?? "this deployment";
  const footer = `From npx convex insights on ${deployment}, covering the last 72 hours.${json.dashboardUrl ? ` Dashboard: ${json.dashboardUrl}` : ""}`;
  const heading = title ?? `Convex insights: ${deployment}`;
  if (!rows.length) {
    return {
      kind: "report",
      title: heading,
      subtitle: "Last 72 hours",
      tiles: [{ label: "Insights", value: "0" }, { label: "Status", value: "All clear", tone: "good" }],
      sections: [{ heading: "Nothing to fix", markdown: "No write conflicts and no functions near their read limits in the last 72 hours." }],
      footer,
    };
  }
  const errors = rows.filter((r) => r.Severity === "Error").length;
  const functions = new Set(rows.map((r) => r.Function)).size;
  return {
    kind: "dashboard",
    title: heading,
    subtitle: "Last 72 hours",
    data: rows,
    tiles: [
      { label: "Insights", agg: "count" },
      { label: "Calls affected", value: "Calls", agg: "sum" },
      { label: "Errors", value: String(errors), tone: errors ? "bad" : "good" },
      { label: "Functions", value: String(functions) },
    ],
    charts: [
      { type: "bar", title: "Calls by function", by: "Function", value: "Calls", top: 8 },
      { type: "bar", title: "Calls by kind", by: "What", value: "Calls" },
    ],
    // Every insight, errors first: none is cut off by a limit.
    table: { title: "Each insight, errors first", columns: ["Function", "What", "Table", "Calls", "Severity", "Last seen"], limit: rows.length },
    footer,
  };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const [input, outDir] = process.argv.slice(2);
  if (!input || !outDir) {
    console.error("Usage: node insights.mjs <insights.json> <out dir>");
    process.exit(2);
  }
  const json = JSON.parse(readFileSync(input, "utf8"));
  const spec = insightsSpec(json);
  const { html } = buildPage(spec, outDir);
  mkdirSync(outDir, { recursive: true });
  writeFileSync(join(outDir, "spec.json"), JSON.stringify(spec, null, 2));
  writeFileSync(join(outDir, "index.html"), html);
  console.log(`${join(outDir, "index.html")}  ${spec.kind}  ${insightsRows(json).length} insights`);
}
