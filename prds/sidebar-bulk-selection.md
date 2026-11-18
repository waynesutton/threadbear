# Sidebar selection and Print search

Created 2026-11-18 UTC, before implementation.

Wayne wants selecting several Prints, Bears or Tables to be as discoverable as selecting notes in Apple Notes. Existing Command/Shift selection is hidden and the long Prints list has no search.

## Delivery

- Keep normal row clicks opening items. Add Select/Done, selection circles, Select All, a selected count and fixed Archive/Delete controls. Keep Command-click and Shift-click, with Escape to clear.
- Keep Prints, Bears and Tables separate. Bears and Tables each have their own search, selection and action row in their existing panel.
- Search each section by title; filter before range selection or Select All. Changing the search or archive filter clears selection so invisible items are never acted on.
- Prints: Archive hides from the active sidebar on this Mac, persisted in native settings. Archived view offers Restore. Files, projects and published links stay intact. Delete retains the existing confirmation and seven-day recovery.
- Bears/Tables: use existing archive APIs, with one confirmation explaining bear routine/webhook/table-seat consequences. Archived items remain accessible for unarchiving. Keep existing menus and delete confirmation.
- Use deferred entity updates where a panel action reads the app, preserving the recent GPUI crash fixes. No forced restart, live-draft changes or other platform work.

## Verification

Focused pure selection tests and isolated native scenarios: visible selection mode, range and select-all, search with empty results, hidden selection cleared, archive/restore, delete cancellation, failure feedback and repeated panel navigation. Inspect screenshots at the normal sidebar width. Run the required landing check, then confirm installed build provenance and logs after automatic update. Keep release provenance separate from headless interaction evidence.

## Status

2026-11-18 21:09 UTC: product landed and installed; verification complete.

- Native automation build and final native typecheck pass; the keyboard/AppKit focus guard passes.
- Prints, Bears, Notes and Tasks: 60 isolated scenarios / 832 steps pass. Two additional context-menu isolation scenarios / 33 steps pass.
- Covered independent selections, title search and empty results, mouse and keyboard select-all, search clearing hidden selection, archive/unarchive, partial archive failure with retryable failed selection, deletion cancellation and filtered deletion. Menus cannot include hidden rows selected on another surface.
- Dark and light screenshots inspected at the standard sidebar width. Examples retained locally in `.fixtures/sidebar-bulk/` and pinned for Wayne.
- Mock syntax, 94 feature/step pairings and whitespace checks pass. All dev instances stopped. No crash observed during these isolated interaction checks; installed provenance is recorded separately below.
- Prints archive is local native state, persisted in the existing settings file. No new backend schema, network service or background job.

## Installed evidence

- Product `f405c0de587583167315fefeef0dd0e4c85755fb` landed after the required 570-second quick check passed (`/tmp/tb-sidebar-land.log`).
- The app was still running `87aece50` when Wayne reported missing changes. Its ten-minute build debounce restarted when the app relaunched at20:53UTC. Started the supported staging script at20:56UTC, without forcing a live restart or altering drafts.
- Release build finished in10m05s and staged at21:06:01UTC (`/tmp/tb-sidebar-stage.log`). Automatic handover began at21:08:27.071UTC; the new app connected at21:08:28.331UTC and took over at21:08:28.960UTC.
- Installed `~/Applications/Threadbear GPUI.app/Contents/Resources/build.json` names `f405c0de` and desktop tree `414474e2e1222af86169f2dfdbdbba3425837721`, equal to `f405c0de:desktop`. Crash fixes `f3367a06`, `8c1fc3c4` and `3d0779cd` are ancestors.
- Live process32752 and the runner connection are confirmed in `~/Library/Logs/Threadbear/desktop.log`. No new panic or hang report through21:09UTC: newest panic is17:13UTC, newest hang09:46UTC. The crash-report bookkeeping file was excluded from panic counts.
- The installed release has no automation interface. Installed verification establishes source provenance, successful handover, connection and observed log health; the62scenarios/865steps and inspected screenshots establish isolated interaction behavior. The last source-only change after that automation build was the singular `1 item` count label, followed by a passing native typecheck.
