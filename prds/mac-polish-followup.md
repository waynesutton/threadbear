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

2026-11-18 18:50 UTC: delivered. Product commit 87aece50 landed after the
required ./check.sh --quick passed in 582 seconds. The automatic updater
built the release, swapped it in at 18:48:37 UTC and logged it running at
18:48:38.999 UTC. Installed build.json names commit
87aece50f7ca2cc07df8e14c290643cc6a73faf7 and desktop tree
f45bdbb756fec9979ae7bed048e915d0ed0bbbca, which matches git's desktop tree.
All three crash fixes f3367a06, 8c1fc3c4 and 3d0779cd are ancestors.

The installed process (PID 90377) connected to the runner at 18:48:38.360 UTC
and remained running at 18:50:30 UTC. No new panic or hang record appeared
between restart and that observation. No draft was cleared and no restart
or installation was forced.

To verify the exact landed source after rebase, rebuilt the automation app
with a clean desktop diff against 87aece50. Five focused scenarios passed
all 89 steps: Print search/no results/Return/Escape/duplicate avoidance and
mouse selection; Notes size/font persistence; Bears and Tables opening,
settings/goals editing, Finder/copy and refresh. All isolated instances
were stopped. Build/spec logs: /tmp/tb-polish-installed-build.log and
/tmp/tb-polish-installed-specs.log.

The installed release has no scripted automation endpoint. Installed
verification above establishes build provenance, successful handover and
runner connection; interaction verification used the isolated app built
from the same desktop source. This is not a claim that every live app
interaction was exercised or that future crashes are impossible.
