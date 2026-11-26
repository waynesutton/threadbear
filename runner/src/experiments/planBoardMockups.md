---
id: planBoardMockups
name: Plan board mockups
title: Plan board mockups: a box on the plan board shows the agent's screens for it as sealed stills, up to three takes side by side
added: 2026-11-26
on: false
reminder: 2206b508
---

## What it does

Needs Plan board on. A box's **Screens…** shows the screens its agent wrote for that part of the plan (for example three takes on "KPI cards") as pictures in a strip beside the box: each with a title, a letter (A, B, C), a size (desktop, tablet or phone) and a version. They're drawn the way an html block in a reply is, sealed: no scripts run, nothing loads from the network, nothing is kept. Click one to put it in the message box as a chip, or press Pick on one to ring it green and send "KPI cards · B" as a chip. When the agent writes a new version of a screen, it gets a Changed badge. Open in browser opens the file in your browser with scripts blocked by its own policy; a screen of a real route on the thread's dev server (`http://localhost:5173/...`) opens there instead. No live page is ever drawn inside Threadbear.

## Lives in

`runner/src/board-frames.ts` (the files under `<dataDir>/boards/<threadId>/frames/`, the policy put first in every stored file, the active SVG, script and size checks, the route check, `FrameTools`), the `update_frame` and `read_frames` tools in `runner/src/threadbear-mcp.ts` (the `frames` option, only with planBoard on too), `board.frames` in `runner/src/board-methods.ts`, `framesFor` in `runner/src/main.ts`. In the desktop app: the frame strip layout and pick chip text in `desktop/core/src/plan_board.rs` (`frame_strip`, `frame_chip`), the stills in `desktop/gpui/src/plan_board.rs` (`ScreensState`, drawn through `threadbear_web::draw_page_at`), and `draw_page_at` beside `draw_page` in `desktop/web/src/mac/rich.rs`. The spec is `features/plan-board-mockups.feature`. Tests: `runner/test/unit/board-frames.test.ts`.

## Taking it out

Delete this file, `runner/src/board-frames.ts` and its test, the `frames` option with `update_frame` and `read_frames` in threadbear-mcp.ts, `board.frames` in board-methods.ts, `framesFor` in main.ts, `frame_strip` and `frame_chip` in desktop/core/src/plan_board.rs, `ScreensState` and the Screens… item in desktop/gpui/src/plan_board.rs, `draw_page_at` in rich.rs and its re-export, and the feature file. Plan board keeps working. Run `npm run experiment-docs`.
