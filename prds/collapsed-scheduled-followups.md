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
