# Tasks

## To Do

- [ ] Pages: `page` fence card in the transcript. Mock it for Wayne first (AGENTS.md): it goes through RichRenderer's off-screen pipeline. prds/threadbear-pages.md
- [ ] Pages: open a page in the thread's side pane (today the project page opens it in its own window), and reload the open pane on `pages.updated`. prds/threadbear-pages.md
- [ ] Pages: only ask once per page for `update_page` from the thread that owns it (adapter change). prds/threadbear-pages.md
- [ ] Bundle report: optional HyperFrames share clip. prds/threadbear-pages.md
- [ ] Book the experiment check-in schedule and put its id in `runner/src/experiments/pages.md` (`reminder` is a placeholder). prds/threadbear-pages.md

## In Progress

## Completed

- [x] 2026-11-10 10:19 UTC Pages section on the project page: rows with title, version, age and summary; a click opens the page in a window of its own. prds/threadbear-pages.md. app/Sources/ThreadbearCore/Protocol/Page.swift, app/Sources/Threadbear/Pages.swift, Views/ProjectPage.swift, AppModel, RPCClient, RunnerAPI. Verified: `swift build` clean; `swift test --filter "PageTests|TipsTests|EveryClickShowsTests|ProjectsAppTests|ProjectsLookTests"` pass; seen in a debug app against a seeded dev runner.
- [x] 2026-11-10 09:50 UTC Pages protocol: `pages.list`, `pages.get`, `pages.setProjects`, `pages.updated`. prds/threadbear-pages.md. docs/protocol.md, runner/src/pages.ts, runner/src/runner.ts, runner/src/main.ts. Verified: `node --test test/unit/pages.test.ts` (10 pass), contracts, main, highlights and sync tests pass.
- [x] 2026-11-10 09:45 UTC Bundle report skill at `~/.claude/skills/bundle-report`: `report.mjs` runs the analyzer and builds a Split hero page (Anatomy scale figure, tiles, insights, packages, heaviest functions, Pele import chains). Outside the repo. Verified: `node --test test.mjs` 6 pass; Anatomy `lines.mjs` reports 0 floating line ends; dark, light and 390 px shots checked; both built pages load into the Pages store.
- [x] 2026-11-10 09:10 UTC Pages runner store and agent tools behind the `pages` experiment. prds/threadbear-pages.md. runner/src/pages.ts, runner/src/threadbear-mcp.ts, runner/src/main.ts, runner/src/experiments/pages.md, runner/test/unit/pages.test.ts, docs/experiments.md, docs/on-your-mac.md. Verified: `npx tsc --noEmit` clean; `node --test test/unit/pages.test.ts` 9 pass; experiments, experiment-docs and context-budget tests pass.
