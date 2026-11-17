---
id: clearerCompletion
name: Clearer thread completion
title: Task progress on completion cards, with amber edges on action buttons
added: 2026-11-17
on: false
reminder: 5c8e322b
---

## What it does

Shows the agent's progress on the whole request above the message box: which tasks are completed, pending or blocked, and how many are complete. Task progress carries across turns. The agent reports it; Threadbear's separate loose-end check shows repository work, running helpers and scheduled check-ins. Only a successful empty check says no loose ends were found. Action buttons such as Archive and answers keep their neutral fill with a thin amber left edge. Archive stays manual. Older threads without reported tasks keep their existing card.

## Lives in

`runner/src/completion.ts`, `runner.ts`, `store.ts` and `threadbear-mcp.ts`; completion fields in the protocol and API 85 mirror; the desktop reducer and composer card, website composer and phone card.

## Taking it out

Remove the tool argument and merge calls, completion fields and UI sections, and this file. Keep existing loose-end behavior for the looseEnds experiment. Remove its runner.json key and remake experiment docs.
