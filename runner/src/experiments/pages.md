---
id: pages
name: Prints
title: Prints: charts, dashboards, reports and docs a thread keeps for you
added: 2026-11-10
on: false
reminder: ee0c4ba6
---

## What it does

Diagram Prints take Mermaid source and draw static SVG offline with Pele. Edit Text edits their source and saves a new version; Download includes the Mermaid and SVG. A malformed edit leaves the previous version unchanged. Published diagrams include their source. With Outside agents on, a key allowed to read all can list and read published Prints, never local unpublished ones. Editable CSV sheets are deferred.

Lets a thread's agent make a Print you keep (called Pages until 10 Nov 2026): a chart, a dashboard, a report or a doc, from a spreadsheet or data file you give it, a project's files, or a command like the Convex bundle analyzer. Asking again updates the same page to a new version instead of posting another, and the last 10 versions are kept. A page belongs to the thread that made it and can be in any of your projects, or none: a thread pinned to one project puts its pages there by itself. In a reply, a print shows as a card with a picture of it that follows its newest version. Each project's page lists its prints under Prints: click one to open it, and use its menu to Refresh it, Edit with Agent, edit a doc's text yourself, Download it (as files or as a PDF), open or restore an older version, move it between projects, or Delete it (kept 7 days under Recently deleted). Prints are HTML, CSS and pictures with no scripts, unless an agent makes a live one, whose scripts run only after you choose Run them. They're kept on your Mac, in the pages folder in Threadbear's data folder. Nothing about a print goes to Convex unless you choose Publish on it: then its page goes to your own Convex backend, opens on your phone, the website and for anyone with its link, follows new versions, and comes down when you unpublish or delete it. The guide's Prints section lists what to say.

## Lives in

Diagram rendering is in `runner/src/page-diagram.ts`, using the pinned bundle and license in `runner/skills/pele/`. The app reuses Edit Text; `Page.swift` and `RunnerAPI.swift` add diagram source and `pages.saveDiagram`.

`runner/src/pages.ts` (the store, `<home>/pages/`) and `runner/src/page-doc.ts` (docs); the page tools (`create_print`, `update_print`, `list_prints`, `read_print`, `delete_print`, `restore_print`, `export_print`, `add_print_to_project`, `remove_print_from_project`, and their old `page` names) in `buildThreadbearTools` (`runner/src/threadbear-mcp.ts`), with `list_prints` and `read_print` (and the old names) in `NEVER_ASK_TOOLS`; `pagesFor` and `pageMethods` (`pages.*` and `pages.updated`) and `askAboutPage` (Refresh and Edit with agent) in `runner/src/main.ts`; in the app, `ThreadbearCore/Protocol/Page.swift`, `Threadbear/Pages.swift` and `PagesSection` in `Views/ProjectPage.swift`; the card in replies, `Transcript/PageCards.swift` (`PageCardCell`), `PageSheets.swift`, `ThreadbearCore/Support/PagePDF.swift` and `pagePDF` in `RichRenderer.swift` (Download as PDF), `PageFence` in `ThreadbearCore/Markdown/RichBlock.swift` and `drawPageCard` in `Transcript/RichRenderer.swift`

## Taking it out

For diagrams, also remove `runner/src/page-diagram.ts`, `runner/test/unit/page-diagram.test.ts`, the copied `runner/skills/pele/pele.js` and its license, the Mermaid fields and save method in Pages and the app. The outside-agent read tools live with Outside agents, not the local Print store.

For publishing, also take out `convex/prints.ts`, the `prints` table in `convex/schema.ts`, `publishPrint` and `unpublishPrint` in `convex/sync.ts` and `convex/runnerAccess.ts`, `web/src/print.tsx`, `print-doc.ts` and the `/p/` route in `web/src/route.ts`; unpublish every print first, or their rows stay in the backend. Delete `runner/src/pages.ts`, `runner/src/page-doc.ts` and its test, the page tools, their `pages` option and their names in `NEVER_ASK_TOOLS` in `threadbear-mcp.ts`, `pagesFor`, `pageMethods` and `askAboutPage` in `main.ts`, and the `pages.updated` notification in `runner.ts`. In the app, delete `Page.swift`, `Pages.swift`, `PageTests.swift`, `PagesSection`, `PageRow`, `PageMenu`, `AskPageSheet` and `EditDocSheet` in `ProjectPage.swift`, `pages`, `deletedPages` and `pagesUpdated` in `AppModel`, `RPCClient` and `RunnerAPI`, and `BusyKey.page`. the `Prints` section of `docs/guide.md`. Saved pages are the `pages` folder in Threadbear's data folder.
