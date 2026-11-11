# Where Prints stand

Updated 2026-11-11. On main at `b221756c`, pushed to GitHub. Prints are still the `pages` experiment, off by default, on for Wayne. The full user-facing walkthrough is the Prints section of `docs/guide.md`. This page is the short version plus the gap list.

## How Prints work

A Print is a page a thread makes for you to keep. Ask for a chart, a dashboard, a report, a doc or a diagram in plain words. Asking again updates the same Print to a new version instead of posting a second one.

| Piece | How it works |
|---|---|
| Storage | Files on your Mac in `~/Library/Application Support/Threadbear/pages/<id>/v<n>/`. Last 10 versions kept. Nothing goes to Convex unless you publish |
| Kinds | chart, dashboard, report, doc, diagram, bundle |
| Projects | A thread pinned to one project files its Prints there. A Print can sit in several projects, or none |
| In a reply | A card with a preview, title, kind, version and age. It follows the newest version and says "New version" |
| Project page | Prints are listed with Open, Refresh, Edit with Agent, Edit Text, Download, Download as PDF, Versions, Projects, Publish and Delete |
| Diagrams | Mermaid source drawn offline with the bundled Pele 1.13.0 into static SVG. Edit Text edits the source. Bad source keeps the old version |
| Safety | No scripts by default. A live Print runs scripts only after you choose Run them, and again per version |
| Delete | Soft delete. Restore within 7 days from Recently deleted |
| Publishing | Opt in per Print. Backend API 73 stores one static `index.html` under a link. It opens on the phone, the website's `/p/<slug>` page and for anyone with the link. New versions follow. Unpublish or Delete takes it down |
| Outside agents | A key with read-all access can `list_prints` and `read_print` published Prints, in bounded pages. Read only. Local Prints are never visible |

The agent side is the `create_print`, `update_print`, `read_print`, `list_prints`, `publish_print` family of tools. The older `*_page` names still work. The make-print skill holds the chart, dashboard and report recipes. The bundle-report skill builds the Convex bundle size Print.

## What's missing

- **Print cards on the iPhone and the website.** A published Print opens as a link in the browser. Neither app draws the card yet.
- **Editable CSV sheets.** Deferred by Wayne. The half-done work sits uncommitted in the slot-12 worktree.
- **Live data.** Live Prints can run scripts, but nothing pushes new data into an open one. The PRD's postMessage phase isn't built.
- **Opening in the side pane.** The project page opens a Print in its own window. The thread's side pane doesn't take it, and doesn't reload on a new version.
- **An "Only make Prints when I ask" setting.** Today an agent can make a Print whenever it thinks one helps, like a progress Print on a long task. There's no switch to keep Prints available but make them wait for your say-so.
- **One approval per Print.** In Ask mode, `update_print` asks every time, even from the thread that owns the Print.
- **A global Prints view.** Prints with no project are reached through their card or by asking "show my prints with no project".
- **PDF page breaks.** The built-in PDF can cut a line across two pages. Asking the thread to print through Chrome avoids it.
- **The GPUI app.** The three desktop specs in `features/prints.feature` are manual. The GPUI port has no Prints yet.
- **Upstream.** All of this is in Wayne's fork. Nothing has gone to mikecann/threadbear.

## What's next

1. Add an "Only make Prints when I ask" setting. With it on, an agent's `create_print` puts a card to you first ("Make the print X?") unless your message asked for one. In progress: prds/prints-only-when-asked.md.
2. Draw published Print cards on the iPhone and the website, since the backend already serves them.
3. Open Prints in the thread's side pane and reload it on a new version.
4. Ask once per Print for updates from its own thread.
5. Pick the CSV sheets work back up from slot-12, or drop it.
6. Write scheduled refresh into the make-print skill and the guide. No code needed.
7. Decide whether Prints stays an experiment or becomes standard, at the check-in booked for 14 Nov.

The optional HyperFrames share clip stays parked until one of the above ships.
