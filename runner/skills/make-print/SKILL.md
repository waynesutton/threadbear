---
name: make-print
description: Makes Threadbear Prints (called Pages until Nov 2026) from data or a project, and a live progress print for long tasks. Charts (bar, column, line, pie, part-to-whole), dashboards, reports and docs, from a CSV, TSV, JSON or XLSX the user uploads or names, a project's files, or any command's output, including a Convex insights report (`npx convex insights`) and a Convex API map (`npx convex function-spec`). Also refreshes, edits, versions, downloads (HTML or PDF), moves and deletes prints. Use when the user asks for a print, page, chart, graph, pie chart, dashboard, report, write-up, or "show me X as a print", in a project or not. The bundle-report skill is the recipe for Convex bundle size prints.
---

# Make a print

A Print is what Threadbear called a Page until 10 Nov 2026. The tools are `create_print`, `update_print` and so on; the old `_page` names still work. It is still an HTML page inside, so "page" below means the file, and "print" means the thing Threadbear keeps.

Every kind ends the same way. The page is static HTML, CSS and SVG, saved with `create_print` or `update_print`, and shown with a ```` ```print <id> ```` block in your reply. They all share the Prints look: the same palette, cards, and light and dark themes.

`<this skill's folder>` in the commands below is the base directory you were given for this skill.

## Pick the kind

| The user says | Kind | How |
|---|---|---|
| "chart / graph / pie of X" | `chart` | A spec with one chart, then `build.mjs` (below) |
| a long task (more than 5 steps, or over 30 minutes) | `progress` | `progress.mjs`, below. Make it before starting, and update it after every step |
| "dashboard for this data" | `dashboard` | A spec with tiles, 2 to 4 charts and a table |
| "report on X", "how is the project going" | `report` | A spec with `sections`: markdown text plus charts |
| "write up / doc / plan / notes as a print" | `doc` | `create_print` with `markdown`. No build step. The user can edit it in the app |
| "todo list", "checklist", "track these tasks" | `todo` | `create_print` with `kind: "todo"` and `markdown`: `- [ ] item` lines, `- [x]` for done ones, `##` headings to group them. No build step. The user ticks items by clicking |
| "kanban", "board", "put these on a board" | `board` | `create_print` with `kind: "board"` and `markdown`: one `##` heading per column (To do, Doing, Done) and a `- card` line per card. No build step. The user moves cards with each card's arrows |
| "bundle report", "run bundle-analyzer" | `bundle` | The `bundle-report` skill |
| "insights report", "convex insights", "any OCC conflicts?" | `dashboard` | `insights.mjs`, below. A Convex app's insights for the last 72 hours |
| "API map", "map my Convex functions", "what does the backend expose?" | `dashboard` | `apimap.mjs`, below. Every function and HTTP route, public or internal, and what it takes |
| "add a figure that explains it" | any kind | An Anatomy or Pele SVG as `figure` in the spec. Only when asked: it's large |
| "make a clip of it" | any kind | HyperFrames, only when asked. Needs Node 22+ and FFmpeg |

When the request doesn't name a chart type, choose by what the data has to show (this follows the `dataviz` skill):
- **Comparing categories:** `bar`, ranked.
- **Over time, or anything in order:** `column`, or `line` for several series.
- **One whole split into 6 parts or fewer:** `segmented`.
- **Pie:** use `pie` (drawn as a donut) only when the user asks for a pie, and only with 6 parts or fewer. Past that, say so and use `bar`.
- **A single number:** a tile, never a one-bar chart.

## Data and prints

1. **Find the data.**
   - An upload or a named file: CSV, TSV, JSON or XLSX. For XLSX, pass `"sheet"` if the sheet isn't the first.
   - A project: `read_project` for its folders, then read what's there.
   - Numbers you worked out yourself: put them in the spec as `items` or a list of row objects.
   - Read a few rows first, so the column names in the spec are exact.
2. **Write a spec** next to the data, or in `$TMPDIR/pages/<slug>/spec.json`:

   ```json
   { "kind": "dashboard", "title": "Sales 2026", "subtitle": "optional",
     "data": { "file": "sales.xlsx", "sheet": "Sales 2026" },
     "tiles": [ { "label": "Revenue", "value": "Revenue", "agg": "sum", "unit": "$" },
                { "label": "Rows", "agg": "count" },
                { "label": "Status", "value": "On track" } ],
     "charts": [ { "type": "line", "title": "Revenue by month", "by": "Month", "value": "Revenue", "series": "Product", "unit": "$" },
                 { "type": "bar", "title": "By region", "by": "Region", "value": "Revenue", "top": 10 },
                 { "type": "pie", "title": "Share", "items": [ { "label": "A", "value": 3 } ] } ],
     "table": { "columns": ["Month", "Region", "Revenue"], "sort": "Revenue", "limit": 20, "units": { "Revenue": "$" } },
     "sections": [ { "heading": "Where it stands", "markdown": "Text **with** lists.", "chart": { "type": "column", "by": "Month", "value": "Seats" } } ],
     "figure": "optional.svg",
     "footer": "optional source line" }
   ```

   - `agg` is `sum` (the default with a `value`), `count`, `avg`, `min` or `max`.
   - `type` is `bar`, `column`, `line`, `segmented` or `pie`.
   - `sections` makes it a report.
3. **Build it:**

   ```sh
   node <this skill's folder>/build.mjs <spec.json> <out dir>
   ```

   A wrong column name fails and lists the real ones. Fix the spec and run it again.
4. **Save it.**
   - Call `list_prints` first. If this data already has a print (same `source`), update it instead of making a new one.
   - Otherwise call `create_print({ title, summary, dir: <out dir>, kind, source: <data file>, recipe, show: true })`. The `recipe` says how to make it again, for the Refresh button, for example: `node <this skill's folder>/build.mjs /path/spec.json <out>; data from /path/sales.xlsx`.
   - Leave `projects` out so the print follows the thread's project, or pass `[]` to keep it out of projects.
5. **Reply.** Put the ```` ```print <id> ```` block in your reply, then give two or three lines on what the data says.

Look at the page before you say it's done. The anatomy skill's `scripts/capture.mjs` takes a shot, with `--theme dark` and `--theme light`, and at `--width 390` for a phone.

## Convex insights reports

For a Convex app's health: write conflicts (OCC) and functions near or over their read limits, from `npx convex insights` (https://docs.convex.dev/cli/reference/insights). It covers the last 72 hours, needs a cloud deployment and a Convex login, and only reads.

1. In the app's folder (the one with `convex/`), save the insights as JSON. Add `--prod` for production, or `--deployment <name>` for another one. Say which you used.

   ```sh
   npx convex insights --details --json > $TMPDIR/pages/convex-insights/insights.json
   ```

2. Build it. With insights, it's a dashboard: tiles for insights, calls affected, errors and functions, calls by function and by kind, and a table of every insight, errors first, with its table and when it was last seen. A component's function shows its component in brackets, `messages.js:list (agent)`. With an empty list, it's a short all-clear report. Output with no `insights` list (a failed command) stops with an error rather than reading as all clear.

   ```sh
   node <this skill's folder>/insights.mjs $TMPDIR/pages/convex-insights/insights.json $TMPDIR/pages/convex-insights/out
   ```

3. Save it with `create_print({ title, summary, dir: <out>, kind: "dashboard", source: "convex insights <deployment>", recipe, show: true })`, or `update_print` when `list_prints` already has one with that `source`. The recipe is the two commands above. In the summary, say how many insights and the worst one ("6 conflicts, most on phone.js:update").
4. In the reply, name the function with the most calls and what usually fixes it: for a conflict, two mutations writing the same document, so split the document or move the hot field into its own table; for a read limit, an index or a smaller `.take()`.

Only `occRetried` has been seen in real output so far. The other kinds (`occFailedPermanently`, `bytesReadLimit`, `bytesReadThreshold`, `documentsReadLimit`, `documentsReadThreshold`) are labelled by name, and a kind it doesn't know shows as itself.

## Convex API maps

Every function a Convex deployment runs, from `npx convex function-spec` (https://docs.convex.dev/cli): queries, mutations, actions and HTTP routes, public or internal, the arguments each takes (`?` for optional), and whether it has argument and return validators. It reads the deployment's metadata, never its data. Read it before adding a function, to see what's there and what's public.

1. In the app's folder, save the spec. Add `--prod` for production. Say which you used.

   ```sh
   npx convex function-spec > $TMPDIR/pages/convex-api/functions.json
   ```

2. Build it. Public functions missing a validator come first, then the rest of the public ones, then internal ones.

   ```sh
   node <this skill's folder>/apimap.mjs $TMPDIR/pages/convex-api/functions.json $TMPDIR/pages/convex-api/out
   ```

3. Save it with `create_print({ title, summary, dir: <out>, kind: "dashboard", source: "convex function-spec <deployment>", recipe, show: true })`, or `update_print` when one with that `source` exists. The summary says the counts ("97 functions, 76 public, 0 missing validators").
4. In the reply, name any public function without validators: anyone can call it with anything. The fix is `args` and `returns` validators, and an auth check if it reads or writes someone's data.

Output with no `functions` list (not logged in, no deployment) stops with an error rather than drawing an empty map. A function without a validator is read as `{ "type": "any" }`, which hasn't been seen in real output yet.

## Progress prints for long tasks

For a task with more than 5 steps, or one that should take over 30 minutes, make a progress print **before you start**, and update it after every step. It shows:
- what's done and what's under way
- questions waiting on the user, each with what you'll do if there's no answer
- anything stuck
- the latest deliverables

Panels with nothing in them are left out. Every time comes from the real clock.

```sh
P=<this skill's folder>/progress.mjs
S=$TMPDIR/pages/<slug>/state.json
node $P $S init "<title>" "<the task in one line>"     # writes the page next to it in page/
node $P $S step "<name>" todo                          # list the plan first
node $P $S step "<name>" doing "<note>"                # then after each step: doing, done, stuck or skipped
node $P $S ask "<question>" "<what you'll do by default>"
node $P $S answer "<question>" "<their answer>"
node $P $S stuck "<what>" "<why>"   /   unstuck "<what>"
node $P $S deliver "<what>" "<path or link>"
node $P $S note "<one line on where it stands>"
```

1. **Start:** after `init` and the plan's steps, call `create_print({ title, summary, dir: <state dir>/page, kind: "dashboard", source: "progress: <slug>", recipe: "node <this skill's folder>/progress.mjs <state.json> build", show: true })`. Put its ```` ```print <id> ```` block in your reply.
2. **After every step:** run the command, then `update_print({ id, dir: <state dir>/page, summary: "<n> of <m> done, <k> waiting on you" })`. The card in the thread follows the new version by itself. Never start a new print for the same task.
3. **Questions:** when you need a decision, `ask` it with the default you'll take, then keep going with that default. Say so in your reply too.
4. **Anything that can't be undone** (deleting, publishing, sending, paying) waits for the user's answer. Mark it `stuck` until then.
5. **Finished:** the last step is `done`, then `deliver` what was made, then one last `update_print`.

## Style (asked once)

Every print uses the style saved in `~/.config/threadbear/print-style.json`:

```json
{ "theme": "auto", "density": "normal", "accent": "#4fb3d9" }
```

- `theme` is `auto` (follows the viewer's appearance), `dark` or `light`.
- `density` is `dense`, `normal` or `airy`.
- `accent` is one `#rrggbb` colour, used for progress, highlights and links.

If the file doesn't exist the first time you make a print, ask the user once: dark, light or auto; dense, normal or airy; and one accent colour. Save their answer there, and don't ask again. Before saving an accent, check its contrast against the card colours with the `dataviz` skill's `validate_palette.js`. If it's too faint, suggest the nearest step that passes. A spec can override the saved style with its own `"style": { ... }`.

## Docs (editors)

- **Agent writes:** `create_print({ title, summary, markdown, kind: "doc" })`.
  - The markdown is a small, safe subset: headings, lists, `- [ ]` task lists, tables, quotes, code and links. No HTML passes through.
- **User edits:** they can edit the doc in the app (Edit Text on the print), and Save makes a new version marked "edited by the user".
  - Before changing a doc, call `read_print`. If it says the user edited it, start from their text, never from your own copy.

## Managing prints

| The user says | Do |
|---|---|
| "update", "refresh this print" | Follow its recipe (`read_print`), rebuild, then `update_print({ id, dir })` |
| "change the chart to bars" | Edit the spec, rebuild, then `update_print` |
| "show version 3", "go back to version 3" | `read_print({ id, version: 3 })`, or `update_print({ id, restoreVersion: 3 })` |
| "delete it" | `delete_print`. It's kept 7 days |
| "bring it back" | `list_prints({ deleted: true })`, then `restore_print` |
| "download it", "save a copy" | `export_print({ id, folder })` for HTML. For a PDF, then run `node <this skill's folder>/export-pdf.mjs <folder>/index.html <file.pdf>` |
| "refresh it every morning", "keep it up to date" | `schedule_message` to this thread with a cron (for example `0 8 * * 1-5`) and the text "Refresh the print <id> from its recipe, then update_print." Say when it will run. `list_schedules` and `delete_schedule` change or stop it |
| "publish it", "put it on my phone", "give me a link" | `publish_print({ id })`. It always asks the user first. Only when they ask: a print can hold private data. `unpublish_print` takes it down |
| "put it in Poddle", "out of projects" | `add_print_to_project` or `remove_print_from_project` |
| "what prints do I have" | `list_prints`, with `project` or `none` to narrow it |

## Files

| File | What it does |
|---|---|
| `build.mjs` | Spec in, `index.html` out. Also exports `buildPage(spec, baseDir)` |
| `charts.mjs` | The charts: bar, column, line, segmented, pie, tiles and table, as static SVG |
| `data.mjs` | Reads CSV, TSV, JSON and XLSX (XLSX through the system's `unzip`, so nothing to install), and groups and totals |
| `theme.mjs` | The Prints look and the chart palette |
| `md.mjs` | The markdown used in report sections |
| `export-pdf.mjs` | PDF through headless Chrome |
| `insights.mjs` | `npx convex insights --json` in, an insights dashboard out. Exports `insightsSpec` and `insightsRows` |
| `apimap.mjs` | `npx convex function-spec` in, an API map dashboard out. Exports `apiSpec` and `apiRows` |
| `fixtures/` | `sales.csv`, `sales.xlsx`, and `pie.json`, `dashboard.json` and `report.json` specs, `insights.json` (insights output with the real shape) and `functions.json` (function-spec output, trimmed, with one unvalidated function added) |
| `progress.mjs`, `progress-view.mjs` | Progress prints: the state commands, and the view |
| `style.json` | The user's saved style. Asked once; missing until then |
| `test.mjs` | `node --test test.mjs` |

**Palette.** It's the `dataviz` skill's categorical palette without violet, six colours in a fixed order, checked with its `validate_palette.js` against these prints' cards, light (`#fbfaf7`) and dark (`#12161b`), on 10 Nov 2026. Anything past six folds into a grey "Other". The light check warns on contrast, so every chart keeps its labels and a "Show the data" table. Don't remove them.
