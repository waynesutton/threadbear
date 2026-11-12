// A Convex API map Print: `npx convex function-spec` turned into a
// make-print dashboard of every function, HTTP route included, with what
// each takes and whether it's public.
//
//   npx convex function-spec > functions.json        (add --prod for production)
//   node <this skill's folder>/apimap.mjs functions.json <out dir>
//
// The spec's shape (seen 12 Nov 2026, convex 1.49): { url, functions: [...] },
// each { identifier: "file.js:name", functionType: "Query" | "Mutation" |
// "Action", visibility: { kind: "public" | "internal" }, args, returns }, or
// { functionType: "HttpAction", method, path } for a route. A function
// without a validator is taken to read as { type: "any" }; that hasn't been
// seen in real output yet.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildPage } from "./build.mjs";

const TYPES = { Query: "Query", Mutation: "Mutation", Action: "Action", HttpAction: "HTTP" };

// The functions list, or an error saying why there isn't one: a failed
// command must never draw an empty, healthy-looking map.
export function functionList(json) {
  if (!json || typeof json !== "object" || !Array.isArray(json.functions)) {
    const why = json && typeof json === "object" && json.error ? `: ${json.error}` : "";
    throw new Error(`This isn't npx convex function-spec output (no "functions" list)${why}. Run it again and check it worked before making the print.`);
  }
  return json.functions;
}

// "threadId, limit?" from an object validator; "any" when it takes anything.
export function argNames(args) {
  if (!args) return "";
  if (args.type !== "object" || !args.value || typeof args.value !== "object") return args.type === "any" ? "any" : String(args.type ?? "");
  const names = Object.entries(args.value).map(([name, v]) => (v?.optional ? `${name}?` : name));
  return names.length ? names.join(", ") : "none";
}

const unchecked = (v) => !v || v.type === "any";

export function apiRows(json) {
  const rows = functionList(json).map((f) => {
    const http = f.functionType === "HttpAction";
    const id = http ? `${f.method ?? "ANY"} ${f.path ?? "?"}` : String(f.identifier ?? "unknown");
    const checks = http ? [] : [unchecked(f.args) && "args", unchecked(f.returns) && "returns"].filter(Boolean);
    return {
      Function: f.componentPath ? `${id} (${f.componentPath})` : id,
      File: http ? "http" : id.split(":")[0],
      Type: TYPES[f.functionType] ?? String(f.functionType ?? "?"),
      Access: http ? "Public route" : f.visibility?.kind === "internal" ? "Internal" : "Public",
      Takes: http ? "" : argNames(f.args),
      Checks: http ? "" : checks.length ? `No ${checks.join(" or ")} validator` : "Validated",
    };
  });
  // What to look at first: public functions missing a validator, then the
  // rest of the public surface, then internal ones, each by name.
  const rank = (r) => (r.Checks.startsWith("No ") && r.Access === "Public" ? 0 : r.Checks.startsWith("No ") ? 1 : r.Access === "Internal" ? 3 : 2);
  return rows.sort((a, b) => rank(a) - rank(b) || a.Function.localeCompare(b.Function));
}

export function apiSpec(json, { title } = {}) {
  const rows = apiRows(json);
  const where = typeof json.url === "string" ? json.url.replace(/^https?:\/\//, "").replace(/\.convex\.(cloud|site).*$/, "") : "this deployment";
  const footer = `From npx convex function-spec on ${where}.`;
  const heading = title ?? `Convex API: ${where}`;
  if (!rows.length) {
    return {
      kind: "report",
      title: heading,
      tiles: [{ label: "Functions", value: "0" }],
      sections: [{ heading: "No functions", markdown: "This deployment has no functions pushed yet. Run `npx convex dev` or deploy, then make the map again." }],
      footer,
    };
  }
  const count = (pred) => String(rows.filter(pred).length);
  const unvalidated = rows.filter((r) => r.Checks.startsWith("No ")).length;
  return {
    kind: "dashboard",
    title: heading,
    subtitle: `${rows.length} functions`,
    data: rows,
    tiles: [
      { label: "Functions", agg: "count" },
      { label: "Public", value: count((r) => r.Access === "Public") },
      { label: "Internal", value: count((r) => r.Access === "Internal") },
      { label: "HTTP routes", value: count((r) => r.Type === "HTTP") },
      { label: "Missing validators", value: String(unvalidated), tone: unvalidated ? "bad" : "good" },
    ],
    charts: [
      { type: "bar", title: "Functions by file", by: "File", agg: "count", top: 12, note: "functions" },
      { type: "bar", title: "By type", by: "Type", agg: "count", note: "functions" },
    ],
    table: { title: "Every function, gaps first", columns: ["Function", "Type", "Access", "Takes", "Checks"], limit: rows.length },
    footer,
  };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const [input, outDir] = process.argv.slice(2);
  if (!input || !outDir) {
    console.error("Usage: node apimap.mjs <functions.json> <out dir>");
    process.exit(2);
  }
  const json = JSON.parse(readFileSync(input, "utf8"));
  const spec = apiSpec(json);
  const { html } = buildPage(spec, outDir);
  mkdirSync(outDir, { recursive: true });
  writeFileSync(join(outDir, "spec.json"), JSON.stringify(spec, null, 2));
  writeFileSync(join(outDir, "index.html"), html);
  console.log(`${join(outDir, "index.html")}  ${spec.kind}  ${apiRows(json).length} functions`);
}
