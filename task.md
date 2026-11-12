# Tasks

## To Do

Status and gap list: prds/prints-status.md

- [ ] Prints: ask once per Print on Codex threads too.
- [ ] Prints: optional HyperFrames share clip for any print. prds/threadbear-pages.md

## In Progress

- [ ] Land the network probe test fix and verify the runner update. prds/runner-update-probe-tests.md

## Completed

- [x] 2026-11-12 04:54 UTC Remove network probe tests' wall-clock assumptions using a controlled socket and mock clock. runner/test/unit/net-probe.test.ts. Verified: four focused tests, runner typecheck, and mutation checks rejecting a missing absolute deadline and missing response-size limit. prds/runner-update-probe-tests.md

- [x] 2026-11-11 Prints in Highlights and on the right-hand rail. Highlights lists the Prints a thread made; a Prints button under Files lists every Print, each with its projects or "No project", and opens it in the side pane (`PrintRows`, `PrintRow`, `PrintsList`, `BuiltinPanels.prints`). Wayne decided Prints stays an experiment. Verified: PageTests, TipsTests, EveryClickShowsTests; seen in a debug app against a dev runner with one Print in a project and one in none.

- [x] 2026-11-11 Prints round two. Ask once per Print for updates in Ask before edits (Claude adapter, `printGrants`). A reply's Print card opens in the thread's side pane, which follows new versions (`openPageHere`, `followPageVersions`, `PagePath`). Print cards on the website from `prints:list` (`web/src/print-card.tsx`, `print-fence.ts`). Scheduled refresh in the guide. iPhone parts left out on purpose. Verified: 32 adapter tests; PageTests, TipsTests, EveryClickShowsTests; 95 web tests and web typecheck; the side pane moving v1 to v2 to v3 in a debug app's state against a dev runner.

- [x] 2026-11-11 Only make Prints when I ask: `printsWhenAsked` in runner.json and Settings > General > Prints (shown while Prints is on). With it on, `create_print` after a message that names no print, chart, graph, dashboard or diagram puts an approval card up first; its description tells agents too. runner config.ts, settings.ts, cli.ts, main.ts, threadbear-mcp.ts; app RunnerSettings.swift, RunnerSettingsModel.swift, SettingsView.swift; protocol, guide, features/prints.feature. Verified: runner typecheck; 107 tests in pages, context budget, threadbear tools and experiments; app SettingsTests, TipsTests and EveryClickShowsTests; the switch seen on and off in a debug app against a dev runner. prds/prints-only-when-asked.md

- [x] 2026-11-11 Diagram Prints and read-only published Prints for outside agents landed on main at `b221756c` and pushed to GitHub. Rebased onto the publishing recovery `e8dd6683` (backend API 73, live). Verified: `./check.sh --quick` passed in 2,916 s; runner typecheck and 25 guard tests (context budget, experiments, parity, scenarios, fork view). Three desktop specs stay manual until GPUI has Prints. prds/prints-diagrams-and-agent-reads.md

- [x] 2026-11-10 20:05 UTC Publish a print (opt-in): backend API 73 (`prints` table, sync:publishPrint, sync:unpublishPrint, prints:view, prints:list), runner pages.publish and pages.unpublish with new versions following, publish_print and unpublish_print tools, Publish…, Copy Link, Open on the Web and Unpublish in both Mac menus, and the website's /p/<slug> page in a sandboxed frame. Verified: convex and web typecheck; runner pages.test.ts and runner-access tests; web tests; swift build and focused app tests.

- [x] 2026-11-10 19:41 UTC Pages renamed to Prints (Wayne picked it) in everything people and agents see: tool names (old ones kept as aliases), the ```print block (```page still draws), the app's wording, guide, protocol and experiment docs. Internals, RPC, ids and the experiment id keep "page". runner/src/threadbear-mcp.ts, pages.ts, main.ts, app Page.swift, RichBlock.swift, PageCards.swift, ProjectPage.swift, docs. Verified: tsc; pages.test.ts 16 pass; swift build; PageTests, PageFenceTests, TipsTests, EveryClickShowsTests pass.

- [x] 2026-11-10 18:34 UTC Download as PDF in both page menus (project page and the card in replies), for the newest version and each kept one: the page laid out light at 800 px in the card's locked-down view, captured whole by WebKit, cut into US Letter pages (PagePDF). app Pages.swift, PageSheets.swift, ProjectPage.swift, RichRenderer.swift, ThreadbearCore/Support/PagePDF.swift, a debug-only pagePDF automation command. Verified: swift build; PagePDFTests, PageTests, TipsTests, EveryClickShowsTests pass; a dashboard (2 pages) and a report (1 page) made through a debug app against a dev runner and looked at.

- [x] 2026-11-10 17:12 UTC Page card in replies, look B "Preview on top" (Wayne picked it in review 1df116ab): the top of the page drawn off screen, title, kind, version, age, New version, Open, Refresh and a ⋯ menu; follows new versions. app Transcript/PageCards.swift, PageSheets.swift, RichRenderer.swift, MarkdownRenderer.swift, TextKit.swift, RichBlock.swift (PageFence). Verified: swift build; PageTests, PageFenceTests, TipsTests, EveryClickShowsTests, ProjectsAppTests pass; built card compared with the B mockup in a debug app against a dev runner (~/ThreadbearThreads/page-card-mocks/B-built-in-app.png).

- [x] 2026-11-10 15:53 UTC Page kinds (chart, dashboard, report, doc, bundle), docs you can edit, delete with 7-day restore, versions, download, Refresh and Edit with Agent; a row menu and Recently deleted on the project page; a Pages section in docs/guide.md. prds/threadbear-pages.md. runner/src/pages.ts, runner/src/page-doc.ts, runner/src/threadbear-mcp.ts, runner/src/main.ts, app Page.swift, Pages.swift, ProjectPage.swift, RunnerAPI, docs/protocol.md, docs/guide.md, docs/experiments.md. Verified: `node --test test/unit/pages.test.ts` 14 pass; `swift build` clean; PageTests, TipsTests, EveryClickShowsTests and Projects tests pass; seen in a debug app against a seeded dev runner.
- [x] 2026-11-10 15:53 UTC make-page skill at `~/.claude/skills/make-page` (outside the repo): CSV, TSV, JSON and XLSX in; bar, column, line, segmented and pie charts, tiles and tables out, in a palette checked with the dataviz validator; dashboard and report pages; PDF export. Verified: `node --test test.mjs` 8 pass; dark, light and PDF looked at.

- [x] 2026-11-10 10:19 UTC Pages section on the project page: rows with title, version, age and summary; a click opens the page in a window of its own. prds/threadbear-pages.md. app/Sources/ThreadbearCore/Protocol/Page.swift, app/Sources/Threadbear/Pages.swift, Views/ProjectPage.swift, AppModel, RPCClient, RunnerAPI. Verified: `swift build` clean; `swift test --filter "PageTests|TipsTests|EveryClickShowsTests|ProjectsAppTests|ProjectsLookTests"` pass; seen in a debug app against a seeded dev runner.
- [x] 2026-11-10 09:50 UTC Pages protocol: `pages.list`, `pages.get`, `pages.setProjects`, `pages.updated`. prds/threadbear-pages.md. docs/protocol.md, runner/src/pages.ts, runner/src/runner.ts, runner/src/main.ts. Verified: `node --test test/unit/pages.test.ts` (10 pass), contracts, main, highlights and sync tests pass.
- [x] 2026-11-10 09:45 UTC Bundle report skill at `~/.claude/skills/bundle-report`: `report.mjs` runs the analyzer and builds a Split hero page (Anatomy scale figure, tiles, insights, packages, heaviest functions, Pele import chains). Outside the repo. Verified: `node --test test.mjs` 6 pass; Anatomy `lines.mjs` reports 0 floating line ends; dark, light and 390 px shots checked; both built pages load into the Pages store.
- [x] 2026-11-10 09:10 UTC Pages runner store and agent tools behind the `pages` experiment. prds/threadbear-pages.md. runner/src/pages.ts, runner/src/threadbear-mcp.ts, runner/src/main.ts, runner/src/experiments/pages.md, runner/test/unit/pages.test.ts, docs/experiments.md, docs/on-your-mac.md. Verified: `npx tsc --noEmit` clean; `node --test test/unit/pages.test.ts` 9 pass; experiments, experiment-docs and context-budget tests pass.
