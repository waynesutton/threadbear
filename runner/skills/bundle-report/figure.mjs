// The bundle scale: an Anatomy figure of a platform scale weighing one crate
// per package. Crate height is the package's share of the bundle, the dial
// reads the total, and a package bundled more than once carries an amber tag.
// Every number comes from the analyzer's JSON, so a rebuild is always true.
import { pathToFileURL } from "node:url";
import { join } from "node:path";
import { homedir } from "node:os";

export const ANATOMY =
  process.env.ANATOMY_SKILL ?? join(homedir(), "Library/Application Support/Threadbear/skills/plugin/skills/anatomy");

const k = await import(pathToFileURL(join(ANATOMY, "kit/iso-kit.mjs")).href);
export const ISO_CSS = k.ISO_CSS;
export const { plateHtml } = k;

const WIDTH = 620;
const HEIGHT = 420;

const BASE = { x: -150, y: -84, w: 300, d: 168, r: 10 };
const BASE_H = 14;
const FOOT_H = 6;
const PAN = { x: -132, y: -70, w: 216, d: 140, r: 6 };
const PAN_Z = 12;
const PAN_H = 5;
const PAN_TOP = PAN_Z + PAN_H;
const CRATE = 40;
const GAP = 12;
const COLS = 4;
const MAX_CRATE_H = 92;
const MIN_CRATE_H = 7;
const POST = [118, -52];
const POST_TOP = 132;
const DIAL_R = 26;
const DIAL_H = 9;
const DIAL_Z = POST_TOP;
const DIAL_TOP = DIAL_Z + DIAL_H;
// The dial's sweep, as on a real scale: 300° from the zero mark, clockwise.
const SWEEP = (300 / 360) * Math.PI * 2;

const P = k.fitProjection(
  [...k.boxCorners(BASE, -BASE_H - FOOT_H, 0), ...k.boxCorners(PAN, PAN_TOP, PAN_TOP + MAX_CRATE_H), [POST[0], POST[1] - DIAL_R, DIAL_TOP], [POST[0] + DIAL_R, POST[1], DIAL_TOP]],
  WIDTH,
  HEIGHT,
  { pad: 26, azimuth: 40 },
);

const at = (p) => k.iso(p, P);
const S = (paths, style) => k.solidSvg(paths, style);
const L = (d, style) => k.lineSvg(d, style);
const D = (points, style) => k.dotsSvg(points, style);
const depth = ([x, y]) => k.depthOf([x, y, 0], P);
const byDepth = (items, of = (i) => i) => [...items].sort((a, b) => depth(of(a)) - depth(of(b)));

// A capacity the way a scale is labelled: the next round figure above the
// total, so the needle sits well inside the sweep.
export function capacityFor(bytes) {
  const mb = bytes / 1e6;
  for (const c of [0.5, 1, 2, 5, 10, 20, 25, 50, 100, 200, 500]) if (mb <= c * 0.92) return c;
  return Math.ceil(mb / 100) * 100;
}

export const mb = (bytes) => (bytes >= 1e6 ? `${(bytes / 1e6).toFixed(2)} MB` : `${(bytes / 1e3).toFixed(1)} KB`);

// The packages to weigh, biggest first, and which names appear in more than
// one version (or one the insights call duplicated).
export function cratesOf(summary, max = 8) {
  const pkgs = [...(summary.topPackages ?? [])].sort((a, b) => b.bytes - a.bytes).slice(0, max);
  const counts = new Map();
  for (const p of summary.topPackages ?? []) counts.set(p.name, (counts.get(p.name) ?? 0) + 1);
  const flagged = new Set((summary.insights ?? []).filter((i) => i.kind === "duplicate-package").map((i) => i.subject));
  const biggest = pkgs[0]?.bytes ?? 1;
  return pkgs.map((p, i) => ({
    name: p.name,
    label: p.version ? `${p.name}@${p.version}` : p.name,
    bytes: p.bytes,
    h: Math.max(MIN_CRATE_H, (p.bytes / biggest) * MAX_CRATE_H),
    dup: (counts.get(p.name) ?? 0) > 1 || flagged.has(p.name),
    plan: { x: PAN.x + 10 + (i % COLS) * (CRATE + GAP), y: PAN.y + 14 + Math.floor(i / COLS) * (CRATE + GAP + 8), w: CRATE, d: CRATE, r: 1.2 },
  }));
}

function base() {
  const out = [`<path class="iso-halo" d="${k.haloOf(BASE, -BASE_H - FOOT_H, BASE_H, P)}"/>`];
  for (const [x, y] of byDepth(k.corners(BASE, 24))) {
    out.push(S(k.cylinder(x, y, 8, -BASE_H - FOOT_H, 2, P, 28), { tone: "lo" }));
    out.push(S(k.cylinder(x, y, 6, -BASE_H - FOOT_H + 2, FOOT_H - 2, P, 28), { tone: "mid" }));
  }
  out.push(S(k.slabOf(BASE, -BASE_H, BASE_H, P, 10, 1.6), { tone: "mid", inner: L(k.sideSeam(BASE, -5, P, 10), { tone: "faint" }) }));
  out.push(L(k.planOutline(k.insetPlan(BASE, 5), 0, P, 10), { tone: "lo" }));
  const screws = k.corners(BASE, 11);
  out.push(L(screws.map(([x, y]) => k.ring(x, y, 2.4, 0, P, 16)).join(""), { tone: "lo" }));
  out.push(D(screws.map(([x, y]) => at([x, y, 0])), { size: 0.55 }));
  // A ruler strip along the front edge, in 10 px steps.
  const strip = { x: PAN.x, y: BASE.y + BASE.d - 11, w: PAN.w, d: 6, r: 1 };
  out.push(S(k.slabOf(strip, 0, 0.6, P, 3), { tone: "lo", crease: "none" }));
  const ticks = k.topTicks(strip.x + 4, strip.x + strip.w - 4, 4, 5, strip.y + 0.8, 0.6, [1.8, 3.4], P);
  out.push(L(ticks.minor, { tone: "lo" }), L(ticks.major, { tone: "mid" }));
  out.push(D([at([strip.x + 2, strip.y + 3, 0.6]), at([strip.x + strip.w - 2, strip.y + 3, 0.6])], { size: 0.45 }));
  return out.join("");
}

function pan() {
  const out = [];
  // Four load cells carry the pan.
  for (const [x, y] of byDepth(k.corners(PAN, 18))) {
    out.push(S(k.slabOf({ x: x - 7, y: y - 5, w: 14, d: 10, r: 1.2 }, 0, 4, P, 3, 0.4), { tone: "lo" }));
    out.push(S(k.cylinder(x, y, 3, 4, PAN_Z - 4, P, 20), { tone: "mid" }));
  }
  out.push(`<path class="iso-halo" d="${k.haloOf(k.insetPlan(PAN, 4), PAN_Z, 0.1, P)}" style="filter:blur(3px)"/>`);
  out.push(S(k.slabOf(PAN, PAN_Z, PAN_H, P, 8, 1.2), { tone: "mid" }));
  out.push(L(k.planOutline(k.insetPlan(PAN, 4), PAN_TOP, P, 8), { tone: "faint" }));
  // Anti-slip ribs across the pan, ending on the inset groove.
  const ribs = [];
  for (let y = PAN.y + 10; y < PAN.y + PAN.d - 6; y += 8) ribs.push(k.lineOnTop([PAN.x + 4, y], [PAN.x + PAN.w - 4, y], PAN_TOP, P));
  out.push(L(ribs.join(""), { tone: "faint" }));
  return out.join("");
}

function crate(c) {
  const out = [];
  const top = PAN_TOP + c.h;
  out.push(`<path class="iso-halo" d="${k.haloOf(k.insetPlan(c.plan, 2), PAN_TOP, 0.1, P)}" style="filter:blur(2px)"/>`);
  // Slat seams wrap the crate every 10 px, the way boards stack.
  const seams = [];
  for (let z = PAN_TOP + 10; z < top - 3; z += 10) seams.push(k.sideSeam(c.plan, z, P, 3));
  out.push(S(k.slabOf(c.plan, PAN_TOP, c.h, P, 3, 0.6), { tone: "hi", crease: "lo", inner: L(seams.join(""), { tone: "faint" }) }));
  // The lid: a frame and a cross brace.
  const lid = k.insetPlan(c.plan, 3);
  out.push(L(k.planOutline(lid, top, P, 3), { tone: "lo" }));
  out.push(L(k.lineOnTop([lid.x, lid.y], [lid.x + lid.w, lid.y + lid.d], top, P), { tone: "faint" }));
  out.push(D(k.corners(c.plan, 1.6).map(([x, y]) => at([x, y, top])), { size: 0.45, tone: "lo" }));
  // A stencilled name on the lid.
  const name = c.name.length > 11 ? `${c.name.slice(0, 10)}…` : c.name;
  out.push(k.faceTextSvg(k.topMatrix([c.plan.x + c.plan.w / 2, c.plan.y + c.plan.d / 2 + 1.2], top, P, "x"), name, { size: 5, tone: "mid", anchor: "middle" }));
  if (c.dup) {
    // An amber tag hung on the front corner: this one ships twice.
    const tag = { x: c.plan.x + c.plan.w - 12, y: c.plan.y + c.plan.d - 0.4, w: 9, d: 1.2, r: 0.4 };
    out.push(`<g class="br-dup">${S(k.slabOf(tag, top - 13, 9, P, 2), { tone: "hi", crease: "none" })}</g>`);
    out.push(D([at([tag.x + 4.5, tag.y + 1.2, top - 6])], { size: 0.5, tone: "lo" }));
  }
  return out.join("");
}

function scaleHead(total, capacity) {
  const out = [];
  const [x, y] = POST;
  // A square column up the back corner to the dial head.
  out.push(S(k.slabOf({ x: x - 9, y: y - 9, w: 18, d: 18, r: 2 }, 0, 6, P, 4, 0.5), { tone: "mid" }));
  out.push(D(k.corners({ x: x - 9, y: y - 9, w: 18, d: 18 }, 2.5).map(([a, b]) => at([a, b, 6])), { size: 0.45 }));
  out.push(S(k.slabOf({ x: x - 4.5, y: y - 4.5, w: 9, d: 9, r: 1.4 }, 6, DIAL_Z - 10, P, 3, 0.4), { tone: "mid", inner: L([40, 80].map((z) => k.sideSeam({ x: x - 4.5, y: y - 4.5, w: 9, d: 9, r: 1.4 }, z, P, 3)).join(""), { tone: "faint" }) }));
  out.push(S(k.cylinder(x, y, 7, DIAL_Z - 4, 4, P, 28), { tone: "mid" }));
  // The dial head: a deep bezel with a knurled ring and a face.
  out.push(S(k.cylinder(x, y, DIAL_R, DIAL_Z, DIAL_H, P, 72, 0.8), { tone: "hi", crease: "lo" }));
  out.push(L(k.knurl(x, y, DIAL_R, DIAL_TOP - 3.6, DIAL_TOP, 96, P, 0.3), { tone: "lo" }));
  out.push(L(k.sideArc(x, y, DIAL_R, DIAL_TOP - 3.6, P, 48), { tone: "faint" }));
  out.push(L(k.ring(x, y, DIAL_R - 2.4, DIAL_TOP, P, 72), { tone: "lo" }));
  const ticks = k.radialTicks(x, y, DIAL_R - 3.4, 50, 5, DIAL_TOP, [1.4, 3], P, 0, SWEEP);
  out.push(L(ticks.minor, { tone: "lo" }), L(ticks.major, { tone: "mid" }));
  out.push(k.faceTextSvg(k.topMatrix([x, y + 10], DIAL_TOP, P, "x"), `0–${capacity} MB`, { size: 2.6, tone: "lo", anchor: "middle" }));
  // The needle: the one lit part. The total over the capacity, across the sweep.
  const a = Math.min(1, total / 1e6 / capacity) * SWEEP;
  const tip = [x + Math.sin(a) * (DIAL_R - 5), y - Math.cos(a) * (DIAL_R - 5)];
  const tail = [x - Math.sin(a) * 4, y + Math.cos(a) * 4];
  out.push(`<path class="iso-line" data-tone="lit" data-free="start" d="${k.lineOnTop(tail, tip, DIAL_TOP + 0.2, P)}" style="stroke-width:1.2"/>`);
  out.push(L(k.ring(x, y, 1.8, DIAL_TOP + 0.2, P, 16), { tone: "mid" }));
  out.push(D([at([x, y, DIAL_TOP + 0.2])], { size: 0.7, tone: "hi" }));
  return out.join("");
}

// The figure as an <svg>, for the page's hero.
export function scaleSvg(summary) {
  const total = summary.totals?.bytes ?? 0;
  const capacity = capacityFor(total);
  const crates = cratesOf(summary);
  const head = { at: POST, svg: () => scaleHead(total, capacity) };
  const items = [...crates.map((c) => ({ at: [c.plan.x + c.plan.w / 2, c.plan.y + c.plan.d / 2], svg: () => crate(c) })), head];
  const dups = crates.filter((c) => c.dup).map((c) => c.name);
  return k.figureSvg({
    width: WIDTH,
    height: HEIGHT,
    label: `A platform scale weighing the Convex bundle. Each crate on the pan is one package, as tall as its share of the bundle: ${crates.map((c) => `${c.label} ${mb(c.bytes)}`).join(", ")}. The dial reads ${mb(total)} on a ${capacity} MB scale.${dups.length ? ` Amber tags mark packages bundled more than once: ${[...new Set(dups)].join(", ")}.` : ""}`,
    body: [base(), pan(), ...byDepth(items, (i) => i.at).map((i) => i.svg())],
  });
}

// The kit's light theme as a media query, so a static page follows the
// viewer's appearance with no script.
export function autoThemeCss() {
  const m = ISO_CSS.match(/\.iso\[data-theme="light"\]\{([^}]*)\}/);
  return m ? `@media (prefers-color-scheme: light){.iso:not([data-theme="dark"]){${m[1]}}}` : "";
}
