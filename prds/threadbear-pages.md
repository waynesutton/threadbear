# Threadbear Pages, with a bundle size report as the first use

## Context

Wayne wants to type something like "run bundle-analyzer summary and make a page" in a thread, and get a page he can open again later. It should look like the terminal screenshot: a size bar, insights and a table of the largest packages. If the thread is in a project, the page should show up on that project's page too. With no project, it stays with the thread.

Threadbear has no artifact or page concept today. The closest things are Highlights pins, the side pane file preview and html fences. Fence ids are a hash of the source, so a fence can't be updated in place. The side pane doesn't reload when a file changes. The web app shows html fences as plain code.

`@convex-dev/bundle-analyzer` (0.1.0, Node 20+) has `summary`, `insights`, `components`, `packages`, `files`, `modules` and `explore`. Every command except `explore` takes `--json`. It needs `convex/_generated/` and installed dependencies. It writes no HTML, so the page has to be built from the JSON.

This is for Wayne's fork only. Nothing goes to mikecann/threadbear without asking him first.

## Approach

There are two layers. The first is a general **Pages** feature: a stored, named, versioned page folder. The second is a **bundle report recipe**, which is a skill plus a project chip. The recipe uses Pages. The runner gets no bundle-analyzer code, so Mike's core stays generic and the recipe can change without a runner release.

The page doesn't need to copy the terminal. It needs to look great, so it's built from the skills Wayne already has:

| Layer | Tool | What it adds | Scripts? |
|---|---|---|---|
| Base page | Hand-built HTML and CSS template in the skill | Hero numbers, a stacked size bar, an insights list, a packages table with share bars, light and dark themes | No |
| Hero figure | **Anatomy** | An isometric machine whose readouts come from the JSON. Suggested object: a freight scale weighing crates, one per package. The bars on the crates come from `share`, and the dial reads total MB. Duplicate packages like `iconv-lite` show as twin crates with a warning tag. | Static SVG by default. Optional live motion |
| Import chains | **Pele** | `packages <name> --json` gives entry points and import chains. Pele draws them as a `flowchart LR` SVG, which explains why `zod` ends up in both runtimes. | No |
| Share clip | **HyperFrames** (optional) | A 15 to 30 second MP4 of the page's numbers animating in, for an X or LinkedIn post. It's HTML-native, so it reuses the page's markup. Apache 2.0. Needs Node 22+ and FFmpeg. | Builds offline. The page embeds the MP4 |

Remotion stays off the default path. It's a separate React project, and its license may require a paid company license for bigger teams. That needs checking before Wayne uses it at Convex. HyperFrames does the same job using the page's own HTML.

**The rule that makes this repeatable:** the Anatomy figure is designed once and saved as a script, `build.mjs`, in the skill folder. That script takes the analyzer JSON as input. Each time the report is refreshed, the figure is rebuilt in seconds with no new drawing work. The figure is a Hero (80 to 150 parts, about 2 to 4 hours, done once). Its numbers stay true because they come straight from the JSON, which is Anatomy's "truth first" rule.

Everything ships behind a new experiment, `pages`, which is off by default.

### 1. Runner store: `runner/src/pages.ts` (new)

- Model it on `runner/src/projects.ts`: the `Projects` class, `atomicWriteFileSync`, the `frozen` drain guard and `projects.updated`.
- Layout:
  - `<home>/pages/index.json` holds the records.
  - Each version is saved as a folder, `<home>/pages/<id>/v<n>/`. It contains `index.html` plus assets: `.svg`, `.png`, `.webp`, `.mp4` and `.css`.
  - The runner copies the files in. Symlinks, `.json` and `.js` files are refused, which matches `LocalPreviewSite.swift`'s rules.
  - Keep the last 10 versions.
- `PageRecord` fields:
  - `id`, `title` and `summary` (up to 280 characters)
  - `ownerThread`
  - `projects: {id, by, pinned?}[]`, which can be empty
  - `version`, `bytes`, `createdAt`, `updatedAt`
  - `source?` (for example `"bundle-analyzer summary"`)
- Size caps: 1 MB for the HTML and 25 MB for the whole folder, which leaves room for one MP4 clip.
- Each page has a `live` flag, false by default.
  - A page with `live: false` that contains `<script` is rejected.
  - A page with `live: true` keeps its inline scripts, used for Anatomy motion.
  - Live scripts run only in the side pane, through the existing "Run them" consent that is tied to the file's digest (`PreviewScriptConsent.swift`). Each new version asks again, as it does today.
  - Even live pages get no network access. Thread cards always show the static page.
- When a page is created with no `projects` given:
  - If the owner thread is pinned to exactly one project, the page joins that project.
  - Otherwise it gets `[]`.
  - Passing `projects: []` explicitly always means no project.

### 2. Agent tools: `runner/src/threadbear-mcp.ts`

- Add a `pageTools` array alongside `projectTools` (:1057). Spread it in at :1583 only when the `pages` experiment is on. Pass `() => experimentOn(cfg.home,'pages')` through both call sites in `main.ts` (:226 and :239).
- The tools:
  - `create_page({title, dir | html, summary, projects?, source?, live?})` returns the id and version. `dir` is a built folder; `html` is a single page.
  - `update_page({id, dir? | html?, title?, summary?})` bumps the version.
  - `list_pages({project?, thread?, none?})`
  - `read_page({id})` returns metadata, the summary and the file path.
  - `add_page_to_project` and `remove_page_from_project`
- Approval rules:
  - `list_pages` and `read_page` go in `NEVER_ASK_TOOLS` (:131).
  - `create_page` asks like other tools, except in Full access.
  - `update_page` from the page's `ownerThread` goes through without asking. That way a refresh is a single approval, not one per update.
- Calling `create_page` or `update_page` with `show: true` reuses the pin path (`runner/src/runner/highlights.ts:43`). It pins the latest version file and opens the side pane.

### 3. Protocol: `docs/protocol.md` first

- Add `pages.list`, `pages.get` and `pages.setProjects`, and a `pages.updated` notification. They go next to `projects.*` (:1011).
- Add an optional `pages?: {id,title,version}[]` field to project payloads, so the app can list them.

### 4. Mac app

- Add `PagesSection(project:)` to `app/Sources/Threadbear/Views/ProjectPage.swift`, between `ThreadsSection` and `PatchesSection`. Model it on `TasksSection` (:293-341), which has a count, a refresh button and loads on appear. Each row shows the title, version, updated time and summary. Clicking a row opens it in the side pane through the existing `LocalPreviewSchemeHandler`.
- Add a page fence. A reply can contain:

  ````
  ```page a_bundle_report
  ```
  ````

  `RichBlock.from` (:31) learns a `page` kind whose id is the page id, not a hash. The card shows the live title, version and summary from `pages.get`, plus an Open button. A newer version updates that same card instead of adding another one.
- Fix the side pane reload. When `pages.updated` arrives for the open page, bump `opened` (`PaneModel.swift:206`). This closes the "no file watcher" gap for pages without adding a general file watcher.
- Pages with no project are listed in a "Pages" row in the thread's Highlights. A global Pages view can come later.

### 5. The bundle report recipe

- **Skill**: `bundle-report` in Wayne's skills. It tells the agent to:
  1. Find the Convex app folder. In a project, check each of the project's folders for `convex/`. Outside a project, use the thread's folder. If there's more than one, ask which.
  2. Check that `convex/_generated/` exists. If it doesn't, ask before running `npx convex codegen`.
  3. Run `npx @convex-dev/bundle-analyzer summary --json` there, plus `insights --json` and `packages --json --limit 20`. Save the output to a scratch folder.
  4. Run `node build-page.mjs <json folder> <out folder>` from the skill folder. It does three things:
     - fills `template.html`
     - calls the Anatomy figure script `figure/build.mjs`, using `figureSvg({ standalone })` for the static SVG and the live version when `--live` is passed
     - renders the import chain for the top two packages, plus every duplicated package, through Pele's `render.mjs`
  5. Optional, only if asked for a clip: run `npx hyperframes render` on `clip/index.html`, which reads the same numbers, and drop `clip.mp4` into the folder.
  6. Call `update_page` if a page with source `bundle-analyzer summary` already exists for this project or folder. Otherwise call `create_page`. Then post the `page` fence.
- **Look, decided 2026-11-10: version 3, "Split hero" (C+A).**
  - Top row: the Anatomy scale on the left, about 60% of the width. On the right are three tiles: the total with a small stacked bar under it, the size uploaded each push, and the duplicates count with the package names. Under those goes a meta line: convex version, components, modules, page version.
  - Second row: insights on the left, the largest packages with share bars on the right.
  - Below that: Pele import chains, folded by default, then the HyperFrames clip if there is one.
  - Under 600 px the figure goes on top and the tiles stack under it.
  - Dark palette: background #0b0d10, cards #12161b, borders #20262e, teal #4fb3d9, slate #5f7a94, amber #e8a33d. Light palette: paper #f5f3ee, cards #fbfaf7, borders #e2ded5, teal #3d86a3, slate #97a8b8, amber #d39634. No purple.
- **Superseded:** before this was decided, the plan said to design the look with Wayne first. Put two or three directions in front of him with `show_options` (dense dashboard, editorial report, figure-led) as stills, and build the template he picks. Use `interface-design` and `baseline-ui` for the template pass.
- Anatomy's verification still applies to the figure, once, when it's built: `--audit` passes, `lines.mjs` reports zero, and 4× captures in both themes.
- **Project chip**: an optional project patch, `.threadbear/patches/bundle.ts`:

  ```ts
  actions: [{ title: "Bundle report", prompt: "Run the bundle-report skill and update the Bundle size page." }]
  ```

  It is written with Write, per patch trust rules.
- The template sits in the skill folder as `template.html`, and the agent fills it from the JSON. Keeping the layout fixed is what makes v3 look like v1.

### 6. Out of this plan

These are later phases from the PRD:
- Website rendering
- iPhone page cards
- Convex sync and publishing
- Grok `list_artifacts`
- Live data over postMessage

Page HTML is never mirrored to Convex in this plan.

## Files

| Area | Files |
|---|---|
| Runner | `runner/src/pages.ts` (new), `runner/src/threadbear-mcp.ts`, `runner/src/main.ts`, `runner/src/experiments/pages.md` (new), `runner/src/runner/highlights.ts` (reuse) |
| Protocol | `docs/protocol.md`, `docs/guide.md`, `docs/on-your-mac.md` (new `<home>/pages/` folder) |
| Mac | `ThreadbearCore/Markdown/RichBlock.swift`, `Views/ProjectPage.swift`, `Views/PaneModel.swift`, Transcript `RichBlocks.swift` |
| Recipe | skill `bundle-report/` (`SKILL.md`, `template.html`, `build-page.mjs`, `figure/build.mjs` on the Anatomy kit, `clip/index.html` for HyperFrames), optional project patch |
| Tracking | `prds/threadbear-pages.md`, `task.md` |

## Verification

- **Runner unit test**, `runner/test/unit/pages.test.ts`:
  - create, update, version prune
  - the `<script` reject and the size cap
  - project inheritance with one pinned project, none, and explicit `[]`
  - `list_pages` filters
  - Run it with `node --test test/unit/pages.test.ts`.
- **Experiment test**: `experiments.test.ts` passes, which means the experiment has a reminder id. Run `npm run experiment-docs`.
- **Swift**: `swift test --filter RichBlockTests` covers the `page` fence parse and the stable id.
- **Whole suite**: run `./check.sh --quick`. Set `THREADBEAR_NODE` first, and read only the log folder this run prints.
- **End to end** in a thread in the Threadbear project, against a Convex repo folder:
  1. Ask "run bundle-analyzer summary and make a page". You should see one card, the page on the project page, and the side pane open.
  2. Ask it again. The same card should go to v2 with no new card, and the open pane should reload.
  3. In a thread with no project, ask the same thing. The page should show only in that thread's Highlights.
- **Assets in the side pane:** check that `LocalPreviewSite` serves `.svg` and `.mp4` from the page folder. The Explore pass said SVG is refused as a separate file, so the template inlines SVG for now. Video playback needs testing. If it fails, widen the allowed passive types in that file.
- **Live pages:** a page with `live: true` opens static. After "Run them", the Anatomy figure moves. A v2 asks again.
- Land with `tb land` on Wayne's fork. Never run `install-runner.sh` or `tb restart`.
