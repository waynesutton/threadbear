---
id: tasks
name: Tasks
title: Tasks with a synced board, list and project links
added: 2026-11-16
on: false
reminder: cb15fc10
---

## What it does

Tasks sits below Notes (Command-7). It opens on a quick-add list: type a task, press Return, tick it off. Switch to a board with Backlog, On Deck, Shaping, Building, Observing, Done, Paused and Canceled (rename or delete any but Building), drag cards, columns, sections and projects into the order you like, and choose which view opens by default. Tasks can belong to a project or stand alone. Their titles, notes, status and appearance are kept in your own Convex deployment, so the signed-in website works while your Mac is off. Agents can add and update tasks when you ask. With Linear connected in Connectors, a task can be sent to Linear, linked to an issue, or brought in from one, and kept in sync from Linear; without it nothing about Linear shows. Notes stay private to your Mac and Notion boards remain separate.

Requires backend API 107 for Linear, 106 for the eight columns and deleting them, 103 for custom columns and Send to thread (100 for the original collection, 101 for Doing and saved orders). Editing needs a connection; an open page retains its last loaded tasks if it loses contact. The desktop and agent tools follow this switch. The authenticated website offers Tasks once its backend supports it.

## Lives in

`convex/tasks.ts`, `convex/taskFields.ts` and their schema tables; `runner/src/tasks.ts` and its RPC/tool wiring; `desktop/gpui/src/tasks_page.rs` and Command-7; `web/src/tasks.tsx` and `tasks.css`. Due times and reminders (API 105): `runner/src/task-reminders.ts`, `tasks:claimReminder`, the sliders popover's Notifications and `desktop/gpui/src/notify.rs`. Linear (API 107): `convex/tasksLinear.ts`, `runner/src/linear-tasks.ts` (`linear.*` RPCs and the sync sweep), `desktop/gpui/src/linear_send.rs` and `linear_import.rs`, the Linear chip in `web/src/tasks.tsx`, and the gateway's `threadbear` agent and its rules in Connectors' `policy.json`. Favorites and keys (API 108): `favorite` on tasks, `desktop/gpui/src/task_keys.rs`, and the star in `web/src/tasks.tsx`.

## Taking it out

Turn off Tasks to hide the desktop page and agent tools. Stored tasks stay in Convex. Removing the feature also removes its web route, task functions/tables, runner handlers and desktop navigation. Do not change encrypted Notes or the Notion task board integration.
