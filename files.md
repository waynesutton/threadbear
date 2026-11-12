# Files

## Runner checks

- `runner/test/unit/net-probe.test.ts` checks proxy deadlines and response-size limits with controlled time, plus real TCP and TLS connections.
- `runner/package.json` and `check.sh` cap unit-test workers for updates and landing at four, with failed files still rerun one at a time.

## Diagram Prints

- `runner/src/page-diagram.ts` renders Mermaid as static SVG and themed HTML with source limits, render timeouts and passive SVG checks.
- `runner/skills/pele/pele.js` is the app's pinned Pele 1.13.0 bundle reused by the runner without a package install.
- `runner/skills/pele/pele-LICENSE.txt` is the bundled renderer's MIT license.
- `runner/test/unit/page-diagram.test.ts` checks renderer reuse, source-as-data handling and rejection of active or externally referenced SVG.
- `runner/src/pages.ts` keeps Print versions and editable Markdown or Mermaid source.
- `app/Sources/Threadbear/Pages.swift` loads and saves doc or diagram source through the shared text editor.
- `app/Sources/Threadbear/Transcript/PageCards.swift` draws solid Print cards and their existing actions.
- `app/Sources/Threadbear/Transcript/RichRenderer.swift` applies the app's native palette to diagram card previews without changing their files.
- `features/prints.feature` records diagram editing, invalid edits and project assignment as manual desktop behavior specs while GPUI support is pending.
- `features/steps/prints.steps.ts` owns the manual Print specs until the shared driver supports the local Print store.
- `runner/test/helpers/scenario-list.ts` includes the diagram Print reference and published link in cross-route transcript checks.

## Outside-agent Print reads

- `convex/mcp.ts` checks keys and scopes, lists published Print metadata and reads bounded public HTML slices.
- `convex/mcpRpc.ts` defines and dispatches read-only Print tools on the authenticated MCP endpoint.
- `convex/http.ts` wires the endpoint to internal backend functions.
- `runner/test/unit/outside-agents-backend.test.ts` verifies authorization, pagination, response bounds, revocation and unpublishing.

## Prints docs

- `prds/prints-status.md` says how Prints work today, what's missing and what's next.
- `changelog.md` lists what shipped in Prints by day.
- `prds/prints-only-when-asked.md` is the plan for the Only make Prints when I ask setting.

## Prints only when asked

- `runner/src/threadbear-mcp.ts` has `asksForPrint` and the card `create_print` puts up when the setting is on.
- `runner/src/settings.ts` and `runner/src/config.ts` keep `printsWhenAsked` in runner.json.
- `app/Sources/Threadbear/Views/SettingsView.swift` shows the switch under Prints while the Prints experiment is on.

## Prints round two

- `runner/src/adapters/claude/adapter.ts` remembers Prints a session may update without asking again (`printGrants`).
- `app/Sources/Threadbear/Pages.swift` opens a card's Print in the side pane and moves the pane to new versions.
- `app/Sources/ThreadbearCore/Protocol/Page.swift` has `PagePath`, which reads a Print file's id and version and finds the newer one.
- `web/src/print-fence.ts` reads a reply's Print block; `web/src/print-card.tsx` draws its card from the published list.

## Prints in Highlights and on the rail

- `app/Sources/ThreadbearCore/Protocol/Page.swift` has `PrintRows`: a thread's Prints, every Print, and the projects line.
- `app/Sources/Threadbear/Views/HighlightsView.swift` has the Prints section and `PrintRow`.
- `app/Sources/Threadbear/Views/PanelsView.swift` has the rail's Prints button and `PrintsList`.

## Print recipes as built-in skills

- `runner/skills/make-print/` builds Prints: charts, dashboards, reports, docs, progress, the Convex insights report (`insights.mjs`) and the Convex API map (`apimap.mjs`).
- `runner/skills/bundle-report/` builds the Convex bundle size Print.
