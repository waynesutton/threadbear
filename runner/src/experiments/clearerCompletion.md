---
id: clearerCompletion
name: Clearer thread completion
title: Task progress on completion cards, with amber edges on action buttons
added: 2026-11-17
on: false
reminder: 5c8e322b
---

## What it does

Shows the agent's progress on the whole request above the message box: which tasks are completed, pending or blocked, and how many are complete. Task progress carries across turns. The agent reports it; Threadbear's separate loose-end check shows repository work, running helpers and scheduled check-ins. Only a successful empty check says no loose ends were found. Action buttons such as Archive and answers keep their neutral fill with a thin amber left edge. Archive stays manual. Older threads without reported tasks keep their existing card. The list also stays in the thread's Highlights as Task list after you reply, with what's left first and the completed tasks folded.

## Lives in

`runner/src/completion.ts`, `runner.ts`, `store.ts` and `threadbear-mcp.ts`; completion fields in the protocol and API 100 mirror; the desktop reducer and composer card, website composer and phone card. Highlights' Task list: `runner/src/runner/highlights.ts` and `highlights.ts` (`threads.highlights`' `completion`), `desktop/core/src/highlights.rs` (`HighlightSections.tasks`) and `SidePane::tasks_block` in `desktop/gpui/src/highlights.rs`, with the mock runner's copy.

## Taking it out

Remove the tool argument and merge calls, completion fields and UI sections, and this file. Loose ends on the done card are standard and stay. Remove its runner.json key and remake experiment docs.
