// Reading the data a page is made from: CSV, TSV, JSON and XLSX, into rows
// of { column: value }. Numbers stay numbers. Nothing here leaves the Mac.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { extname } from "node:path";

// RFC 4180: quoted fields, doubled quotes, newlines inside quotes.
export function parseDelimited(text, sep = ",") {
  const rows = [];
  let row = [];
  let field = "";
  let quoted = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quoted) {
      if (c === '"' && text[i + 1] === '"') {
        field += '"';
        i++;
      } else if (c === '"') quoted = false;
      else field += c;
    } else if (c === '"' && field === "") quoted = true;
    else if (c === sep) {
      row.push(field);
      field = "";
    } else if (c === "\n" || c === "\r") {
      if (c === "\r" && text[i + 1] === "\n") i++;
      row.push(field);
      rows.push(row);
      row = [];
      field = "";
    } else field += c;
  }
  if (field !== "" || row.length) {
    row.push(field);
    rows.push(row);
  }
  return toObjects(rows.filter((r) => r.some((v) => v.trim() !== "")));
}

// "1,234.5", "$12", "45%" read as numbers; anything else stays text.
export function cell(v) {
  if (typeof v !== "string") return v;
  const t = v.trim();
  if (t === "") return null;
  const n = t.replace(/^[$€£¥]/, "").replace(/,/g, "").replace(/%$/, "");
  return /^-?\d+(\.\d+)?([eE][-+]?\d+)?$/.test(n) ? Number(n) : t;
}

function toObjects(rows) {
  if (!rows.length) return { columns: [], rows: [] };
  const columns = rows[0].map((h, i) => (h.trim() || `Column ${i + 1}`));
  return {
    columns,
    rows: rows.slice(1).map((r) => Object.fromEntries(columns.map((c, i) => [c, cell(r[i] ?? "")]))),
  };
}

// XLSX is a zip of XML. The first sheet (or the one named) is read with the
// system's unzip, so nothing needs installing. Shared strings, inline
// strings, numbers and booleans; formulas give their cached values.
export function readXlsx(file, sheetName) {
  const unzip = (entry) => {
    try {
      return execFileSync("unzip", ["-p", file, entry], { encoding: "utf8", maxBuffer: 256 * 1024 * 1024 });
    } catch {
      return "";
    }
  };
  const decode = (s) => s.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"').replace(/&apos;/g, "'").replace(/&amp;/g, "&");
  const shared = [...unzip("xl/sharedStrings.xml").matchAll(/<si>([\s\S]*?)<\/si>/g)].map((m) => decode([...m[1].matchAll(/<t[^>]*>([\s\S]*?)<\/t>/g)].map((t) => t[1]).join("")));
  let target = "xl/worksheets/sheet1.xml";
  const workbook = unzip("xl/workbook.xml");
  const sheets = [...workbook.matchAll(/<sheet [^>]*name="([^"]*)"[^>]*r:id="([^"]*)"/g)].map((m) => ({ name: decode(m[1]), rid: m[2] }));
  const wanted = sheetName ? sheets.find((s) => s.name === sheetName) : sheets[0];
  if (sheetName && !wanted) throw new Error(`No sheet "${sheetName}" in ${file}. Sheets: ${sheets.map((s) => s.name).join(", ")}`);
  if (wanted) {
    const rels = unzip("xl/_rels/workbook.xml.rels");
    const rel = new RegExp(`<Relationship [^>]*Id="${wanted.rid}"[^>]*Target="([^"]*)"`).exec(rels) ?? new RegExp(`<Relationship [^>]*Target="([^"]*)"[^>]*Id="${wanted.rid}"`).exec(rels);
    if (rel) target = rel[1].startsWith("/") ? rel[1].slice(1) : `xl/${rel[1].replace(/^\.\//, "")}`;
  }
  const xml = unzip(target);
  if (!xml) throw new Error(`Couldn't read ${target} from ${file}. Is it an .xlsx file?`);
  const colIndex = (ref) => {
    const letters = /^[A-Z]+/.exec(ref)[0];
    let n = 0;
    for (const ch of letters) n = n * 26 + (ch.charCodeAt(0) - 64);
    return n - 1;
  };
  const grid = [];
  for (const r of xml.matchAll(/<row[^>]*>([\s\S]*?)<\/row>/g)) {
    const out = [];
    for (const c of r[1].matchAll(/<c ([^>]*?)(?:\/>|>([\s\S]*?)<\/c>)/g)) {
      const attrs = c[1];
      const ref = /r="([A-Z]+\d+)"/.exec(attrs)?.[1];
      const type = /t="([^"]+)"/.exec(attrs)?.[1];
      const body = c[2] ?? "";
      const v = /<v>([\s\S]*?)<\/v>/.exec(body)?.[1];
      let value = "";
      if (type === "s" && v !== undefined) value = shared[Number(v)] ?? "";
      else if (type === "inlineStr") value = decode([...body.matchAll(/<t[^>]*>([\s\S]*?)<\/t>/g)].map((t) => t[1]).join(""));
      else if (type === "b") value = v === "1" ? "TRUE" : "FALSE";
      else if (v !== undefined) value = decode(v);
      out[ref ? colIndex(ref) : out.length] = value;
    }
    grid.push(Array.from(out, (x) => x ?? ""));
  }
  return { ...toObjects(grid.filter((r) => r.some((v) => String(v).trim() !== ""))), sheets: sheets.map((s) => s.name) };
}

export function readData(file, { sheet } = {}) {
  const ext = extname(file).toLowerCase();
  if (ext === ".xlsx" || ext === ".xlsm") return readXlsx(file, sheet);
  const text = readFileSync(file, "utf8").replace(/^﻿/, "");
  if (ext === ".json") {
    const j = JSON.parse(text);
    const rows = Array.isArray(j) ? j : (Object.values(j).find(Array.isArray) ?? []);
    const columns = [...new Set(rows.flatMap((r) => Object.keys(r ?? {})))];
    return { columns, rows: rows.map((r) => Object.fromEntries(columns.map((c) => [c, cell(r?.[c] ?? null)]))) };
  }
  if (ext === ".tsv" || ext === ".tab") return parseDelimited(text, "\t");
  if (ext === ".csv" || ext === ".txt") return parseDelimited(text, text.split("\n")[0].includes("\t") && !text.split("\n")[0].includes(",") ? "\t" : ",");
  throw new Error(`Can't read ${ext || "that"} files. Use CSV, TSV, JSON or XLSX (an .xls or Numbers file can be exported to one).`);
}

// Group rows by one column and total a value: sum (default), count, avg,
// min or max. Returns [{ label, value }] in first-seen order.
export function group(rows, by, value, agg = "sum") {
  const m = new Map();
  for (const r of rows) {
    const k = r[by] === null || r[by] === undefined || r[by] === "" ? "(blank)" : String(r[by]);
    const v = value ? r[value] : 1;
    const n = typeof v === "number" ? v : null;
    if (!m.has(k)) m.set(k, []);
    if (agg === "count" || n !== null) m.get(k).push(agg === "count" ? 1 : n);
  }
  return [...m.entries()].map(([label, vs]) => ({ label, value: total(vs, agg) }));
}

export function total(vs, agg = "sum") {
  if (!vs.length) return 0;
  if (agg === "count") return vs.length;
  if (agg === "avg") return vs.reduce((a, b) => a + b, 0) / vs.length;
  if (agg === "min") return Math.min(...vs);
  if (agg === "max") return Math.max(...vs);
  return vs.reduce((a, b) => a + b, 0);
}

export function numbers(rows, col) {
  return rows.map((r) => r[col]).filter((v) => typeof v === "number");
}
