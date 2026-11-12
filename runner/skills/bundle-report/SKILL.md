---
name: bundle-report
description: Runs @convex-dev/bundle-analyzer on a Convex app and turns it into a Threadbear Print, a bundle size report with an isometric scale figure, size tiles, insights, the largest packages and import chains for duplicated packages. Updates the same print on later runs. Use when the user asks for a bundle report, bundle size print or page, "run bundle-analyzer summary", how big their Convex bundle is, or why a package is bundled twice, inside or outside a project.
---

# Bundle report

One recipe of the `make-print` skill, which covers charts, dashboards, reports and docs from any data, and how to refresh, edit, download and delete prints.

Makes a static page from the analyzer's JSON. Static means no scripts, so it opens anywhere a Print can. Every number on it comes from that JSON, so nothing on the page is drawn or written by hand.

`<this skill's folder>` in the commands below is the base directory you were given for this skill.

## Steps

1. **Find the Convex app.** Start with the folder that holds `convex/`.
   - In a project, check each of the project's folders (`read_project`). If more than one has `convex/`, ask which one. One question only.
   - Outside a project, use this thread's folder.
2. **Run the report.** One command runs the analyzer and builds the page:

   ```sh
   node <this skill's folder>/report.mjs <app dir> <out dir> --app "<app name>"
   ```

   - Use `$TMPDIR/bundle-report/<app>/page` for `<out dir>`. The JSON goes next to it in `page-json/`, never inside the page folder, because Prints refuse `.json` files.
   - The analyzer version is pinned to `@convex-dev/bundle-analyzer@0.1.0`.
   - Exit 3 means `convex/_generated` is missing. Ask the user before running `npx convex codegen`.
   - Exit 4 means the analyzer failed. Show its error.
   - It prints `summary: ...`. Use that line as the print's summary.
3. **Make or update the Print.** First call `list_prints` with `source: "bundle-analyzer summary"`.
   - If a print for this app already exists, call `update_print({ id, dir, summary })`.
   - Otherwise call `create_print({ title: "Bundle size · <app>", dir, summary, source: "bundle-analyzer summary", kind: "bundle", recipe: "node <this skill's folder>/report.mjs <app dir> <out dir> --app <app>" })`. The recipe is what the Refresh button follows later. Leave `projects` out so the print follows the thread's project. Pass `projects: []` only if the user said to keep it out of projects.
   - Pass `show: true` either way.
4. **Reply.** Put a ```` ```print <id> ```` block in your reply, then add a few lines on what matters. Lead with what's duplicated, then the biggest packages and the heaviest functions to load. The page shows the rest.

If the print tools aren't there, the Prints experiment is off. Pin `<out dir>/index.html` with `show: true` instead, and tell the user that Settings > General > Experiments > Prints keeps reports as versioned prints.

## What's on the page

The layout is the Split hero Wayne picked on 10 Nov 2026:

- **Figure.** An Anatomy platform scale, on the left.
  - Each crate is one of the 8 largest packages, and its height is that package's share of the bundle.
  - Duplicated packages carry an amber tag.
  - The dial reads the total against a round capacity.
- **Tiles**, on the right:
  - the total, with a bar for code, source maps and external packages
  - what's uploaded on each push
  - the packages bundled more than once
- **Below the hero:**
  - insights, and the largest packages with share bars
  - the heaviest functions to load
  - "Why they're bundled": a Pele import-chain diagram for each duplicated package, folded

It follows the viewer's light or dark appearance with CSS only.

## Files

| File | What it does |
|---|---|
| `report.mjs` | Runs the analyzer, saves the JSON, fetches chains for duplicates, then calls `build-page.mjs` |
| `build-page.mjs` | JSON in, `index.html` out. Can be run on its own with saved JSON |
| `figure.mjs` | The scale figure, drawn with the Anatomy kit |
| `fixtures/sample/summary.json` | A sample with duplicates and insights, made from the numbers in a screenshot, for checking the look |
| `test.mjs` | `node --test test.mjs` |

The skill needs the Anatomy and Pele skills from Threadbear's plugin. Point `ANATOMY_SKILL` and `PELE_SKILL` at them if they move. Without Pele, the page still builds, just without the chain diagrams.

## Changing the look

Run `node build-page.mjs fixtures/sample /tmp/br-sample`.

The Anatomy skill's `scripts/capture.mjs` checks the result. Pass `--theme dark` and then `--theme light`, at `--width 1180` and at `--width 390`, and look at every shot.

`scripts/lines.mjs` must report `line ends: 0`. A line that floats is a geometry fix in `figure.mjs`. Never use `free` to hide one.

Not built yet: a HyperFrames share clip (an MP4 of the numbers animating in). That needs Node 22 or later and FFmpeg, and should only run when the user asks for a clip.
