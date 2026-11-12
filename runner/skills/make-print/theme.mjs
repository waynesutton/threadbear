// The Prints look: neutral near-black and near-white, quiet grey lines and
// the system's own type, no font to load (Wayne, 12 Nov 2026). One palette,
// cards and type for every kind, so a chart, a dashboard, a report and a doc
// sit together as one set. The same base as the runner's doc pages
// (runner/src/page-doc.ts, PAGE_THEME_CSS) and the bundle report.
//
// Chart colours are the dataviz skill's validated categorical slots
// without violet (no purple), six in order, checked with its
// validate_palette.js on 10 Nov 2026 against cards of #fbfaf7 and #12161b,
// near the neutral #f7f7f7 and #141414 used since 12 Nov (contrast warns in
// light, so every chart ships labels and a table).
import { CHART_CSS } from "./charts.mjs";

const DARK = `color-scheme:dark;--bg:#0a0a0a;--card:#141414;--line:#262626;--text:#fafafa;--dim:#a1a1a1;--faint:#737373;--teal:#4fb3d9;--amber:#e8a33d;--amberline:#3a2e1a;--good:#3fb950;--bad:#ff6b6b;
--c1:#3987e5;--c2:#d95926;--c3:#199e70;--c4:#c98500;--c5:#d55181;--c6:#008300;--other:#5b6470`;
const LIGHT = `color-scheme:light;--bg:#ffffff;--card:#f7f7f7;--line:#e7e7e7;--text:#0a0a0a;--dim:#5c5c5c;--faint:#8a8a8a;--teal:#2f7fa0;--amber:#d39634;--amberline:#ecd6ad;--good:#2f8a4f;--bad:#d1393e;
--c1:#2a78d6;--c2:#eb6834;--c3:#1baf7a;--c4:#eda100;--c5:#e87ba4;--c6:#008300;--other:#a3a3a3`;
const DENSITY = {
  dense: "--pad:10px;--gap:8px;--fs:12.5px",
  normal: "--pad:16px;--gap:14px;--fs:14px",
  airy: "--pad:22px;--gap:18px;--fs:15px",
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
body{margin:0;background:var(--bg);color:var(--text);font:var(--fs)/1.6 -apple-system,BlinkMacSystemFont,"SF Pro Text","Segoe UI",system-ui,sans-serif;-webkit-font-smoothing:antialiased;text-rendering:optimizeLegibility}
main{max-width:1080px;margin:0 auto;padding:40px 24px 56px}
main.narrow{max-width:820px}
header{display:flex;justify-content:space-between;align-items:baseline;gap:12px;flex-wrap:wrap;margin-bottom:24px}
h1,h2{font-family:-apple-system,BlinkMacSystemFont,"SF Pro Display","Segoe UI",system-ui,sans-serif;font-weight:600}
h1{font-size:32px;line-height:1.15;margin:0;letter-spacing:-.025em}
h2{font-size:17px;margin:28px 0 10px;letter-spacing:-.01em}
.dim,.sub{color:var(--dim)}
.card{background:var(--card);border:1px solid var(--line);border-radius:10px;padding:var(--pad)}
.tiles{display:grid;grid-template-columns:repeat(auto-fit,minmax(170px,1fr));gap:var(--gap);margin-bottom:var(--gap)}
.tile .k{color:var(--dim);font-size:12px}
.tile .v{font-size:28px;font-weight:600;letter-spacing:-.02em;font-variant-numeric:tabular-nums;margin-top:2px}
.tile.warn{border-color:var(--amberline)}.tile.warn .v{color:var(--amber)}
.grid2{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:var(--gap)}
.stack>*+*{margin-top:14px}
.prose p{margin:0 0 12px;font-size:15px;line-height:1.7}
.prose ul,.prose ol{margin:0 0 12px;padding-left:20px;font-size:15px}
.prose a{color:var(--text);text-decoration:underline dotted;text-decoration-thickness:1.5px;text-underline-offset:4px}
.prose code{font:13px ui-monospace,"SF Mono",Menlo,Consolas,monospace}
.more{color:var(--faint);font-size:12px;margin:8px 0 0}
.figure svg{display:block;width:100%;height:auto}
footer{color:var(--faint);font-size:12px;margin-top:24px;display:flex;justify-content:space-between;gap:12px;flex-wrap:wrap}
@media (max-width:700px){.grid2{grid-template-columns:1fr}main{padding:24px 16px 40px}h1{font-size:26px}}
${CHART_CSS}`;

// The same rules without the palette, for pages with a saved style.
export const BASE_CSS = THEME_CSS.slice(themeCss().length);
