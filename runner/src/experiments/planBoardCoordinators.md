---
id: planBoardCoordinators
name: Plan board coordinators
title: Plan board coordinators: Send to… can hand a box of an approved plan to the project's coordinator as a to-do
added: 2026-11-26
on: false
reminder: 2206b508
---

## What it does

Needs Plan board on, and a copy of Threadbear with Mike's project coordinators. Once a plan is approved, a box's Send to… can also **Give to the project coordinator**: rather than a new thread, the project's coordinator gets one to-do with the same brief, and the box shows its state with the same badges. It's offered only when the board's thread is in a project whose coordinator is running. You press Start each time; nothing is handed over in bulk or by an agent. Coordinators run Claude only today. Wayne's copy has no coordinators, so turning this on there changes nothing.

## Lives in

`BoardCoordinatorPort`, the `coordinator` destination of `board.send` and the `coordinator` field of `board.get` in `runner/src/board-methods.ts`, and the `coordinators` port and `coordinatorsOn` passed to `boardMethods` in `runner/src/main.ts` (none on a copy without coordinators). In the desktop app: `CoordinatorTask` and `coordinator_status` in `desktop/core/src/plan_board.rs`, and the destination in the Send to… sheet in `desktop/gpui/src/plan_board.rs`. The spec is `features/plan-board-coordinators.feature`, skipped on a copy without coordinators. Tests: the Phase 3 cases in `runner/test/unit/board-methods.test.ts`.

## Taking it out

Delete this file, `BoardCoordinatorPort` and the `coordinator` branches in board-methods.ts with their tests, `coordinatorsOn` and `coordinators` in main.ts's `boardMethods` call, `CoordinatorTask` and `coordinator_status` in plan_board.rs, the destination in the sheet, and the feature file. Plan board keeps working. Run `npm run experiment-docs`.
