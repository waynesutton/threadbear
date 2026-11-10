---
id: pages
name: Pages
title: Pages: named, versioned pages a thread keeps for you
added: 2026-11-10
on: false
reminder: 00000000
---

## What it does

Lets a thread's agent make a Page: a named page Threadbear keeps for you, like a bundle size report or a progress card, that outlives the reply it was posted in. Asking again updates the same page to a new version instead of posting another one, and the last 10 versions are kept. A page belongs to the thread that made it and can also be in any of your projects, or none: a thread pinned to one project puts its pages there by itself, and you can ask for a page to go in a project or stay out of them all. Pages are HTML, CSS and pictures with no scripts, unless the agent makes a live one, whose scripts run only in the side pane after you choose Run them. They're kept on your Mac, in the pages folder in Threadbear's data folder, and nothing about them goes to Convex.

## Lives in

`runner/src/pages.ts` (the store, `<home>/pages/`); the page tools (`create_page`, `update_page`, `list_pages`, `read_page`, `add_page_to_project`, `remove_page_from_project`) in `buildThreadbearTools` (`runner/src/threadbear-mcp.ts`), with `list_pages` and `read_page` in `NEVER_ASK_TOOLS`; `pagesFor` in `runner/src/main.ts`

## Taking it out

Delete `runner/src/pages.ts` and its test, the page tools, their `pages` option and their names in `NEVER_ASK_TOOLS` in `threadbear-mcp.ts`, and `pagesFor` in `main.ts`. Saved pages are the `pages` folder in Threadbear's data folder.
