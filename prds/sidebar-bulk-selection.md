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

2026-11-18 20:33 UTC: implementation and local verification complete; landing and installed verification remain.

- Native automation build and final native typecheck pass; the keyboard/AppKit focus guard passes.
- Prints, Bears, Notes and Tasks: 60 isolated scenarios / 832 steps pass. Two additional context-menu isolation scenarios / 33 steps pass.
- Covered independent selections, title search and empty results, mouse and keyboard select-all, search clearing hidden selection, archive/unarchive, partial archive failure with retryable failed selection, deletion cancellation and filtered deletion. Menus cannot include hidden rows selected on another surface.
- Dark and light screenshots inspected at the standard sidebar width. Examples retained locally in `.fixtures/sidebar-bulk/` and pinned for Wayne.
- Mock syntax, 94 feature/step pairings and whitespace checks pass. All dev instances stopped. No crash observed during these isolated checks; this is not yet an installed-build verification.
- Prints archive is local native state, persisted in the existing settings file. No new backend schema, network service or background job.
