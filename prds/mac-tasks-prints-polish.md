# Mac Tasks and Prints polish

Recorded 2026-11-18T10:33:00Z after implementation, during rollout. This is a delivery record of the approved work, not a claim that a PRD preceded it.

## Scope

Make the Mac Tasks, Bears & Tables, Prints and Notes controls easier to use. Keep existing data and permissions, and retain compatibility with older clients. Backend and website changes support the shared Tasks fields and published Prints. No new Windows, Linux or iPhone work.

## Delivered behavior

- Tasks has photo fade and position sliders, an icon for appearance controls, an explicit List/Board default, more space around list completion controls, taller board entry fields, horizontal board scrolling and a local Wrap Columns option.
- The native Bears & Tables panel supports rename, right-click actions and multi-selection deletion. Prints supports multi-selection, project assignment and confirmed deletion; web links open in the browser. Return confirms Print deletion and Escape cancels it.
- Notes uses readable Sans or Serif text with size controls while retaining line numbers.
- Tasks opens a detail page with status, priority, due date, project, description, one-level sub-tasks, links, images and activity/comments. Related project threads and Prints appear there; Highlights can add a task. List rows and cards show priority and date chips. API 87 adds optional fields and operations while preserving existing callers.
- Generated Prints have updated light/dark layouts, chart tiles and report sections. Published Prints offer System/Light/Dark selection. Composer pictures wrap and scroll after three rows.

## Verification and rollout

Batch one landed through db62f91d. The installed GPUI build metadata and updater log confirmed that build on 18 November.

Batch two landed as 96084760, ce4d102c and 01feefdd after the required quick check passed in 650 seconds. Its runner suite passed 4,146 tests with five skipped and no failures. Native core/client tests, GPUI automation compilation, runner/Convex/web typechecks, web tests, feature pairing, site build and mock checks passed in that landing. The preceding combined behavior run passed 39 scenarios and 513 steps; dark/light task-detail and Prints screenshots were reviewed.

The backend and website deployment records both show 01feefdd at 2026-11-18T10:32Z. Live API 87 checks passed priority/date/link saves and reads, sub-task relationships, comment/activity, comment removal and cascading deletion. Both temporary verification tasks were deleted. Runner release 20261118T104205Z-01feefdd is running; its get, activity and related RPCs passed live checks.

## Installed checkpoint

2026-11-18T11:08:00Z: installed GPUI build.json reports 1eae75ff, release configuration, built at 11:02:31 UTC. Git ancestry includes 01feefdd; both commits have desktop tree 90f594ef2e086c506944843b1bf5c4eb24390e0c. The updater and desktop logs confirm automatic handover at 11:02:34 UTC, with the new app connected to runner 01feefdd. The installed process was running from the expected app bundle. No new hang or crash report appeared after the restart.

The release does not expose the scripted control, and Accessibility returned no window. Interaction verification used the matching headless automation build on an isolated mock runner, not the user's live data. The focused run passed rendered task properties; priority/due-date saves; sub-task relationships; comment/activity; links; calendar/clear; wrapped columns; Highlights Add to Tasks; Print deletion with Escape/Return; and opening the eighth of eight wrapped composer pictures. New detail/composer screenshots were inspected. The test instance was stopped; no live draft was cleared and no app restart was forced. Local test evidence is ignored under `.fixtures/mac-polish/`.

The composer-wrap upstream PR remains a local draft. Extract only its composer changes from the combined Prints commit if submission is later requested.
