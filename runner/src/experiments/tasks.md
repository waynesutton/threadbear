---
id: tasks
name: Tasks
title: Tasks with a synced board, list and project links
added: 2026-11-16
on: false
reminder: cb15fc10
---

## What it does

Tasks sits below Notes (Command-7). It opens on a quick-add list: type a task, press Return, tick it off. Switch to a board with Backlog, Todo, Doing and Done, drag cards, columns, sections and projects into the order you like, and choose which view opens by default. Tasks can belong to a project or stand alone. Their titles, notes, status and appearance are kept in your own Convex deployment, so the signed-in website works while your Mac is off. Agents can add and update tasks when you ask. Notes stay private to your Mac and Notion boards remain separate.

Requires backend API 86 for custom columns and Send to thread (83 for the original collection, 84 for Doing and saved orders). Editing needs a connection; an open page retains its last loaded tasks if it loses contact. The desktop and agent tools follow this switch. The authenticated website offers Tasks once its backend supports it.

## Lives in

`convex/tasks.ts`, `convex/taskFields.ts` and their schema tables; `runner/src/tasks.ts` and its RPC/tool wiring; `desktop/gpui/src/tasks_page.rs` and Command-7; `web/src/tasks.tsx` and `tasks.css`. Due times and reminders (API 88): `runner/src/task-reminders.ts`, `tasks:claimReminder`, the sliders popover's Notifications and `desktop/gpui/src/notify.rs`.

## Taking it out

Turn off Tasks to hide the desktop page and agent tools. Stored tasks stay in Convex. Removing the feature also removes its web route, task functions/tables, runner handlers and desktop navigation. Do not change encrypted Notes or the Notion task board integration.
