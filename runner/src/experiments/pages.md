---
id: pages
name: Pages
title: Pages: named, versioned pages a thread keeps for you
added: 2026-11-10
on: false
reminder: ee0c4ba6
---

## What it does

Lets a thread's agent make a Page you keep: a chart, a dashboard, a report or a doc, from a spreadsheet or data file you give it, a project's files, or a command like the Convex bundle analyzer. Asking again updates the same page to a new version instead of posting another, and the last 10 versions are kept. A page belongs to the thread that made it and can be in any of your projects, or none: a thread pinned to one project puts its pages there by itself. In a reply, a page shows as a card with a picture of it that follows its newest version. Each project's page lists its pages under Pages: click one to open it, and use its menu to Refresh it, Edit with Agent, edit a doc's text yourself, Download it (as files or as a PDF), open or restore an older version, move it between projects, or Delete it (kept 7 days under Recently deleted). Pages are HTML, CSS and pictures with no scripts, unless an agent makes a live one, whose scripts run only after you choose Run them. They're kept on your Mac, in the pages folder in Threadbear's data folder, and nothing about them goes to Convex. The guide's Pages section lists what to say.

## Lives in

`runner/src/pages.ts` (the store, `<home>/pages/`) and `runner/src/page-doc.ts` (docs); the page tools (`create_page`, `update_page`, `list_pages`, `read_page`, `delete_page`, `restore_page`, `export_page`, `add_page_to_project`, `remove_page_from_project`) in `buildThreadbearTools` (`runner/src/threadbear-mcp.ts`), with `list_pages` and `read_page` in `NEVER_ASK_TOOLS`; `pagesFor` and `pageMethods` (`pages.*` and `pages.updated`) and `askAboutPage` (Refresh and Edit with agent) in `runner/src/main.ts`; in the app, `ThreadbearCore/Protocol/Page.swift`, `Threadbear/Pages.swift` and `PagesSection` in `Views/ProjectPage.swift`; the card in replies, `Transcript/PageCards.swift` (`PageCardCell`), `PageSheets.swift`, `ThreadbearCore/Support/PagePDF.swift` and `pagePDF` in `RichRenderer.swift` (Download as PDF), `PageFence` in `ThreadbearCore/Markdown/RichBlock.swift` and `drawPageCard` in `Transcript/RichRenderer.swift`

## Taking it out

Delete `runner/src/pages.ts`, `runner/src/page-doc.ts` and its test, the page tools, their `pages` option and their names in `NEVER_ASK_TOOLS` in `threadbear-mcp.ts`, `pagesFor`, `pageMethods` and `askAboutPage` in `main.ts`, and the `pages.updated` notification in `runner.ts`. In the app, delete `Page.swift`, `Pages.swift`, `PageTests.swift`, `PagesSection`, `PageRow`, `PageMenu`, `AskPageSheet` and `EditDocSheet` in `ProjectPage.swift`, `pages`, `deletedPages` and `pagesUpdated` in `AppModel`, `RPCClient` and `RunnerAPI`, and `BusyKey.page`. the `Pages` section of `docs/guide.md`. Saved pages are the `pages` folder in Threadbear's data folder.
