# Collapsed scheduled follow-ups

Created: 2026-11-17T07:28:13Z

## Goal

Scheduled continuation prompts should not fill the conversation with internal instructions. Wayne requested a collapsed heading with Show details.

## Scope

GPUI transcript only. Scheduled user messages start collapsed, retaining schedule title and time. Show details reveals selectable, left-aligned Markdown; Hide details closes it. Use stable row IDs and the existing local expansion state, with no animation or protocol/data changes. Runner restart notes and ordinary messages retain their current presentation. Empty scheduled bodies need no disclosure. Other clients continue to receive and display the original message; no backend or phone changes.

## Verification

Native behavior scenario: default collapsed; show and hide details with height changing; complete original text remains in the model. Check multiple messages independently and ordinary messages unchanged. Render actual build and inspect screenshot; check native compilation and normal landing. No package installation, push, cloud deployment or forced app restart.

## Status

2026-11-17T07:39:54Z: implemented using existing row expansion and Markdown/link handling. Native automation all-targets check and debug build pass. Actual Show/Hide clicks, row-height change, original text retention, independent notes, unchanged runner/phone messages, empty bodies and malformed headers pass through the headless driver. Screenshots inspected in ignored `.fixtures/scheduled-followups/closed.png` and `open.png`; owned dev app/mock stopped. All 22 cross-route scenarios pass, including a scheduled-origin/full-body integrity scenario. Pairing, generated native parity and syntax checks pass. Cucumber is unavailable locally; equivalent direct native commands were exercised. Commit and normal landing remain.

Future scheduled instructions in this thread should be short and point to this PRD rather than repeat a dense handoff.


2026-11-17T09:25:17Z: feature is live. Normal landing passed in689 seconds and main contains1dcad0fb. Automatic release compilation completed after63m09 under competing build load, staged at09:08:46, and automatically swapped/restarted with nothing typed. Installed build.json and actual app process19187 match1dcad0fb; desktop.log records app started09:08:47 and09:14:56. No forced restart or draft edits.

Exported this real thread (1193 events,15 scheduled messages total) and used the existing cached scripted native reader without rebuilding. All8 scheduled rows in the loaded page start collapsed with Show details. Expanding real messageu:1179 retains its original text; after scrolling its header into view, the actual Hide details click closes it. Earlier direct fixture checks cover independent notes, empty/malformed headers, Markdown and unchanged runner/phone messages. Actual screenshot and dumps are in ignored `.fixtures/scheduled-installed/` in borrowedslot20; owned test app/mock stopped. This is installed-version/source and exported-data verification, not live-pointer automation.

Final documentation uses borrowed worktree /Users/waynesutton/threadbear-threads/slot-20, because slot12 was handed to another thread and main is shared. The required borrow helper automatically bootstrapped its locked runner dependencies; no package manifest was changed and no additional install was requested. Only task.md and this PRD change for the final docs landing. Give the borrowed worktree back after landing. No push, cloud deployment or phone changes.
