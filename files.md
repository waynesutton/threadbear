# Files

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
