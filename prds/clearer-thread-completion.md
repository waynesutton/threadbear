# Clearer thread completion

Created: 2026-11-16T21:42:56Z

## Goal

Wayne selected clearer completion for threads with multiple tasks. Show the whole request's progress above the composer and keep Archive manual.

## Design review

Use the existing composer-attached card, native system typography, subdued dark surfaces and green only for completed work. Thread vocabulary: request, turn, task, loose end, check-in, Archive. Keep the task ledger and runner checks visibly distinct. Avoid a separate dashboard, progress ring or transcript-only card.

Wayne approved the neutral action buttons with amber left edges. Example data in the design review is illustrative, not an implementation report.

## Proposed contract

An off-by-default clearerCompletion experiment adds a typed task summary to end_turn. Stable task IDs preserve the whole request across turns; additions and scope changes update the ledger. Agent-reported states are completed, pending or blocked. A turn ending never completes unfinished tasks automatically. Do not derive counts from prose. Missing ledger retains today's card.

Show all tasks completed only when every listed task is completed. Runner checks remain separate: checking, no loose ends found, still open, unavailable. A successful empty check is required for no loose ends found. Task completion never implies independent verification. Scheduled work remains visible. Needs-you cards keep their existing actions and show progress without replacing asks.

## Implementation after design review

- Update protocol first, preserving old tool arguments and missing-field behavior.
- Add schema, persistence, reducer and mirrored field support; consult Convex skill before backend changes.
- Add experiment metadata, generated docs and review reminder.
- Render the task summary on GPUI, website and phone or explicitly document any surface gap.
- Cover multi-turn ledger updates, scope changes, partial completion, blocked tasks, missing ledger, failed runner checks, mirror parity and existing archive behavior.
- Update guide, files.md, task.md and changelog; run focused tests and appropriate checks before landing.

## Status

2026-11-16T21:42:56Z: source reviewed; static mock prepared for review.

Wayne rejected the colored button revision and supplied a narrow screenshot showing an amber accent on a rounded left edge. He clarified that the amber edge belongs on action buttons such as It works and Archive, never on the card. The approved mock keeps neutral button fills and adds a 3px amber left border to the buttons only. Typography follows the native system font (.SystemUIFont in GPUI, -apple-system in the HTML preview), with source-matched 14px medium card headings, 15px body text with 23px line height, and 13px button labels.

2026-11-17T00:09:11Z: implementation in progress. Runner persistence, stable-ID merges, explicit scope removals, experiment-gated tool argument, helper archive guard, protocol, API84 mirror and GPUI/web/iOS card sources are implemented. Optional Convex schema addition only; no data migration. Five focused runner completion tests, 100 completion/experiment/tool tests, 148 sync/mirror tests, 27 cross-route scenarios and completion tests, four Rust reducer tests, three Swift model tests, web tests/typechecks/build and runner/Convex typechecks pass. Counts overlap across commands. Broad quick check and native rendered verification are running. No production backend/web deploy, phone install, commit or landing yet.

Release constraints: schema/mirror fields are backward-compatible behind API84. An old backend receives no completion field. Wayne's App Store phone stays untouched; source/model tests are not device validation. Browser access host was unavailable for mock inspection. Review reminder 5c8e322b is booked four days out; it must check landing/enabled state before assessing use.

2026-11-17T00:27:25Z: implementation committed as 9a64d5ba. The broad runner suite passed in 975 seconds after the supported isolated retry of one remote-host lease test. Runner/Convex/web typechecks, web tests, site build and feature pairing passed in the same quick check. Its remaining native checks are still queued behind the supported headless build. GPUI automation cargo check passed earlier; actual rendered-card screenshots remain outstanding. Headless build is progressing through dependencies without a source error, at background priority/jobs2. No duplicate build started. Desktop CI thread 1b046499 is still finishing its Windows timeout fix; do not bypass that gate for this feature. No landing, enablement, production deploy or phone install yet.

2026-11-17T00:58:12Z: native build and rendered verification pass. Inspected actual incomplete and complete cards beside the approved mock: native typography, composer-attached summary, neutral buttons with amber left edges only. Direct scripted It works selection and Archive clicks work; Archive increased the archived count and changed selection. Turning the experiment off restores the original Done card for a stored completion event; reenabling restores the ledger. No unverified loose-end claim appears. Evidence is in ignored `.fixtures/completion-review/built-{incomplete,complete,off}.png` and accompanying dumps; the owned dev app/mock are stopped.

The original full quick check returned nonzero because both native steps hit their 600-second timeout waiting for cargo's build-directory lock behind the cold native build. Logs contain only lock waiting and process samples, not a compiler/test failure. Both failed commands passed after the lock cleared: all native core/client tests (including all five completion tests) in `.fixtures/completion-review/native-core-check.log`, and GPUI `cargo check --locked -p threadbear-gpui --features automation --all-targets` in `native-app-check.log` (10.95 seconds; existing warnings). The same original check passed Swift mock tests and the iPhone kit tests. Normal `tb land` remains required; main is 9d1311f9 and Desktop CI 43988092381 is still running on all three platforms.
