# Tasks

## To Do

- [ ] Prints: publish, opt-in per print, so it opens on iPhone, the website and a share link.
- [ ] Prints: a sheet kind (CSV kept beside it, full table, Edit Data, CSV download; no formulas).
- [ ] Prints: a diagram kind (Mermaid source drawn with Pele, Edit Text).
- [ ] Prints: read-only list and read for outside agents (Grok).
- [ ] Prints: scheduled refresh, as a line in the make-print skill and the guide (no code).

- [ ] Pages: open a page in the thread's side pane (today the project page opens it in its own window), and reload the open pane on `pages.updated`. prds/threadbear-pages.md
- [ ] Pages: only ask once per page for `update_page` from the thread that owns it (adapter change). prds/threadbear-pages.md
- [ ] Pages: optional HyperFrames share clip for any page. prds/threadbear-pages.md

## In Progress

## Completed

- [x] 2026-11-10 19:41 UTC Pages renamed to Prints (Wayne picked it) in everything people and agents see: tool names (old ones kept as aliases), the ```print block (```page still draws), the app's wording, guide, protocol and experiment docs. Internals, RPC, ids and the experiment id keep "page". runner/src/threadbear-mcp.ts, pages.ts, main.ts, app Page.swift, RichBlock.swift, PageCards.swift, ProjectPage.swift, docs. Verified: tsc; pages.test.ts 16 pass; swift build; PageTests, PageFenceTests, TipsTests, EveryClickShowsTests pass.

- [x] 2026-11-10 18:34 UTC Download as PDF in both page menus (project page and the card in replies), for the newest version and each kept one: the page laid out light at 800 px in the card's locked-down view, captured whole by WebKit, cut into US Letter pages (PagePDF). app Pages.swift, PageSheets.swift, ProjectPage.swift, RichRenderer.swift, ThreadbearCore/Support/PagePDF.swift, a debug-only pagePDF automation command. Verified: swift build; PagePDFTests, PageTests, TipsTests, EveryClickShowsTests pass; a dashboard (2 pages) and a report (1 page) made through a debug app against a dev runner and looked at.

- [x] 2026-11-10 17:12 UTC Page card in replies, look B "Preview on top" (Wayne picked it in review 1df116ab): the top of the page drawn off screen, title, kind, version, age, New version, Open, Refresh and a ⋯ menu; follows new versions. app Transcript/PageCards.swift, PageSheets.swift, RichRenderer.swift, MarkdownRenderer.swift, TextKit.swift, RichBlock.swift (PageFence). Verified: swift build; PageTests, PageFenceTests, TipsTests, EveryClickShowsTests, ProjectsAppTests pass; built card compared with the B mockup in a debug app against a dev runner (~/ThreadbearThreads/page-card-mocks/B-built-in-app.png).

- [x] 2026-11-10 15:53 UTC Page kinds (chart, dashboard, report, doc, bundle), docs you can edit, delete with 7-day restore, versions, download, Refresh and Edit with Agent; a row menu and Recently deleted on the project page; a Pages section in docs/guide.md. prds/threadbear-pages.md. runner/src/pages.ts, runner/src/page-doc.ts, runner/src/threadbear-mcp.ts, runner/src/main.ts, app Page.swift, Pages.swift, ProjectPage.swift, RunnerAPI, docs/protocol.md, docs/guide.md, docs/experiments.md. Verified: `node --test test/unit/pages.test.ts` 14 pass; `swift build` clean; PageTests, TipsTests, EveryClickShowsTests and Projects tests pass; seen in a debug app against a seeded dev runner.
- [x] 2026-11-10 15:53 UTC make-page skill at `~/.claude/skills/make-page` (outside the repo): CSV, TSV, JSON and XLSX in; bar, column, line, segmented and pie charts, tiles and tables out, in a palette checked with the dataviz validator; dashboard and report pages; PDF export. Verified: `node --test test.mjs` 8 pass; dark, light and PDF looked at.

- [x] 2026-11-10 10:19 UTC Pages section on the project page: rows with title, version, age and summary; a click opens the page in a window of its own. prds/threadbear-pages.md. app/Sources/ThreadbearCore/Protocol/Page.swift, app/Sources/Threadbear/Pages.swift, Views/ProjectPage.swift, AppModel, RPCClient, RunnerAPI. Verified: `swift build` clean; `swift test --filter "PageTests|TipsTests|EveryClickShowsTests|ProjectsAppTests|ProjectsLookTests"` pass; seen in a debug app against a seeded dev runner.
- [x] 2026-11-10 09:50 UTC Pages protocol: `pages.list`, `pages.get`, `pages.setProjects`, `pages.updated`. prds/threadbear-pages.md. docs/protocol.md, runner/src/pages.ts, runner/src/runner.ts, runner/src/main.ts. Verified: `node --test test/unit/pages.test.ts` (10 pass), contracts, main, highlights and sync tests pass.
- [x] 2026-11-10 09:45 UTC Bundle report skill at `~/.claude/skills/bundle-report`: `report.mjs` runs the analyzer and builds a Split hero page (Anatomy scale figure, tiles, insights, packages, heaviest functions, Pele import chains). Outside the repo. Verified: `node --test test.mjs` 6 pass; Anatomy `lines.mjs` reports 0 floating line ends; dark, light and 390 px shots checked; both built pages load into the Pages store.
- [x] 2026-11-10 09:10 UTC Pages runner store and agent tools behind the `pages` experiment. prds/threadbear-pages.md. runner/src/pages.ts, runner/src/threadbear-mcp.ts, runner/src/main.ts, runner/src/experiments/pages.md, runner/test/unit/pages.test.ts, docs/experiments.md, docs/on-your-mac.md. Verified: `npx tsc --noEmit` clean; `node --test test/unit/pages.test.ts` 9 pass; experiments, experiment-docs and context-budget tests pass.
