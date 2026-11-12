// Static SVG charts for Prints, drawn to the dataviz skill's rules: thin
// marks with 4px rounded ends on the baseline, a 2px surface gap between
// fills, 2px lines, recessive grid, categorical colour in a fixed order
// (validated light and dark, 10 Nov 2026: 6 slots, no violet, the rest
// fold into a grey "Other"), a legend for 2+ series, direct labels where
// they fit, a native hover title on every mark, and a table view always.
// No script: pages are static.

const esc = (s) => String(s ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
export const SLOTS = 6;

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

const frame = (title, note, body, table) =>
  `<figure class="chart">${title ? `<figcaption><b>${esc(title)}</b>${note ? `<span>${esc(note)}</span>` : ""}</figcaption>` : ""}${body}${table}</figure>`;

const legend = (items) => `<div class="legend">${items.map((it, i) => `<span><b class="${cls(i, it)}"></b>${esc(it.label)}</span>`).join("")}</div>`;

// Ranked horizontal bars: the default for comparing categories.
export function bar(items, { title, note, unit = "", top = 12 } = {}) {
  const list = [...items].sort((a, b) => b.value - a.value);
  const shown = list.length > top ? [...list.slice(0, top - 1), { label: `Other (${list.length - top + 1})`, value: list.slice(top - 1).reduce((s, x) => s + x.value, 0), other: true }] : list;
  const max = Math.max(...shown.map((x) => x.value), 0) || 1;
  const rowH = 26;
  const W = 600;
  const labelW = 170;
  const valueW = 70;
  const barW = W - labelW - valueW - 16;
  const H = shown.length * rowH + 4;
  const rows = shown
    .map((it, i) => {
      const w = Math.max(2, (Math.max(0, it.value) / max) * barW);
      const y = i * rowH + 6;
      return `<g><title>${esc(it.label)}: ${esc(fmt(it.value, unit))}</title><text x="${labelW - 10}" y="${y + 11}" class="lab" text-anchor="end">${esc(it.label.length > 26 ? `${it.label.slice(0, 25)}…` : it.label)}</text><rect x="${labelW}" y="${y + 2}" width="${w.toFixed(1)}" height="12" rx="4" class="${it.other ? "sOther" : "s1"}"/><text x="${labelW + w + 8}" y="${y + 12}" class="val">${esc(fmt(it.value, unit))}</text></g>`;
    })
    .join("");
  const svg = `<svg viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(title ?? "Bar chart")}">${rows}</svg>`;
  return frame(title, note, svg, dataTable(["", "Value"], shown.map((x) => [x.label, fmt(x.value, unit)])));
}

// Vertical columns in the data's own order (months, stages).
export function column(items, { title, note, unit = "" } = {}) {
  const W = 600;
  const H = 240;
  const pad = { l: 44, r: 8, t: 14, b: 30 };
  const max = niceMax(Math.max(...items.map((x) => x.value), 0));
  const iw = W - pad.l - pad.r;
  const ih = H - pad.t - pad.b;
  const slot = iw / Math.max(1, items.length);
  const bw = Math.max(4, Math.min(42, slot - 6));
  const grid = [0, 0.25, 0.5, 0.75, 1]
    .map((f) => {
      const y = pad.t + ih * (1 - f);
      return `<line x1="${pad.l}" x2="${W - pad.r}" y1="${y}" y2="${y}" class="${f === 0 ? "axis" : "grid"}"/><text x="${pad.l - 6}" y="${y + 3}" class="tick" text-anchor="end">${esc(fmt(max * f, unit))}</text>`;
    })
    .join("");
  const every = Math.ceil(items.length / 12);
  const peak = items.reduce((b, x, i) => (x.value > items[b].value ? i : b), 0);
  const marks = items
    .map((it, i) => {
      const h = Math.max(1, (Math.max(0, it.value) / max) * ih);
      const x = pad.l + slot * i + (slot - bw) / 2;
      const y = pad.t + ih - h;
      const label = i % every === 0 ? `<text x="${x + bw / 2}" y="${H - pad.b + 16}" class="tick" text-anchor="middle">${esc(String(it.label).slice(0, 10))}</text>` : "";
      const direct = i === peak || i === items.length - 1 ? `<text x="${x + bw / 2}" y="${y - 5}" class="val" text-anchor="middle">${esc(fmt(it.value, unit))}</text>` : "";
      return `<g><title>${esc(it.label)}: ${esc(fmt(it.value, unit))}</title><path d="M${x},${pad.t + ih} V${y + 4} q0,-4 4,-4 h${bw - 8} q4,0 4,4 V${pad.t + ih} Z" class="s1"/>${direct}${label}</g>`;
    })
    .join("");
  return frame(title, note, `<svg viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(title ?? "Column chart")}">${grid}${marks}</svg>`, dataTable(["", "Value"], items.map((x) => [x.label, fmt(x.value, unit)])));
}

// Lines over an ordered x, up to six series, each named at its end.
export function line(series, { title, note, unit = "" } = {}) {
  const ss = series.slice(0, SLOTS);
  const xs = [...new Set(ss.flatMap((s) => s.points.map((p) => String(p.x))))];
  const W = 600;
  const H = 250;
  const pad = { l: 44, r: ss.length > 1 ? 96 : 14, t: 14, b: 30 };
  const all = ss.flatMap((s) => s.points.map((p) => p.y)).filter((v) => typeof v === "number");
  const lo = Math.min(0, ...all);
  const max = niceMax(Math.max(...all, 0));
  const iw = W - pad.l - pad.r;
  const ih = H - pad.t - pad.b;
  const X = (x) => pad.l + (xs.length < 2 ? iw / 2 : (xs.indexOf(String(x)) / (xs.length - 1)) * iw);
  const Y = (y) => pad.t + ih * (1 - (y - lo) / (max - lo || 1));
  const grid = [0, 0.25, 0.5, 0.75, 1]
    .map((f) => {
      const v = lo + (max - lo) * f;
      return `<line x1="${pad.l}" x2="${W - pad.r}" y1="${Y(v)}" y2="${Y(v)}" class="${v === 0 ? "axis" : "grid"}"/><text x="${pad.l - 6}" y="${Y(v) + 3}" class="tick" text-anchor="end">${esc(fmt(v, unit))}</text>`;
    })
    .join("");
  const every = Math.ceil(xs.length / 8);
  const ticks = xs.map((x, i) => (i % every === 0 || (i === xs.length - 1 && (xs.length - 1) % every >= every / 2 + 0.5) ? `<text x="${X(x)}" y="${H - pad.b + 16}" class="tick" text-anchor="middle">${esc(x.slice(0, 10))}</text>` : "")).join("");
  const ends = [];
  const paths = ss
    .map((s, i) => {
      const pts = s.points.filter((p) => typeof p.y === "number");
      const d = pts.map((p, j) => `${j ? "L" : "M"}${X(p.x).toFixed(1)},${Y(p.y).toFixed(1)}`).join("");
      const last = pts[pts.length - 1];
      if (last && ss.length > 1) ends.push({ y: Y(last.y), x: X(last.x), name: s.name, i });
      const dots = pts.length <= 24 ? pts.map((p) => `<circle cx="${X(p.x).toFixed(1)}" cy="${Y(p.y).toFixed(1)}" r="4" class="dot s${i + 1}"><title>${esc(s.name)} · ${esc(p.x)}: ${esc(fmt(p.y, unit))}</title></circle>`).join("") : "";
      return `<path d="${d}" class="ln s${i + 1}"/>${dots}`;
    })
    .join("");
  // Direct labels at the right, nudged apart so they never collide.
  ends.sort((a, b) => a.y - b.y);
  for (let k = 1; k < ends.length; k++) if (ends[k].y - ends[k - 1].y < 13) ends[k].y = ends[k - 1].y + 13;
  const labels = ends.map((e) => `<text x="${W - pad.r + 8}" y="${e.y + 4}" class="lab">${esc(e.name.length > 12 ? `${e.name.slice(0, 11)}…` : e.name)}</text>`).join("");
  const body = `<svg viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(title ?? "Line chart")}">${grid}${ticks}${paths}${labels}</svg>${ss.length > 1 ? legend(ss.map((s) => ({ label: s.name }))) : ""}`;
  const head = ["", ...ss.map((s) => s.name)];
  const rows = xs.map((x) => [x, ...ss.map((s) => fmt(s.points.find((p) => String(p.x) === x)?.y, unit))]);
  return frame(title, note, body, dataTable(head, rows));
}

// One whole in up to six parts: a tall segmented bar, labelled below.
export function segmented(items, { title, note, unit = "" } = {}) {
  const parts = fold(items.filter((x) => x.value > 0));
  const sum = parts.reduce((s, x) => s + x.value, 0) || 1;
  const W = 600;
  let x = 0;
  const segs = parts
    .map((it, i) => {
      const w = (it.value / sum) * W;
      const seg = `<rect x="${(x + (i ? 1 : 0)).toFixed(1)}" y="0" width="${Math.max(1, w - (i ? 2 : 0)).toFixed(1)}" height="28" rx="4" class="${cls(i, it)}"><title>${esc(it.label)}: ${esc(fmt(it.value, unit))} (${((it.value / sum) * 100).toFixed(1)}%)</title></rect>`;
      x += w;
      return seg;
    })
    .join("");
  const keys = `<div class="parts">${parts.map((it, i) => `<div><span><b class="${cls(i, it)}"></b>${esc(it.label)}</span><strong>${((it.value / sum) * 100).toFixed(1)}%</strong><em>${esc(fmt(it.value, unit))}</em></div>`).join("")}</div>`;
  return frame(title, note, `<svg viewBox="0 0 ${W} 28" class="seg" role="img" aria-label="${esc(title ?? "Parts of a whole")}">${segs}</svg>${keys}`, dataTable(["", "Value", "Share"], parts.map((p) => [p.label, fmt(p.value, unit), `${((p.value / sum) * 100).toFixed(1)}%`])));
}

// A donut, for when a pie is asked for by name: up to six slices plus
// Other, each named with its share beside it.
export function pie(items, { title, note, unit = "" } = {}) {
  const parts = fold(items.filter((x) => x.value > 0));
  const sum = parts.reduce((s, x) => s + x.value, 0) || 1;
  const R = 90;
  const r = 56;
  const cx = 110;
  const cy = 110;
  let a = -Math.PI / 2;
  const gap = parts.length > 1 ? 0.012 : 0;
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
  const centre = `<text x="${cx}" y="${cy - 2}" text-anchor="middle" class="big">${esc(fmt(sum, unit))}</text><text x="${cx}" y="${cy + 16}" text-anchor="middle" class="tick">total</text>`;
  const keys = `<div class="parts pieparts">${parts.map((it, i) => `<div><span><b class="${cls(i, it)}"></b>${esc(it.label)}</span><strong>${((it.value / sum) * 100).toFixed(1)}%</strong><em>${esc(fmt(it.value, unit))}</em></div>`).join("")}</div>`;
  return frame(title, note, `<div class="pie"><svg viewBox="0 0 220 220" role="img" aria-label="${esc(title ?? "Pie chart")}">${slices}${centre}</svg>${keys}</div>`, dataTable(["", "Value", "Share"], parts.map((p) => [p.label, fmt(p.value, unit), `${((p.value / sum) * 100).toFixed(1)}%`])));
}

// Big numbers. A single number is a tile, never a one-bar chart.
export function tiles(list) {
  return `<div class="tiles">${list
    .map((t) => `<div class="card tile${t.tone === "warn" ? " warn" : ""}"><div class="k">${esc(t.label)}</div><div class="v">${esc(t.value)}</div>${t.note ? `<div class="k">${esc(t.note)}</div>` : ""}</div>`)
    .join("")}</div>`;
}

export function table(columns, rows, { title, limit = 50, units = {} } = {}) {
  const shown = rows.slice(0, limit);
  const body = `<div class="tablewrap"><table class="rows"><thead><tr>${columns.map((c) => `<th>${esc(c)}</th>`).join("")}</tr></thead><tbody>${shown
    .map((r) => `<tr>${columns.map((c) => `<td${typeof r[c] === "number" ? ' class="num"' : ""}>${esc(typeof r[c] === "number" ? fmt(r[c], units[c]) : r[c])}</td>`).join("")}</tr>`)
    .join("")}</tbody></table></div>${rows.length > limit ? `<p class="more">Showing ${limit} of ${rows.length} rows.</p>` : ""}`;
  return frame(title, null, body, "");
}

export const CHART_CSS = `
.s1{fill:var(--c1);stroke:var(--c1)}.s2{fill:var(--c2);stroke:var(--c2)}.s3{fill:var(--c3);stroke:var(--c3)}.s4{fill:var(--c4);stroke:var(--c4)}.s5{fill:var(--c5);stroke:var(--c5)}.s6{fill:var(--c6);stroke:var(--c6)}.sOther{fill:var(--other);stroke:var(--other)}
b.s1{background:var(--c1)}b.s2{background:var(--c2)}b.s3{background:var(--c3)}b.s4{background:var(--c4)}b.s5{background:var(--c5)}b.s6{background:var(--c6)}b.sOther{background:var(--other)}
rect[class^=s],path[class^=s]:not(.ln){stroke:none}
.ln{fill:none!important;stroke-width:2;stroke-linejoin:round;stroke-linecap:round}
.dot{stroke:var(--card)!important;stroke-width:2}
.chart{margin:0}
.chart figcaption{display:flex;justify-content:space-between;gap:10px;align-items:baseline;margin-bottom:8px;font-size:13px}
.chart figcaption span{color:var(--dim);font-size:11px}
.chart svg{display:block;width:100%;height:auto;overflow:visible}
.chart svg text{font:11px -apple-system,BlinkMacSystemFont,"SF Pro Text",system-ui,sans-serif}
.lab{fill:var(--text)}.val{fill:var(--text);font-weight:600}.tick{fill:var(--dim)}.big{fill:var(--text);font-size:20px!important;font-weight:600}
.grid{stroke:var(--line);stroke-width:1}.axis{stroke:var(--dim);stroke-width:1;opacity:.6}
.seg{border-radius:4px}
.legend{display:flex;flex-wrap:wrap;gap:4px 14px;color:var(--dim);font-size:11px;margin-top:8px}
.legend b,.parts b{display:inline-block;width:9px;height:9px;border-radius:2px;margin-right:6px;vertical-align:-1px}
.parts{display:grid;grid-template-columns:repeat(auto-fill,minmax(150px,1fr));gap:8px 14px;margin-top:10px;font-size:12px}
.parts div{display:grid;grid-template-columns:1fr auto;gap:0 8px}
.parts span{white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.parts em{grid-column:1/-1;font-style:normal;color:var(--dim);font-size:11px;padding-left:15px}
.pie{display:grid;grid-template-columns:200px 1fr;gap:18px;align-items:center}
.pieparts{grid-template-columns:1fr;margin-top:0}
.data{margin-top:10px;font-size:11px;color:var(--dim)}
.data summary{cursor:pointer}
.data table,.rows{width:100%;border-collapse:collapse;font-size:12px;margin-top:6px;color:var(--text)}
.data th,.data td,.rows th,.rows td{text-align:left;padding:4px 6px;border-bottom:1px solid var(--line)}
.data th,.rows th{color:var(--dim);font-weight:500}
td.num{text-align:right;font-variant-numeric:tabular-nums}
.tablewrap{overflow-x:auto}
@media (max-width:640px){.pie{grid-template-columns:1fr}.pie svg{max-width:220px;margin:0 auto}}
`;
