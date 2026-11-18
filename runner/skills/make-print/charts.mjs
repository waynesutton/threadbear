// Static SVG charts for Prints, drawn to the dataviz skill's rules and the
// Prints look (theme.mjs): thin marks, 4px rounded ends, a 2px surface gap
// between fills, 1.75px lines, a dashed recessive grid, tabular numbers,
// one accent per chart (a column chart lights its peak and its latest and
// dims the rest), categorical colour in a fixed order only for parts and
// several series (6 slots, no violet, the rest fold into a grey "Other"),
// a legend for 2+ series, direct labels where they fit, a native hover
// title on every mark, and a table view always. No script: pages are static.

const esc = (s) => String(s ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
export const SLOTS = 6;
// Gradient ids, unique on the page however many charts it has.
let uid = 0;

// Numbers the way people read them: 1.2K, 3.4M, with a unit.
export function fmt(n, unit = "") {
  if (n === null || n === undefined || Number.isNaN(n)) return "–";
  const a = Math.abs(n);
  let s;
  if (a >= 1e9) s = `${(n / 1e9).toFixed(a >= 1e10 ? 0 : 1)}B`;
  else if (a >= 1e6) s = `${(n / 1e6).toFixed(a >= 1e7 ? 0 : 1)}M`;
  else if (a >= 1e4) s = `${(n / 1e3).toFixed(a >= 1e5 ? 0 : 1)}K`;
  else if (Number.isInteger(n) || a >= 1000) s = Math.round(n).toLocaleString("en-US");
  else s = n.toLocaleString("en-US", { maximumFractionDigits: a < 10 ? 2 : 1 });
  if (!unit) return s;
  return ["$", "€", "£", "¥"].includes(unit) ? `${n < 0 ? "-" : ""}${unit}${s.replace(/^-/, "")}` : `${s}${unit === "%" ? "%" : ` ${unit}`}`;
}

// The biggest `slots`, then everything else as one grey "Other".
export function fold(items, slots = SLOTS) {
  const sorted = [...items].sort((a, b) => b.value - a.value);
  if (sorted.length <= slots) return sorted;
  const kept = sorted.slice(0, slots - 1);
  const rest = sorted.slice(slots - 1);
  return [...kept, { label: `Other (${rest.length})`, value: rest.reduce((s, x) => s + x.value, 0), other: true }];
}

const cls = (i, item) => (item?.other ? "sOther" : `s${(i % SLOTS) + 1}`);

function niceMax(v) {
  if (v <= 0) return 1;
  const p = 10 ** Math.floor(Math.log10(v));
  return [1, 1.2, 1.5, 2, 2.5, 3, 4, 5, 6, 8, 10].map((m) => m * p).find((m) => m >= v);
}

function dataTable(head, rows) {
  return `<details class="data"><summary>Show the data</summary><table><thead><tr>${head.map((h) => `<th>${esc(h)}</th>`).join("")}</tr></thead><tbody>${rows
    .map((r) => `<tr>${r.map((c, i) => `<td${i ? ' class="num"' : ""}>${esc(c)}</td>`).join("")}</tr>`)
    .join("")}</tbody></table></details>`;
}

const frame = (title, note, body, table, extra = "") =>
  `<figure class="chart">${title || extra ? `<figcaption><div class="cap"><b>${esc(title ?? "")}</b>${note ? `<span>${esc(note)}</span>` : ""}</div>${extra}</figcaption>` : ""}${body}${table}</figure>`;

const legend = (items) => `<div class="legend">${items.map((it, i) => `<span><b class="${cls(i, it)}"></b>${esc(it.label)}</span>`).join("")}</div>`;

// Ranked horizontal bars: the default for comparing categories. One colour,
// each on a faint track, its number at the end.
export function bar(items, { title, note, unit = "", top = 12, width = 520 } = {}) {
  const list = [...items].sort((a, b) => b.value - a.value);
  const shown = list.length > top ? [...list.slice(0, top - 1), { label: `Other (${list.length - top + 1})`, value: list.slice(top - 1).reduce((s, x) => s + x.value, 0), other: true }] : list;
  const max = Math.max(...shown.map((x) => x.value), 0) || 1;
  const rowH = 28;
  const W = width;
  const labelW = 150;
  const valueW = 60;
  const barW = W - labelW - valueW - 12;
  const H = shown.length * rowH;
  const rows = shown
    .map((it, i) => {
      const w = Math.max(3, (Math.max(0, it.value) / max) * barW);
      const y = i * rowH + 8;
      return `<g><title>${esc(it.label)}: ${esc(fmt(it.value, unit))}</title><text x="0" y="${y + 10}" class="lab">${esc(it.label.length > 26 ? `${it.label.slice(0, 25)}…` : it.label)}</text><rect x="${labelW}" y="${y + 1}" width="${barW}" height="12" rx="4" class="track"/><rect x="${labelW}" y="${y + 1}" width="${w.toFixed(1)}" height="12" rx="4" class="${it.other ? "sOther" : "s1"}"/><text x="${W}" y="${y + 11}" class="val" text-anchor="end">${esc(fmt(it.value, unit))}</text></g>`;
    })
    .join("");
  const svg = `<svg viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(title ?? "Bar chart")}">${rows}</svg>`;
  return frame(title, note, svg, dataTable(["", "Value"], shown.map((x) => [x.label, fmt(x.value, unit)])));
}

const gridLines = (W, pad, Y, ticks, unit) =>
  ticks
    .map((v, k) => `<line x1="${pad.l}" x2="${W - pad.r}" y1="${Y(v).toFixed(1)}" y2="${Y(v).toFixed(1)}" class="${k === 0 ? "axis" : "grid"}"/><text x="${pad.l - 8}" y="${(Y(v) + 3.5).toFixed(1)}" class="tick" text-anchor="end">${esc(fmt(v, unit))}</text>`)
    .join("");

// Vertical columns in the data's own order (months, stages). The peak and
// the latest are lit; the rest are dimmed, so the eye goes where it should.
export function column(items, { title, note, unit = "", width = 520 } = {}) {
  const W = width;
  const H = 230;
  const pad = { l: 44, r: 6, t: 18, b: 28 };
  const max = niceMax(Math.max(...items.map((x) => x.value), 0));
  const iw = W - pad.l - pad.r;
  const ih = H - pad.t - pad.b;
  const Y = (v) => pad.t + ih * (1 - v / max);
  const slot = iw / Math.max(1, items.length);
  const bw = Math.max(4, Math.min(36, slot * 0.62));
  const grid = gridLines(W, pad, Y, [0, 0.25, 0.5, 0.75, 1].map((f) => max * f), unit);
  // Every label that fits: about 7 units a character, with room between.
  const longest = Math.max(...items.map((x) => Math.min(10, String(x.label).length)), 1);
  const every = Math.max(1, Math.ceil((longest * 7 + 10) / slot));
  const peak = items.reduce((b, x, i) => (x.value > items[b].value ? i : b), 0);
  const marks = items
    .map((it, i) => {
      const h = Math.max(2, (Math.max(0, it.value) / max) * ih);
      const x = pad.l + slot * i + (slot - bw) / 2;
      const y = pad.t + ih - h;
      const r = Math.min(4, bw / 2, h / 2);
      const lit = i === peak || i === items.length - 1;
      const label = i % every === 0 ? `<text x="${(x + bw / 2).toFixed(1)}" y="${H - pad.b + 17}" class="tick" text-anchor="middle">${esc(String(it.label).slice(0, 10))}</text>` : "";
      const direct = lit ? `<text x="${(x + bw / 2).toFixed(1)}" y="${(y - 7).toFixed(1)}" class="val" text-anchor="middle">${esc(fmt(it.value, unit))}</text>` : "";
      return `<g><title>${esc(it.label)}: ${esc(fmt(it.value, unit))}</title><path d="M${x.toFixed(1)},${pad.t + ih} V${(y + r).toFixed(1)} q0,-${r} ${r},-${r} h${(bw - 2 * r).toFixed(1)} q${r},0 ${r},${r} V${pad.t + ih} Z" class="s1${lit ? "" : " dimmed"}"/>${direct}${label}</g>`;
    })
    .join("");
  return frame(title, note, `<svg viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(title ?? "Column chart")}">${grid}${marks}</svg>`, dataTable(["", "Value"], items.map((x) => [x.label, fmt(x.value, unit)])));
}

// Lines over an ordered x, up to six series, each named at its end. One
// series gets a faint area under it and its last point marked.
export function line(series, { title, note, unit = "", width = 520 } = {}) {
  const ss = series.slice(0, SLOTS);
  const xs = [...new Set(ss.flatMap((s) => s.points.map((p) => String(p.x))))];
  const W = width;
  const H = 240;
  const pad = { l: 44, r: ss.length > 1 ? 70 : 12, t: 16, b: 28 };
  const all = ss.flatMap((s) => s.points.map((p) => p.y)).filter((v) => typeof v === "number");
  const lo = Math.min(0, ...all);
  const max = niceMax(Math.max(...all, 0));
  const iw = W - pad.l - pad.r;
  const ih = H - pad.t - pad.b;
  const X = (x) => pad.l + (xs.length < 2 ? iw / 2 : (xs.indexOf(String(x)) / (xs.length - 1)) * iw);
  const Y = (y) => pad.t + ih * (1 - (y - lo) / (max - lo || 1));
  const grid = gridLines(W, pad, Y, [0, 0.25, 0.5, 0.75, 1].map((f) => lo + (max - lo) * f), unit);
  const every = Math.ceil(xs.length / 8);
  const ticks = xs.map((x, i) => (i % every === 0 || (i === xs.length - 1 && (xs.length - 1) % every >= every / 2 + 0.5) ? `<text x="${X(x).toFixed(1)}" y="${H - pad.b + 17}" class="tick" text-anchor="middle">${esc(x.slice(0, 10))}</text>` : "")).join("");
  const ends = [];
  const id = `g${++uid}`;
  const defs = ss.length === 1 ? `<defs><linearGradient id="${id}" x1="0" x2="0" y1="0" y2="1"><stop offset="0" class="area-top"/><stop offset="1" class="area-end"/></linearGradient></defs>` : "";
  const paths = ss
    .map((s, i) => {
      const pts = s.points.filter((p) => typeof p.y === "number");
      const d = pts.map((p, j) => `${j ? "L" : "M"}${X(p.x).toFixed(1)},${Y(p.y).toFixed(1)}`).join("");
      const last = pts[pts.length - 1];
      if (last && ss.length > 1) ends.push({ y: Y(last.y), x: X(last.x), name: s.name, i });
      const area = ss.length === 1 && pts.length > 1 ? `<path d="${d}L${X(last.x).toFixed(1)},${pad.t + ih}L${X(pts[0].x).toFixed(1)},${pad.t + ih}Z" fill="url(#${id})" class="area"/>` : "";
      // Every point keeps a hover title; only the last is drawn.
      const hits = pts.length <= 60 ? pts.map((p) => `<circle cx="${X(p.x).toFixed(1)}" cy="${Y(p.y).toFixed(1)}" r="7" class="hit"><title>${esc(s.name)} · ${esc(p.x)}: ${esc(fmt(p.y, unit))}</title></circle>`).join("") : "";
      const end = last ? `<circle cx="${X(last.x).toFixed(1)}" cy="${Y(last.y).toFixed(1)}" r="3.5" class="dot s${i + 1}"/>` : "";
      return `${area}<path d="${d}" class="ln s${i + 1}"/>${end}${hits}`;
    })
    .join("");
  // Direct labels at the right, nudged apart so they never collide.
  ends.sort((a, b) => a.y - b.y);
  for (let k = 1; k < ends.length; k++) if (ends[k].y - ends[k - 1].y < 13) ends[k].y = ends[k - 1].y + 13;
  const labels = ends.map((e) => `<text x="${W - pad.r + 10}" y="${(e.y + 4).toFixed(1)}" class="lab">${esc(e.name.length > 12 ? `${e.name.slice(0, 11)}…` : e.name)}</text>`).join("");
  const body = `<svg viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(title ?? "Line chart")}">${defs}${grid}${ticks}${paths}${labels}</svg>`;
  const head = ["", ...ss.map((s) => s.name)];
  const rows = xs.map((x) => [x, ...ss.map((s) => fmt(s.points.find((p) => String(p.x) === x)?.y, unit))]);
  return frame(title, note, body, dataTable(head, rows), ss.length > 1 ? legend(ss.map((s) => ({ label: s.name }))) : "");
}

const partRows = (parts, sum, unit) =>
  `<div class="parts">${parts.map((it, i) => `<div><span><b class="${cls(i, it)}"></b>${esc(it.label)}</span><em>${esc(fmt(it.value, unit))}</em><strong>${((it.value / sum) * 100).toFixed(1)}%</strong></div>`).join("")}</div>`;

// One whole in up to six parts: a slim segmented bar, its parts listed below.
export function segmented(items, { title, note, unit = "", width = 520 } = {}) {
  const parts = fold(items.filter((x) => x.value > 0));
  const sum = parts.reduce((s, x) => s + x.value, 0) || 1;
  const W = width;
  let x = 0;
  const segs = parts
    .map((it, i) => {
      const w = (it.value / sum) * W;
      const seg = `<rect x="${(x + (i ? 1.5 : 0)).toFixed(1)}" y="0" width="${Math.max(1, w - (i ? 3 : 0)).toFixed(1)}" height="14" rx="4" class="${cls(i, it)}"><title>${esc(it.label)}: ${esc(fmt(it.value, unit))} (${((it.value / sum) * 100).toFixed(1)}%)</title></rect>`;
      x += w;
      return seg;
    })
    .join("");
  return frame(title, note, `<div class="seg-total">${esc(fmt(sum, unit))}<span>total</span></div><svg viewBox="0 0 ${W} 14" class="seg" role="img" aria-label="${esc(title ?? "Parts of a whole")}">${segs}</svg>${partRows(parts, sum, unit)}`, dataTable(["", "Value", "Share"], parts.map((p) => [p.label, fmt(p.value, unit), `${((p.value / sum) * 100).toFixed(1)}%`])));
}

// A donut, for when a pie is asked for by name: up to six slices plus
// Other, the total in the middle and each part named with its share beside.
export function pie(items, { title, note, unit = "" } = {}) {
  const parts = fold(items.filter((x) => x.value > 0));
  const sum = parts.reduce((s, x) => s + x.value, 0) || 1;
  const R = 92;
  const r = 70;
  const cx = 110;
  const cy = 110;
  let a = -Math.PI / 2;
  const gap = parts.length > 1 ? 0.014 : 0;
  const pt = (rad, ang) => `${(cx + rad * Math.cos(ang)).toFixed(2)},${(cy + rad * Math.sin(ang)).toFixed(2)}`;
  const slices = parts
    .map((it, i) => {
      const sweep = (it.value / sum) * Math.PI * 2;
      const a0 = a + gap;
      const a1 = a + sweep - gap;
      a += sweep;
      const big = a1 - a0 > Math.PI ? 1 : 0;
      const d = parts.length === 1 ? `M${cx},${cy - R} A${R},${R} 0 1 1 ${cx - 0.01},${cy - R} L${cx - 0.01},${cy - r} A${r},${r} 0 1 0 ${cx},${cy - r} Z` : `M${pt(R, a0)} A${R},${R} 0 ${big} 1 ${pt(R, a1)} L${pt(r, a1)} A${r},${r} 0 ${big} 0 ${pt(r, a0)} Z`;
      return `<path d="${d}" class="${cls(i, it)}"><title>${esc(it.label)}: ${esc(fmt(it.value, unit))} (${((it.value / sum) * 100).toFixed(1)}%)</title></path>`;
    })
    .join("");
  const centre = `<text x="${cx}" y="${cy + 4}" text-anchor="middle" class="big">${esc(fmt(sum, unit))}</text><text x="${cx}" y="${cy + 24}" text-anchor="middle" class="tick">total</text>`;
  return frame(title, note, `<div class="pie"><svg viewBox="0 0 220 220" role="img" aria-label="${esc(title ?? "Pie chart")}">${slices}${centre}</svg>${partRows(parts, sum, unit).replace('class="parts"', 'class="parts pieparts"')}</div>`, dataTable(["", "Value", "Share"], parts.map((p) => [p.label, fmt(p.value, unit), `${((p.value / sum) * 100).toFixed(1)}%`])));
}

// A small line with no axes, for a tile or a table cell: the shape, and
// its last point marked. `kind: "bars"` draws mini columns instead.
export function sparkline(values, { kind = "line", tone = "" } = {}) {
  const v = (values ?? []).map(Number).filter((n) => Number.isFinite(n));
  if (v.length < 2) return "";
  const W = 120;
  const H = 28;
  const lo = Math.min(...v);
  const hi = Math.max(...v);
  const Y = (n) => 3 + (H - 6) * (1 - (n - lo) / (hi - lo || 1));
  const c = tone === "down" ? "sp-down" : tone === "up" ? "sp-up" : "sp";
  if (kind === "bars") {
    const bw = W / v.length;
    return `<svg class="spark" viewBox="0 0 ${W} ${H}" preserveAspectRatio="none" aria-hidden="true">${v.map((n, i) => `<rect x="${(i * bw + 1).toFixed(1)}" y="${Y(n).toFixed(1)}" width="${Math.max(1, bw - 2).toFixed(1)}" height="${(H - Y(n)).toFixed(1)}" rx="1.5" class="${c}${i === v.length - 1 ? "" : " dimmed"}"/>`).join("")}</svg>`;
  }
  const X = (i) => 2 + (i / (v.length - 1)) * (W - 6);
  const d = v.map((n, i) => `${i ? "L" : "M"}${X(i).toFixed(1)},${Y(n).toFixed(1)}`).join("");
  return `<svg class="spark" viewBox="0 0 ${W} ${H}" preserveAspectRatio="none" aria-hidden="true"><path d="${d}" class="sp-ln ${c}" vector-effect="non-scaling-stroke"/></svg>`;
}

// Whether a change went up (1), down (-1) or neither (0): a number's sign,
// or a string's leading + or -.
function deltaSign(delta) {
  if (typeof delta === "number") return Math.sign(delta);
  const text = String(delta ?? "");
  return /^\s*[+▲↑]/.test(text) ? 1 : /^\s*[-−▼↓]/.test(text) ? -1 : 0;
}

// Whether that change is good news (1), bad (-1) or neither: up is good
// unless `better` is "down" (costs, errors, latency).
const verdict = (delta, better) => (better === "down" ? -1 : 1) * deltaSign(delta);

// A change as a pill: "+12%" green, "-3%" red, anything else plain, with an
// arrow so it never leans on colour alone. A number gets its sign and unit.
export function deltaPill(delta, { unit = "%", better = "up" } = {}) {
  if (delta === undefined || delta === null || delta === "") return "";
  const sign = deltaSign(delta);
  const text = (typeof delta === "number" ? fmt(Math.abs(delta), unit) : String(delta)).replace(/^\s*[+▲↑−▼↓-]\s*/, "");
  const good = verdict(delta, better);
  return `<span class="pill${good > 0 ? " up" : good < 0 ? " down" : ""}">${sign > 0 ? "↑ " : sign < 0 ? "↓ " : ""}${esc(text)}</span>`;
}

// Big numbers. A single number is a tile, never a one-bar chart. A tile
// can carry a change (`delta`, a pill) and a little history (`spark`).
export function tiles(list) {
  return `<div class="tiles">${list
    .map((t) => {
      const tone = ["warn", "bad", "good"].includes(t.tone) ? ` ${t.tone}` : "";
      const pill = deltaPill(t.delta, { unit: t.deltaUnit ?? "%", better: t.better });
      const good = verdict(t.delta, t.better);
      const spark = sparkline(t.spark, { kind: t.sparkKind, tone: good > 0 ? "up" : good < 0 ? "down" : "" });
      const row = pill || spark || t.note ? `<div class="row">${pill}${t.note ? `<span class="n">${esc(t.note)}</span>` : ""}${spark}</div>` : "";
      return `<div class="card tile${tone}"><div class="k">${esc(t.label)}</div><div class="v">${esc(t.value)}</div>${row}</div>`;
    })
    .join("")}</div>`;
}

// Rows. Columns named in `pills` show as chips, those in `deltas` as
// green or red change pills, and numbers sit right, in tabular figures.
export function table(columns, rows, { title, limit = 50, units = {}, pills = [], deltas = [] } = {}) {
  const shown = rows.slice(0, limit);
  const cell = (r, c) => {
    const v = r[c];
    if (deltas.includes(c) && v !== null && v !== undefined && v !== "") return `<td class="num">${deltaPill(v, { unit: units[c] ?? "%" })}</td>`;
    if (pills.includes(c) && v) return `<td><span class="pill">${esc(v)}</span></td>`;
    return `<td${typeof v === "number" ? ' class="num"' : ""}>${esc(typeof v === "number" ? fmt(v, units[c]) : v)}</td>`;
  };
  const body = `<div class="tablewrap"><table class="rows"><thead><tr>${columns.map((c) => `<th${shown.length && typeof shown[0][c] === "number" ? ' class="num"' : ""}>${esc(c)}</th>`).join("")}</tr></thead><tbody>${shown
    .map((r) => `<tr>${columns.map((c) => cell(r, c)).join("")}</tr>`)
    .join("")}</tbody></table></div>${rows.length > limit ? `<p class="more">Showing ${limit} of ${rows.length} rows.</p>` : ""}`;
  return frame(title, null, body, "");
}

export const CHART_CSS = `
.s1{fill:var(--c1);stroke:var(--c1)}.s2{fill:var(--c2);stroke:var(--c2)}.s3{fill:var(--c3);stroke:var(--c3)}.s4{fill:var(--c4);stroke:var(--c4)}.s5{fill:var(--c5);stroke:var(--c5)}.s6{fill:var(--c6);stroke:var(--c6)}.sOther{fill:var(--other);stroke:var(--other)}
b.s1{background:var(--c1)}b.s2{background:var(--c2)}b.s3{background:var(--c3)}b.s4{background:var(--c4)}b.s5{background:var(--c5)}b.s6{background:var(--c6)}b.sOther{background:var(--other)}
rect[class^=s],path[class^=s]:not(.ln):not(.sp-ln){stroke:none}
.dimmed{opacity:.32}
.track{fill:var(--raise);stroke:var(--line);stroke-width:1}
.ln{fill:none!important;stroke-width:1.75;stroke-linejoin:round;stroke-linecap:round}
.dot{stroke:var(--card)!important;stroke-width:2}
.hit{fill:transparent;stroke:none}
.area{stroke:none}.area-top{stop-color:var(--c1);stop-opacity:.16}.area-end{stop-color:var(--c1);stop-opacity:0}
.chart{margin:0}
.chart figcaption{display:flex;justify-content:space-between;gap:12px;align-items:flex-start;margin-bottom:18px}
.chart figcaption .cap{display:grid;gap:3px;flex:none}
.chart figcaption .legend{flex:1;min-width:0}
.chart figcaption b{font-size:15px;font-weight:600;letter-spacing:-.011em;color:var(--text)}
.chart figcaption span{color:var(--faint);font-size:12px}
.chart svg{display:block;width:100%;height:auto;overflow:visible}
.chart svg text{font:12.5px -apple-system,BlinkMacSystemFont,"SF Pro Text",system-ui,sans-serif;font-variant-numeric:tabular-nums}
.lab{fill:var(--text)}.val{fill:var(--text);font-weight:600}.tick{fill:var(--faint)}.big{fill:var(--text);font-size:26px!important;font-weight:650;letter-spacing:-.02em}
.grid{stroke:var(--grid);stroke-width:1;stroke-dasharray:2 4}.axis{stroke:var(--line2);stroke-width:1}
.legend{display:flex;flex-wrap:wrap;gap:6px;justify-content:flex-end}
.legend span{display:inline-flex;align-items:center;height:22px;padding:0 8px;border:1px solid var(--line);border-radius:999px;font-size:11.5px;color:var(--dim);white-space:nowrap}
.legend b,.parts b{display:inline-block;width:8px;height:8px;border-radius:2px;margin-right:7px}
.seg-total{font:650 26px/1 -apple-system,BlinkMacSystemFont,"SF Pro Display",system-ui,sans-serif;letter-spacing:-.02em;font-variant-numeric:tabular-nums;margin:0 0 14px}
.seg-total span{font:400 12px/1 -apple-system,system-ui,sans-serif;color:var(--faint);margin-left:8px;letter-spacing:0}
.parts{display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:0 28px;margin-top:16px;font-size:13px}
.parts div{display:grid;grid-template-columns:1fr auto 52px;gap:10px;align-items:center;padding:8px 0;border-bottom:1px solid var(--line)}
.parts span{white-space:nowrap;overflow:hidden;text-overflow:ellipsis;display:flex;align-items:center}
.parts em{font-style:normal;color:var(--dim);font-variant-numeric:tabular-nums}
.parts strong{text-align:right;font-weight:600;font-variant-numeric:tabular-nums}
.pie{display:grid;grid-template-columns:200px 1fr;gap:32px;align-items:center}
.pieparts{grid-template-columns:1fr;margin-top:0}
.data{margin-top:14px;font-size:12px;color:var(--faint)}
.data summary{cursor:pointer;width:max-content}
.data summary:hover{color:var(--dim)}
.data table,.rows{width:100%;border-collapse:collapse;font-size:13px;margin-top:8px;color:var(--text)}
.data th,.data td,.rows th,.rows td{text-align:left;padding:9px 10px;border-bottom:1px solid var(--line);white-space:nowrap}
.rows td{max-width:340px;overflow:hidden;text-overflow:ellipsis}
.data th,.rows th{color:var(--faint);font-weight:500;font-size:11.5px;letter-spacing:.04em;text-transform:uppercase}
.rows tbody tr:last-child td,.data tbody tr:last-child td{border-bottom:0}
td.num,th.num{text-align:right;font-variant-numeric:tabular-nums}
.tablewrap{overflow-x:auto;margin:0 calc(var(--pad) * -1);padding:0 var(--pad)}
.spark{display:block;width:100%;height:26px;overflow:visible}
.sp-ln{fill:none;stroke-width:1.5;stroke-linecap:round;stroke-linejoin:round}
.sp{stroke:var(--accent);fill:var(--accent)}.sp-up{stroke:var(--good);fill:var(--good)}.sp-down{stroke:var(--bad);fill:var(--bad)}
.sp-ln.sp,.sp-ln.sp-up,.sp-ln.sp-down{fill:none}
rect.sp,rect.sp-up,rect.sp-down{stroke:none}
@media (max-width:640px){.pie{grid-template-columns:1fr}.pie svg{max-width:200px;margin:0 auto}}
`;
