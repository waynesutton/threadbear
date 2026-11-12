# Where Prints stand

Updated 2026-11-12, after todo lists and boards. On main at `b221756c`, pushed to GitHub. Prints stay the `pages` experiment, off by default and on for Wayne. Wayne decided on 11 Nov to keep it an experiment rather than make it standard. The full user-facing walkthrough is the Prints section of `docs/guide.md`. This page is the short version plus the gap list.

## How Prints work

A Print is a page a thread makes for you to keep. Ask for a chart, a dashboard, a report, a doc or a diagram in plain words. Asking again updates the same Print to a new version instead of posting a second one.

| Piece | How it works |
|---|---|
| Storage | Files on your Mac in `~/Library/Application Support/Threadbear/pages/<id>/v<n>/`. Last 10 versions kept. Nothing goes to Convex unless you publish |
| Kinds | chart, dashboard, report, doc, diagram, bundle, todo, board |
| Projects | A thread pinned to one project files its Prints there. A Print can sit in several projects, or none |
| In a reply | A card with a preview, title, kind, version and age. It follows the newest version and says "New version" |
| Project page | Prints are listed with Open, Refresh, Edit with Agent, Edit Text, Download, Download as PDF, Versions, Projects, Publish and Delete |
| Diagrams | Mermaid source drawn offline with the bundled Pele 1.13.0 into static SVG. Edit Text edits the source. Bad source keeps the old version. A wide one uses the whole window and scrolls sideways, never below 70% of its size |
| Safety | No scripts by default. A live Print runs scripts only after you choose Run them, and again per version |
| Delete | Soft delete. Restore within 7 days from Recently deleted |
| Publishing | Opt in per Print. Backend API 73 stores one static `index.html` under a link. It opens on the phone, the website's `/p/<slug>` page and for anyone with the link. New versions follow. Unpublish or Delete takes it down |
| Only when asked | Settings > General > Prints > Only make Prints when I ask. Off by default. With it on, a thread that goes to make a Print your message didn't ask for puts a card up first. Saying print, chart, graph, dashboard or diagram counts as asking |
| Side pane | Open on a reply's card opens the Print in the thread's side pane. A new version replaces it there as soon as it's saved; an older version opened on purpose stays put |
| Updates in Ask mode | Once you allow one update to a Print, later versions of it from that thread go ahead without asking, until the thread restarts. A no isn't remembered as a yes |
| Keeping it fresh | "Refresh it every weekday morning" has the thread schedule itself to rebuild the Print from its recipe |
| On the website | A reply's Print block is a card. Published: title, kind, version and Open to its `/p/` page. Not published: says it's kept on the Mac |
| In Highlights | A thread's Highlights list the Prints it made, under Prints, each with its kind, version and the projects it's in, or "No project". A click opens it beside the thread |
| Prints on the rail | A Prints button on the right-hand rail, under Files, while Prints is on and at least one exists. It lists every Print, newest first, with its projects. A click opens it in the pane; Back returns to the list |
| Convex insights | "Make a Convex insights report" runs `npx convex insights --details --json` in the app's folder and builds a dashboard: insights, calls affected, errors, functions, calls by function and by kind, and each insight's table and last time seen. No insights gives a short all-clear report. Read only, last 72 hours, cloud deployments only |
| Convex API map | "Map my Convex API" runs `npx convex function-spec` and builds a dashboard of every function and HTTP route: public or internal, the arguments each takes, and missing validators, listed first. Reads metadata only |
| Todo lists and boards | `kind: "todo"` or `"board"` with Markdown. Click an item to tick it, or a card's arrow to move it; each click is a new version, so Versions undoes it. Edit Text edits the Markdown. Works in the side pane and a Print's window; published, it's a plain list. No scripts: each click is a link the app catches |
| Outside agents | A key with read-all access can `list_prints` and `read_print` published Prints, in bounded pages. Read only. Local Prints are never visible |

The agent side is the `create_print`, `update_print`, `read_print`, `list_prints`, `publish_print` family of tools. The older `*_page` names still work. The recipes ship with Threadbear as built-in skills in `runner/skills/`: make-print (charts, dashboards, reports, docs, progress, the Convex insights report and the Convex API map) and bundle-report (the Convex bundle size Print). Add them from the Skills page. make-print keeps the saved style in `~/.config/threadbear/print-style.json`.

## What's missing

- **Live data.** Live Prints can run scripts, but nothing pushes new data into an open one. The PRD's postMessage phase isn't built.
- **The project page still opens a window.** Cards, Highlights and the rail's Prints list open in the side pane. The project page has no thread beside it to open into.
- **Another Mac's Prints.** The rail's Prints list shows this Mac's Prints only, and hides for a thread on another runner.
- **Ask once on Codex.** Codex threads still ask for every update to a Print.
- **PDF page breaks.** The built-in PDF can cut a line across two pages. Asking the thread to print through Chrome avoids it.
- **The GPUI app.** The desktop specs in `features/prints.feature` are manual. The GPUI port has no Prints yet.
- **Upstream.** Sent to Mike as a draft PR, [mikecann/threadbear#61](https://github.com/mikecann/threadbear/pull/61), on 12 Nov: everything here except the recipe skills, Outside agents' Print reads and the iPhone app. Publishing is backend API 77 there. The recipes need a Skills store upstream first, so they're a follow-up PR.

## Not planned

- **Editable CSV sheets.** Dropped by Wayne on 12 Nov. The half-done work in the slot-12 worktree was never committed and isn't part of Prints.
- **The iPhone app.** Wayne isn't shipping his own iPhone app, and changes under `ios/` would clash with Mike's. Print blocks show as text on the phone. Leave `ios/` alone unless a feature can't work without it.

## Not yet seen on screen

- The side pane following a new version was checked through the debug app's state, which moved from v1 to v2 to v3. A screenshot of the redraw wasn't possible: the debug window sits behind other windows, and macOS stops a hidden web view from drawing.
- The website's Print card was checked by its tests and typecheck, not in a browser against a live backend.

## What's next

1. Check the insights report against a deployment with read-limit insights. Only write conflicts (`occRetried`) have shown up in real output so far, so the other kinds' counts are a best guess.
2. Ask once per Print on Codex threads too.
3. Group the rail's Prints list by project if it gets long.

The optional HyperFrames share clip stays parked until one of the above ships.
