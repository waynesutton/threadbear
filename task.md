# Tasks

## To Do

- [ ] Pages: protocol (`pages.list`, `pages.get`, `pages.setProjects`, `pages.updated`) in docs/protocol.md, then the runner side. prds/threadbear-pages.md
- [ ] Pages: Mac app `page` fence card, Pages section on the project page, side pane reload on `pages.updated`. prds/threadbear-pages.md
- [ ] Pages: only ask once per page for `update_page` from the thread that owns it (adapter change). prds/threadbear-pages.md
- [ ] Bundle report skill: template (Split hero), `build-page.mjs`, Anatomy scale figure, Pele import chains, optional HyperFrames clip. prds/threadbear-pages.md
- [ ] Book the experiment check-in schedule and put its id in `runner/src/experiments/pages.md` (`reminder` is a placeholder). prds/threadbear-pages.md

## In Progress

## Completed

- [x] 2026-11-10 09:10 UTC Pages runner store and agent tools behind the `pages` experiment. prds/threadbear-pages.md. runner/src/pages.ts, runner/src/threadbear-mcp.ts, runner/src/main.ts, runner/src/experiments/pages.md, runner/test/unit/pages.test.ts, docs/experiments.md, docs/on-your-mac.md. Verified: `npx tsc --noEmit` clean; `node --test test/unit/pages.test.ts` 9 pass; experiments, experiment-docs and context-budget tests pass.
