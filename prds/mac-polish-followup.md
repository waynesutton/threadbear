# Mac Tasks, Notes and Bears polish

Created 2026-11-18 18:06 UTC, before implementation.

## Request and cause

The Tasks list appends its first row directly after the status header. The
Attach Print menu lists every Print without search. Notes currently offers
Sans and Serif. Bears & Tables has only rename/delete in its context menu,
although opening and file/project navigation already exist elsewhere.

## Delivery

- Add a visible non-clickable gap above the first task in expanded sections.
- Search Prints by title when attaching one, retaining normal selection,
  keyboard navigation, empty results and dismissal.
- Offer only Sans and Mono in Notes. Keep size controls, line numbers,
  editing and saving; old Serif preferences fall back to Sans.
- Add Open, Edit, Show in Finder and Refresh where applicable to Bears and
  Tables, using existing navigation/editing APIs. Keep rename/delete and
  multi-selection. Avoid synchronous nested app/pane updates.
- Include the already-landed panel navigation and rootless-window crash
  fixes in the tested build. No Windows/Linux work or upstream submission.

## Verification

Run focused Tasks, Notes and Bears scenarios against the automation build;
inspect dark/light screenshots and first-row hover spacing. Check menu
actions, Print search/select/no results, font persistence and navigation
without panics. Run the required landing check, then await the automatic
installed app update. Keep installed provenance and isolated interaction
evidence separate if live automation is unavailable. Never clear drafts or
force-restart the installed app.

## Status

2026-11-18 18:17 UTC: implemented. The task gap is 10 points outside both
click targets. Print search matches titles without case sensitivity; mouse
selection and Return attach, already-attached results cannot be added twice,
and Escape preserves the open task. Notes uses the existing app Mono font.
Bears Edit Settings opens bear.json in the safeguarded file editor; Edit
Table opens its project goals. File navigation is deferred outside the app
update to avoid recursive entity access.

Validation: GPUI automation build passed; 41 existing Tasks/Notes/Bears
scenarios passed, and the expanded Print search scenario passed all 29 steps
after fixing immediate reopening and test text visibility. The focus guard,
94 feature/step pairings, mock syntax and diff checks passed. Dark/light
screenshots were inspected, including task spacing, search/no results, Mono
Notes and Bears menus. All test interactions completed without a panic and
the isolated app was stopped. Screenshots are private local fixtures under
.fixtures/mac-polish-followup/.

Required landing check and automatic installed verification remain pending.
The existing crash fixes f3367a06 and 8c1fc3c4 are ancestors of this build;
this does not claim the whole app is crash-free.
