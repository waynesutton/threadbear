// The Prints look (redesigned 18 Nov 2026, Wayne: "make it look like a
// product, not vibe coded"): one neutral base per theme, hairline borders,
// 12px cards, generous room, numbers before pictures and one accent per
// chart. Dark is near-black (#08090a) with quiet white-alpha lines, as
// Linear and Resend are; light is warm paper (#f4f3ef) with white cards.
// Big titles get a soft grey-to-white (or ink-to-grey) gradient and tight
// tracking. The system's own type stands in for Inter: no font to load, as
// a print can't fetch one. The same base as the runner's doc pages
// (runner/src/page-doc.ts, PAGE_THEME_CSS), so every kind sits together.
//
// Tokens follow Wayne's chart components page: accent #4F5BFF, positive
// #16B57F, highlight #FF7A45, warning #F2B53A, ink #111113, brighter steps
// of each in dark so they hold their contrast on #0f1011. The six series
// slots keep the old rule of no violet: accent, sky, positive, highlight,
// warning, rose, then a grey "Other". Every chart still ships direct labels
// and a "Show the data" table, so colour is never the only key.
//
// Light or dark follows the viewer (prefers-color-scheme), unless the
// user's style forces one, and a viewer can force either with
// `data-theme="light|dark"` on <html> (the website's Light, Dark and System
// switch, web/src/print-doc.ts).
import { CHART_CSS } from "./charts.mjs";

const DARK = `color-scheme:dark;--bg:#08090a;--card:#0f1011;--raise:#151618;--line:#1d1e21;--line2:#26272b;--text:#f7f8f8;--dim:#8a8f98;--faint:#62666d;
--h1a:#ffffff;--h1b:#9a9ca3;--invert:#14162b;--invtext:#f7f8f8;--grid:#2a2c31;
--teal:#7b84ff;--amber:#f5c15a;--amberline:#3d3320;--good:#2fd39a;--goodbg:rgba(47,211,154,.12);--bad:#ff6b70;--badbg:rgba(255,107,112,.12);--warnbg:rgba(245,193,90,.12);
--c1:#7b84ff;--c2:#4cb4f0;--c3:#2fd39a;--c4:#ff8a5c;--c5:#f5c15a;--c6:#ef6f9f;--other:#4a4d55;--shadow:0 0 0 1px rgba(255,255,255,.02)`;
const LIGHT = `color-scheme:light;--bg:#f4f3ef;--card:#ffffff;--raise:#faf9f6;--line:#e7e5df;--line2:#dcd9d1;--text:#111113;--dim:#5e5d58;--faint:#8e8c85;
--h1a:#111113;--h1b:#55555c;--invert:#111113;--invtext:#f7f8f8;--grid:#e3e1da;
--teal:#4f5bff;--amber:#b7811a;--amberline:#efdcae;--good:#0f9a6a;--goodbg:rgba(22,181,127,.12);--bad:#d63c41;--badbg:rgba(229,72,77,.1);--warnbg:rgba(242,181,58,.16);
--c1:#4f5bff;--c2:#1f8fd6;--c3:#16b57f;--c4:#ff7a45;--c5:#e5a52a;--c6:#de4f88;--other:#c9c6bd;--shadow:0 1px 2px rgba(17,17,19,.04)`;
const DENSITY = {
  dense: "--pad:14px;--gap:10px;--fs:13px",
  normal: "--pad:20px;--gap:16px;--fs:14px",
  airy: "--pad:26px;--gap:20px;--fs:15px",
};
const HEX = /^#[0-9a-f]{6}$/i;

// The user's saved style (make-print's style.json, asked once): theme
// "auto" (follows the viewer), "dark" or "light"; density "dense",
// "normal" or "airy"; and an accent for progress, links and highlights.
// "teal" (an old answer) is each theme's own accent.
export function themeCss(style = {}) {
  const density = DENSITY[style.density] ?? DENSITY.normal;
  const accent = HEX.test(style.accent ?? "") ? `--accent:${style.accent}` : "--accent:var(--teal)";
  const root = style.theme === "light" ? LIGHT : DARK;
  const media = style.theme === "dark" || style.theme === "light" ? "" : `@media (prefers-color-scheme: light){:root{${LIGHT}}}`;
  return `:root{${root};${density};${accent}}${media}:root[data-theme=light]{${LIGHT}}:root[data-theme=dark]{${DARK}}
`;
}

const SANS = `"Inter var",Inter,-apple-system,BlinkMacSystemFont,"SF Pro Text","Segoe UI",system-ui,sans-serif`;
const DISPLAY = `"Inter var",Inter,-apple-system,BlinkMacSystemFont,"SF Pro Display","Segoe UI",system-ui,sans-serif`;
export const MONO = `ui-monospace,"SF Mono","JetBrains Mono",Menlo,Consolas,monospace`;

export const THEME_CSS = themeCss() + `*{box-sizing:border-box}
html{background:var(--bg)}
body{margin:0;background:var(--bg);color:var(--text);font:var(--fs)/1.55 ${SANS};font-feature-settings:"cv11","ss01";-webkit-font-smoothing:antialiased;text-rendering:optimizeLegibility}
main{max-width:1120px;margin:0 auto;padding:56px 28px 64px}
main.narrow{max-width:880px}
header.top{margin-bottom:32px}
.eyebrow{font:500 11.5px/1 ${SANS};letter-spacing:.08em;text-transform:uppercase;color:var(--faint);margin:0 0 14px;display:flex;gap:10px;flex-wrap:wrap}
.eyebrow span+span::before{content:"";display:inline-block;width:3px;height:3px;border-radius:50%;background:currentColor;margin-right:10px;vertical-align:middle;opacity:.7}
h1,h2,h3{font-family:${DISPLAY};color:var(--text)}
h1{font-size:40px;line-height:1.08;font-weight:600;margin:0;letter-spacing:-.032em;background:linear-gradient(180deg,var(--h1a) 30%,var(--h1b));-webkit-background-clip:text;background-clip:text;-webkit-text-fill-color:transparent;text-wrap:balance}
.lede{color:var(--dim);font-size:16px;line-height:1.55;margin:12px 0 0;max-width:640px;text-wrap:pretty}
h2{font-size:15px;line-height:1.3;font-weight:600;margin:0 0 4px;letter-spacing:-.011em}
h3{font-size:13px;font-weight:600;margin:16px 0 6px}
.dim,.sub{color:var(--dim)}
.card{background:var(--card);border:1px solid var(--line);border-radius:12px;padding:var(--pad);box-shadow:var(--shadow);min-width:0}
.tiles{display:grid;grid-template-columns:repeat(auto-fit,minmax(190px,1fr));gap:var(--gap)}
.tile{display:flex;flex-direction:column;gap:6px;min-height:118px}
.tile .k{color:var(--dim);font-size:12.5px;font-weight:500}
.tile .v{font:650 32px/1.05 ${DISPLAY};letter-spacing:-.025em;font-variant-numeric:tabular-nums;color:var(--text);margin-top:2px}
.tile .row{display:flex;align-items:center;gap:8px;margin-top:auto;min-height:22px}
.tile .n{color:var(--faint);font-size:12px}
.tile.warn .v{color:var(--amber)}.tile.bad .v{color:var(--bad)}.tile.good .v{color:var(--good)}
.tile .spark{flex:1;min-width:0;height:26px}
.pill{display:inline-flex;align-items:center;gap:3px;height:20px;padding:0 7px;border-radius:999px;font:600 11.5px/1 ${SANS};font-variant-numeric:tabular-nums;white-space:nowrap;background:var(--raise);color:var(--dim);border:1px solid var(--line)}
.pill.up{background:var(--goodbg);color:var(--good);border-color:transparent}
.pill.down{background:var(--badbg);color:var(--bad);border-color:transparent}
.pill.warn{background:var(--warnbg);color:var(--amber);border-color:transparent}
.grid2{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:var(--gap)}
.stack>*+*{margin-top:16px}
.prose{max-width:68ch}
.prose p{margin:0 0 12px;font-size:15px;line-height:1.7;color:var(--text)}
.prose ul,.prose ol{margin:0 0 12px;padding-left:20px;font-size:15px;line-height:1.65}
.prose li{margin:4px 0}.prose li::marker{color:var(--faint)}
.prose li.task{list-style:none;margin-left:-20px;display:flex;gap:10px;align-items:baseline}
.prose li.task .box{flex:none;width:14px;height:14px;border:1.5px solid var(--line2);border-radius:4px;transform:translateY(2px)}
.prose li.task.done .box{background:var(--accent);border-color:var(--accent)}
.prose li.task.done{color:var(--dim)}
.prose a{color:var(--text);text-decoration:underline;text-decoration-color:var(--line2);text-decoration-thickness:1px;text-underline-offset:3px}
.prose code{font:12.5px ${MONO};background:var(--raise);border:1px solid var(--line);border-radius:5px;padding:1px 5px}
.prose strong{font-weight:600}
.more{color:var(--faint);font-size:12px;margin:10px 0 0}
.figure svg{display:block;width:100%;height:auto}
footer{color:var(--faint);font-size:12px;margin-top:40px;padding-top:16px;border-top:1px solid var(--line);display:flex;justify-content:space-between;gap:12px;flex-wrap:wrap;font-variant-numeric:tabular-nums}
main>*+*{margin-top:var(--gap)}main>header.top+*{margin-top:0}
.report{display:grid;grid-template-columns:180px minmax(0,1fr);gap:48px;align-items:start}
.toc{position:sticky;top:32px;font-size:12.5px;color:var(--dim)}
.toc b{display:block;font:500 11px/1 ${SANS};letter-spacing:.08em;text-transform:uppercase;color:var(--faint);margin-bottom:12px}
.toc ol{list-style:none;margin:0;padding:0;display:grid;gap:9px}
.toc li{display:flex;gap:10px}.toc i{font:500 11px/1.6 ${MONO};font-style:normal;color:var(--faint)}
.report .body>*+*{margin-top:var(--gap)}
.sec{padding-top:8px}
.sec+.sec{margin-top:40px!important}
.sec-head{display:flex;align-items:baseline;gap:14px;margin-bottom:14px}
.sec-head i{font:500 12px/1 ${MONO};font-style:normal;color:var(--accent)}
.sec-head h2{font-size:22px;letter-spacing:-.02em;margin:0}
.callout{background:var(--invert);color:var(--invtext);border-radius:12px;padding:24px var(--pad);display:grid;gap:6px}
.callout .k{font:500 11px/1 ${SANS};letter-spacing:.08em;text-transform:uppercase;opacity:.6}
.callout .v{font:650 44px/1 ${DISPLAY};letter-spacing:-.03em;font-variant-numeric:tabular-nums}
.callout .n{font-size:14px;opacity:.75;max-width:60ch}
@media (max-width:860px){.report{grid-template-columns:1fr}.toc{display:none}}
@media (max-width:700px){.grid2{grid-template-columns:1fr}main{padding:32px 18px 44px}h1{font-size:30px}.tile .v{font-size:28px}}
${CHART_CSS}`;

// The same rules without the palette, for pages with a saved style.
export const BASE_CSS = THEME_CSS.slice(themeCss().length);
