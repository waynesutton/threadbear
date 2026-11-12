# Diagram Prints and outside-agent reads

Created: 2026-11-11 01:26 UTC

## Scope

Wayne wants diagram Prints and read-only access for outside agents. Editable CSV sheets are deferred; their uncommitted work in slot-12 stays untouched. This work starts from main in slot-15 and stays behind the existing Prints and Outside agents experiments.

## Diagram source and rendering

`create_print` and `update_print` accept `mermaid`, mutually exclusive with HTML, a folder or Markdown. The store renders static SVG using the same pinned Pele bundle as the app, without installing a package or reaching the network. Mermaid source is kept as `source.mmd` beside the self-contained HTML. A source size limit and render timeout bound the work. Invalid input must leave the prior version intact.

The Mac reuses Edit Text for Mermaid and saves through `pages.saveDiagram`. New versions, title changes, restore, export and publishing preserve the diagram kind and source. Diagrams remain static; Mermaid text is data and never executed as JavaScript.

Script permission is a Print-wide opt-in, preserved by updates and imports as it was before this feature. Converting an existing live Print to Mermaid does not revoke that permission or prevent restoring its prior scripted versions. Such a Print retains the existing restriction on publishing live Prints; create a new static diagram Print to publish separately. Newly created Mermaid Prints are static.

## Outside-agent access

Add `list_prints` and `read_print` to the existing authenticated MCP endpoint. Only published Prints are eligible; nothing new is mirrored from local storage. A key must have the existing `readAll` permission. Restricted keys receive no list and the same not-found result for all reads. Read keys can never create, update, publish or delete a Print.

Use the existing `prints` table and indexes, without schema changes. Listing is bounded and paginated in creation order; reading uses the unique published slug, so identical local ids on separate runners cannot collide. Responses contain public metadata and bounded HTML, no paths, recipes, thread text, credentials or internal runner metadata. Existing revocation, expiry and rate limits apply. Unpublishing removes access on the next call.

## Verification

- Runner tests first: valid and malformed Mermaid, escaping, no scripts or network content, limits, user edits, source preservation through versions and export, and static publishing.
- Backend tests first: discovery and read tools, restricted/read/write scopes, revocation, expiry, not-found responses, response bounds, pagination and unpublish. Existing thread authorization tests must still pass.
- Typecheck runner and Convex. Swift build and focused PageTests, TipsTests and EveryClickShowsTests.
- Inspect a rendered diagram and source editor in the debug app when available. Whole checks run one at a time through check.sh after focused checks.
- Update the protocol before implementation, then the guide, experiment docs and file/task notes. Do not deploy the backend, publish private data, push or open PRs without authorization.

## Card appearance

Added 2026-11-11 01:51 UTC: Wayne asked to remove the preview gradient and match Threadbear in light and dark mode. Keep the current layout and actions, use the existing solid native card surface and Pele palettes, and check both appearances in a debug app. Diagram preview recoloring must not rewrite the Print's source or exported files.

## Local verification status

2026-11-11 01:54 UTC, slot-15, not landed or deployed:

- 63 focused runner/backend tests pass: pages, diagram renderer, outside-agent backend and runner, experiments, generated experiment docs and context budget.
- Runner and Convex typechecks pass. The broader quick check also passed web typecheck and web tests; its runner suite is still running, logged at `/private/tmp/prints-slot15-check.log`.
- Swift build and 14 focused PageTests, RichBlockTests, EveryClickShowsTests and TipsTests pass.
- Diagram HTML inspected at dark desktop and light phone widths. Native diagram card, project row and Mermaid editor inspected against an isolated dev runner. A source save made version 2 and preserved its project assignment; restoring version 1 made version 3. Both light and dark cards inspected after the fade removal and palette change.
- Review found and fixed restoring HTML retaining an editable diagram kind; a regression test covers both directions. Passive SVG checks also reject SMIL and escaped CSS references.
- Temporary debug app and runner stopped. No real model turn, backend deployment or public publication was used for these checks. CSV work in slot-12 remains untouched.

2026-11-11 02:09 UTC: the broad run passed 2,586 tests and retried three failing skill-trust/process-cleanup files in isolation. Final review also caught live-HTML-to-diagram conversion revoking the Print-wide permission and preventing restoration; preserving that permission fixes it. All 22 page/renderer tests pass after the fix (64 focused runner/backend tests in total across the focused runs). Native card integration review found no further issues. Broad retry and landing remain pending.

## Integration with Mike's release

2026-11-11 04:17 UTC: rebased onto `005659b0`. Resolved two menu conflicts by keeping Mike's new icons while enabling Edit Text for diagrams. Regenerated experiment docs from the merged sources.

- Runner and Convex typechecks pass. A focused run of page, renderer, outside-agent, context budget, generated experiment docs, request parity and cross-route scenario tests passes all 71 tests.
- The rebased Swift build and 14 focused PageTests, RichBlockTests, EveryClickShowsTests and TipsTests pass, through the machine's build lock.
- `features/prints.feature` records three manual desktop flows: editing/version restore, malformed edits and no-project/project assignment. Their shared-driver automation and GPUI implementation remain pending. `features` pairing and Cucumber dry-run checks pass; no actual shared-driver UI scenarios were run.
- Added the Print fence and published-link scenario to the existing three-route backend harness. Local Print files are not newly mirrored; the portable text and link survive the relay and phone/web routes.
- Preserved the recorded GPUI parity results rather than replacing them with empty local reports. The added Print behavior is listed separately as pending.

Implementation is not on main yet. Its mandatory landing check, app release build and backend deployment remain pending. No launcher install, push or public publication was performed.

## Landed

2026-11-11: on main at `b221756c` and pushed to GitHub, after `./check.sh --quick` passed. Rebased onto the publishing recovery `e8dd6683`, which put backend API 73 back and is live. The three desktop specs stay manual until the GPUI port has Prints. What's left is in `prds/prints-status.md`.
