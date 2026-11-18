# Respect unwanted PR reminders

Created2026-11-18 UTC before implementation.

Wayne asked repeatedly not to see reminders about PRs awaiting Mike. Agent reply preferences do not control the automatic Still open card. The runner generates its unmerged entries independently in loose-ends.ts.

## Change

Add optional runner.json `looseEndsIgnoredPrOwners`, a list of GitHub repository owners whose matched PR reminders are omitted. Scope by the destination repository owner, case-insensitively; keep uncommitted, unlanded and unpushed warnings, other owners' PRs and unknown branch warnings. No repository owner is hardcoded. Set Wayne's preference to `mikecann` in his own configuration. Preserve the default for other copies.

Explain the screenshot's six entries from the thread's recorded commit IDs and freshly fetched remote refs. The slot name is the first related checkout scanned, not the location of six unfinished features. Do not push merely to remove a warning.

## Verification

Test ignored upstream and origin PR owners, case normalization, unrelated-owner/default behavior and retained unpushed warnings. Run focused tests and runner typecheck, required landing check, then verify the new live runner recomputes this thread's card without the unwanted entry. Preserve other configuration and existing work.

## Status

2026-11-18 21:32 UTC: implementation complete, 14 focused tests and runner typecheck pass. Preference saved in Wayne's runner.json. Landing, runner rollout and live-card verification remain.

The six screenshot entries, confirmed against fetched origin refs: 1eae75f (rollout record), 3914ecf (installed verification record), 87ad1b4 (task search and native menus, rebased as87aece50 on main), 136a4db (follow-up record), f405c0d (sidebar selection and search), dc3c0d4 (sidebar verification record). Their changes are on local main; no remote branch contains these recorded IDs.
