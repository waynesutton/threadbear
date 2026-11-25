# Files

## Terminal shortcut

- `desktop/gpui/src/terminal.rs`: shell sessions and Terminal controls, including focus when the panel hides.
- `features/terminal.feature`: Terminal behavior, repeated shortcut toggling and retained shell state.
- `prds/terminal-shortcut-reopen.md`: scope and verification for reopening Terminal from the keyboard.

## Cloud runners setup

- `runner/src/cloud-setup.ts`: sets up a Fly virtual runner from one token (flyctl, the Fly app, backend env, box image, GitHub variables, the runner row, a test box), and checks or rebuilds it; `cloud.*` RPC methods in `runner/src/main.ts`, `tb cloud` in `runner/src/cli.ts`.
- `runner/src/fly-cost.ts`: this month's cost of a Fly virtual runner's boxes, from each Machine's start and stop events at Fly's list price (`cloud.cost`), shown as This month in Settings > Runners.
- `runner/test/unit/cloud-setup.test.ts`: the steps against a fake Fly API, flyctl, gh and backend.
- `desktop/core/src/cloud_runners.rs`: `cloud.status` read into rows, and the section's and sheet's words.
- `desktop/gpui/src/settings_window/cloud.rs`: Settings › Runners › Cloud runners and the Add a cloud runner sheet.

## November release verification

- `prds/bring-in-2026-11-20-verification.md`: remaining merge repairs, verification and landing review.
- `runner/test/deploy/deploy.test.ts`: isolated runner installation and deployment lifecycle tests.
- `runner/test/unit/rpc.test.ts`: RPC behavior, including sign-in discovery and Usage refresh compatibility.
- `desktop/core/src/highlights.rs`, `desktop/core/src/reply_links.rs`: file preview classification and click routing, including exact diff and patch regression cases.

## Reminder preferences

- `prds/loose-end-pr-preferences.md`: owner-specific automatic PR reminder preferences and rollout verification.

## Sidebar selection and search

- `desktop/gpui/src/sidebar_lists.rs`: separate Print/Bear/Table search, visible selection, bulk archive/restore controls and native confirmations.
- `prds/sidebar-bulk-selection.md`: scope and verification record for sidebar search and bulk actions.

## Scheduled follow-up details

- `desktop/gpui/src/rows.rs`: collapsed scheduled headings and selectable Markdown details using existing row expansion state.
- `features/reading-a-thread.feature`: scheduled disclosure behavior alongside transcript reading checks.
- `prds/collapsed-scheduled-followups.md`: scope, display-only behavior and verification checkpoint.

## Clearer thread completion

- `runner/src/completion.ts`: bounded task updates merged by stable ID across turns.
- `runner/src/experiments/clearerCompletion.md`: opt-in completion card and removal guidance.
- `desktop/core/tests/completion_tests.rs`, `runner/test/unit/completion*.test.ts`: reducer, persistence, compatibility and mirror checks.
- `web/src/completion.ts`, `web/src/completion.test.ts`: task summary labels and heading checks.
- `prds/clearer-thread-completion.md`: approved appearance, contract and verification state.

## Realtime Tasks

- `convex/tasks.ts`, `convex/taskFields.ts`: authenticated cloud-owned tasks and synced preferences.
- `convex/tasksLinear.ts`: a task's Linear issue: send claim and settle, link, import, sync apply and the one-Mac sync sweep claim.
- `runner/src/tasks.ts`: desktop/agent bridge and realtime invalidation.
- `runner/src/linear-tasks.ts`: Tasks and Linear through the Connectors gateway: send (with picture uploads), search, import, link, refresh and the timed sync sweep.
- `desktop/gpui/src/tasks_page.rs`: native Tasks list, scrolling/wrapped board, detail navigation and photo sliders.
- `desktop/gpui/src/task_detail.rs`: task properties, calendar, sub-tasks, attachments and activity UI.
- `desktop/gpui/src/task_keys.rs`: Tasks' Linear-style keys on the list and board, the status, priority and due date quick picker, and Favorites (list section, board column, star).
- `desktop/core/src/task_dates.rs`: priority labels, calendar grids, due-date labels and date tests.
- `web/src/tasks.tsx`, `web/src/tasks.css`: authenticated responsive Tasks route.
- `web/src/tasks-issue.tsx`: browser task properties, attachments, sub-tasks and activity editor.
- `web/src/tasks-fields.ts`: task priority/date labels and validated field helpers.
- `web/src/tasks-fields.test.ts`: date, priority and field-helper regression checks.
- `web/src/tasks-columns.ts`: stable column identity, names and ordering shared by web list and board.
- `web/src/tasks-draft.ts`, `web/src/tasks-draft.test.ts`: field-specific edits that preserve concurrent changes.
- `web/src/tasks-order.ts`, `desktop/core/src/task_order.rs`: where dropped tasks, columns, list sections and project groups land, and how a quick-added line splits into title and note. Same rules on both, each with tests.
- `runner/test/unit/tasks*.test.ts`, `features/tasks.feature`: backend/bridge and native behavior checks.
- `prds/realtime-tasks.md`: scope, delivery constraints and verification checkpoint.
- `prds/mac-tasks-prints-polish.md`: both Mac polish batches and remaining installed verification.

## Mac Prints and composer polish

- `desktop/gpui/src/print_select.rs`: multi-selection, bulk project assignment and keyboard-aware delete confirmation.
- `desktop/gpui/src/composer.rs`: attachment thumbnails that wrap and scroll after three rows.
- `runner/skills/make-print/theme.mjs`, `charts.mjs`, `progress-view.mjs`: shared Print themes, charts and progress layouts.
- `runner/src/page-doc.ts`: themed editable document rendering.
- `web/src/print.tsx`, `web/src/print-doc.ts`: published Print theme selection and sandboxed document styling.

## GPUI Looks

- `desktop/core/src/looks/` the Librarian, Scheduled and Projects pages' looks in the Swift app's JSON, the background's focus and crop, the greeting, GitHub contributions, Threadbear activity, the quote of the day and the profile page made from the Librarian's look.
- `desktop/gpui/src/looks.rs` the pictures in `looks/` (import, sizes, removal), the background photo and its Position preview, the Customize panel's shared sections, and the app's look settings.
- `desktop/gpui/src/librarian_look.rs` the customized Librarian page and its Customize panel.
- `desktop/gpui/src/scheduled_look.rs` the customized Scheduled page and its Customize panel.
- `desktop/gpui/src/projects_look.rs` the customized Projects page and its Customize panel; `projects_page.rs` drops the card parts it hides.
- `desktop/gpui/src/look_cards.rs` the GitHub and Threadbear activity calendars.

## GPUI Notes

- `desktop/core/src/notes.rs` notes as the Swift app keeps them: sealing (AES-256-GCM with `ring`), names, Markdown spans, list typing and the files.
- `desktop/gpui/src/notes_page.rs` the Notes page, Sans/Mono fonts and sizes, encrypted editor, delete question and scripted controls.
- `features/notes.feature` and `features/steps/notes.steps.ts` Notes scenarios.
- `prds/gpui-notes.md` scope and verification for the port.
## GPUI Voice

- `desktop/core/src/voice.rs` what's heard and said: the utterance tracker, spoken text, spoken answers and approvals, the voice catalog, the button icons, saved prefs and dictated text.
- `desktop/core/src/voice_keys.rs` the Fn key, its options and the machine that turns presses into dictation.
- `desktop/core/tests/voice.rs` the Swift app's Voice tests, ported.
- `desktop/gpui/src/voice/` the Voice entity (`mod.rs`), Voice mode (`mode.rs`), Fn dictation (`keys.rs`), the button and pill (`button.rs`) and the helper process (`helper.rs`).
- `desktop/gpui/src/settings_window/voice.rs` Settings > Voice & Audio for thread Voice, keyboard dictation and the pause before sending.
- `desktop/gpui/native/voice_helper.swift` the microphone, SpeechAnalyzer, playback and pocket-tts, as a helper program the app carries.
- `desktop/gpui/native/voice_keys.m` the app-local key monitor and typing into the focused box.
- `desktop/gpui/native/voice_build.rs` builds both for build.rs.
- `features/voice.feature` the Voice scenarios, checked by hand.

## GPUI Settings

- `desktop/gpui/src/app_update.rs` holds automatic restarts while an Instructions edit is unsaved.
- `app/Tools/mock-runner/server.mjs` includes one-shot Instructions save rejection for the Settings regression.

- `desktop/gpui/src/palettes.rs`, `desktop/gpui/themes/harbor-squall.theme.json` the Harbor and Squall themes' colours: the app's own, and the same values in the kit's theme format. `style.rs` picks between them and Stock.
- `desktop/gpui/src/settings_window.rs` the main-window Settings page, section navigation and pending instruction saves.
- `desktop/gpui/src/shell.rs` Settings hosting and Back/Close behavior.
- `features/settings.feature` and `features/steps/settings.steps.ts` Settings window identity, navigation and control scenarios.
- `prds/gpui-settings-in-main.md` scope and verification for embedded Settings.

## GPUI Connectors

- `desktop/core/src/connectors.rs` connector status, catalog and agent types, masked form fields, local pin validation and hidden-row rules.
- `desktop/gpui/src/connectors_setup.rs` setup and add actions, outside-agent forms and revoke, local trust sheet and hidden-row menu.
- `desktop/gpui/src/connectors_panel.rs` service rows, panel status and automation dump.
- `desktop/core/tests/connectors_setup.rs` pure rule checks for keys, permissions, local commands and hidden rows.
- `features/connectors.feature` panel flows against the scripted desktop app.
- `prds/gpui-connectors-setup.md` the Swift-to-GPUI port scope and checks.

## Bears, tables and webhooks

- `desktop/core/src/bears_panel.rs`: native bear/table rows, selection and bulk-operation rules.
- `desktop/gpui/src/bears_panel.rs`: Bears & Tables panel, open/edit/Finder menus, rename and confirmed deletion controls.
- `runner/src/bears.ts` bear folders, access state, shared worker lookup, managed memory rules, tidy refresh and the `bears.*` RPCs.
- `runner/src/mentions.ts` routes mentions within mode and sandbox boundaries and matches replies to each asking turn.
- `runner/src/runner/quiet-inbox.ts` checks unfinished work before archiving helpers or explicitly empty unattended bear runs.
- `runner/test/unit/bears-always-on.test.ts` integrated routine, webhook and mention routing with replies matched to queued turns.
- `prds/bears-always-on.md` the five runner improvements, safeguards and verification status.
- `prds/bears-resources.md` source links supplied for bears, what they informed, and supporting implementation references.
- `runner/src/tables.ts` tables as marked projects, their host folder and the `tables.*` RPCs.
- `runner/src/webhooks.ts` webhook runs on the runner, its hook registry and the `webhooks.*` RPCs.
- `runner/src/cli-bears.ts` `tb bears`, `tb tables` and `tb webhooks`, and the note that Auto works like Ask on a Codex bear.
- `app/Sources/ThreadbearCore/Protocol/Bears.swift` a thread's bear for its sidebar tile, the @ list's bears, and the colour and list calls.
- `app/Sources/Threadbear/AppModel+Bears.swift` Bear Colour and the @ list's cached bears.
- `app/Sources/Threadbear/Views/BearPicker.swift` the composer's @ list of bears.
- `app/Tests/ThreadbearAppTests/BearPickerTests.swift` what the @ list offers and when it stays shut.
- `features/bears.feature` the bear tile, Bear Colour and @ list, checked by hand.
- `runner/skills/bears/SKILL.md` how agents make, run and give faces to bears, routines and team templates.
- `runner/skills/bears/avatars/*.json` six bear-shaped Open Bot Avatar presets.
- `runner/src/experiments/{bears,webhooks,tables,bearNotes}.md` the four experiments.
- `convex/webhooks.ts` webhook keys, delivery limits, dedupe, batching and pruning.
- `convex/hookHttp.ts` the `/hook/<key>` route's status decisions.
- `runner/test/unit/{bears,mentions,tables,bear-notes,webhooks,webhooks-backend}.test.ts` their tests.
- `prds/bears-and-webhooks.md` the plan as built.

## Runner checks

- `.github/workflows/desktop.yml` runs desktop tests, behaviour specs, release builds and artifact uploads with 60-minute Linux/Mac and 90-minute Windows limits.
- `runner/test/unit/net-probe.test.ts` checks proxy deadlines and response-size limits with controlled time, plus real TCP and TLS connections.
- `runner/package.json` caps the updater's npm unit suite at four workers, with failed files still rerun one at a time.

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

## Todo lists and boards

- `runner/src/page-list.ts` parses, draws, ticks and moves todo lists and boards from Markdown.
- `app/Sources/ThreadbearCore/Protocol/Page.swift` has `PrintLink`, the `threadbear-print:` click.
- `app/Sources/Threadbear/Pages.swift` has `handlePrintLink`; `Views/PanelsView.swift` catches the link in a Print's view.

## Embedded panel appearance

- `desktop/web/src/mac/panel.rs` hosts live Chromium panels and applies app appearance without reloading.
- `desktop/gpui/src/web_panels.rs` keeps panel instances and synchronizes their appearance.
- `features/web-panels.feature` covers panel navigation and appearance changes.
- `prds/prints-navigation-and-panel-theme.md` tracks Prints navigation and panel appearance verification.

- `prds/mac-polish-followup.md`: Tasks spacing and Print search, Notes fonts, Bears menus and crash verification.
