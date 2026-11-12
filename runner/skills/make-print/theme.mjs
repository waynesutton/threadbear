// The Prints look (the Split hero, Wayne, 10 Nov 2026): one palette, cards
// and type for every kind, so a chart, a dashboard, a report and a doc sit
// together as one set. The same base as the runner's doc pages
// (runner/src/page-doc.ts, PAGE_THEME_CSS) and the bundle report.
//
// Chart colours are the dataviz skill's validated categorical slots
// without violet (no purple), six in order, checked with its
// validate_palette.js on 10 Nov 2026: light on #fbfaf7 passes (contrast
// warns, so every chart ships labels and a table); dark on #12161b passes.
import { CHART_CSS } from "./charts.mjs";

const DARK = `color-scheme:dark;--bg:#0b0d10;--card:#12161b;--line:#20262e;--text:#e6e8eb;--dim:#7d8590;--teal:#4fb3d9;--amber:#e8a33d;--amberline:#3a2e1a;--good:#3fb950;--bad:#ff6b6b;
--c1:#3987e5;--c2:#d95926;--c3:#199e70;--c4:#c98500;--c5:#d55181;--c6:#008300;--other:#5b6470`;
const LIGHT = `color-scheme:light;--bg:#f5f3ee;--card:#fbfaf7;--line:#e2ded5;--text:#1d1f22;--dim:#7a766e;--teal:#3d86a3;--amber:#d39634;--amberline:#ecd6ad;--good:#2f8a4f;--bad:#d1393e;
--c1:#2a78d6;--c2:#eb6834;--c3:#1baf7a;--c4:#eda100;--c5:#e87ba4;--c6:#008300;--other:#a8a39a`;
const DENSITY = {
  dense: "--pad:10px;--gap:8px;--fs:12.5px",
  normal: "--pad:14px;--gap:12px;--fs:13px",
  airy: "--pad:20px;--gap:18px;--fs:14px",
};
const HEX = /^#[0-9a-f]{6}$/i;

// The user's saved style (make-print's style.json, asked once): theme
// "auto" (follows the viewer), "dark" or "light"; density "dense",
// "normal" or "airy"; and an accent for progress, links and highlights.
export function themeCss(style = {}) {
  const density = DENSITY[style.density] ?? DENSITY.normal;
  const accent = HEX.test(style.accent ?? "") ? `--accent:${style.accent}` : "--accent:var(--teal)";
  const root = style.theme === "light" ? LIGHT : DARK;
  const media = style.theme === "dark" || style.theme === "light" ? "" : `@media (prefers-color-scheme: light){:root{${LIGHT}}}`;
  return `:root{${root};${density};${accent}}${media}
`;
}

export const THEME_CSS = themeCss() + `*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--text);font:var(--fs)/1.5 -apple-system,BlinkMacSystemFont,"SF Pro Text",system-ui,sans-serif;-webkit-font-smoothing:antialiased}
main{max-width:1080px;margin:0 auto;padding:22px}
main.narrow{max-width:820px}
header{display:flex;justify-content:space-between;align-items:baseline;gap:12px;flex-wrap:wrap;margin-bottom:14px}
h1{font-size:20px;margin:0;letter-spacing:-.01em}
h2{font-size:15px;margin:22px 0 8px}
.dim,.sub{color:var(--dim)}
.card{background:var(--card);border:1px solid var(--line);border-radius:12px;padding:var(--pad)}
.tiles{display:grid;grid-template-columns:repeat(auto-fit,minmax(170px,1fr));gap:var(--gap);margin-bottom:var(--gap)}
.tile .k{color:var(--dim);font-size:11px}
.tile .v{font-size:24px;font-weight:600;letter-spacing:-.01em;font-variant-numeric:tabular-nums}
.tile.warn{border-color:var(--amberline)}.tile.warn .v{color:var(--amber)}
.grid2{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:var(--gap)}
.stack>*+*{margin-top:12px}
.prose p{margin:0 0 10px;font-size:14px;line-height:1.6}
.prose ul,.prose ol{margin:0 0 10px;padding-left:20px;font-size:14px}
.prose a{color:var(--teal)}
.prose code{font:12px ui-monospace,"SF Mono",Menlo,monospace}
.more{color:var(--dim);font-size:11px;margin:8px 0 0}
.figure svg{display:block;width:100%;height:auto}
footer{color:var(--dim);font-size:11px;margin-top:16px;display:flex;justify-content:space-between;gap:12px;flex-wrap:wrap}
@media (max-width:700px){.grid2{grid-template-columns:1fr}main{padding:14px}}
${CHART_CSS}`;

// The same rules without the palette, for pages with a saved style.
export const BASE_CSS = THEME_CSS.slice(themeCss().length);
