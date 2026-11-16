# Bears that pick up where they left off

Updated 2026-11-14 23:46 UTC. Approved in thread 327a5a9e.

## Purpose

Build the five approved improvements from the always-on agents plan on this fork. Keep runner behaviour behind `bears`, with late tool answers behind a separate `lateAnswers` experiment. No Swift features or production backend deployment. GPUI presentation is tracked in bears-gpui.md.

## Contract

- Routines, webhooks and mentions share a busy unattended bear thread. User-created chats stay independent. Running, starting, waiting, queued and background work count as busy. Queued work starts after the current turn. Access and working folders must match: incompatible work waits or is refused, never starts a second worker with wider access or runs relative paths in another folder.
- Bears read `memory/told.md` and keep unfinished jobs in `memory/tasks/`, linked from their summary. Refresh only a marked rules block in existing personas; preserve user text.
- Directly created Claude memory tidies use Auto; Codex and agent-created bears use Ask. An authorized refresh upgrades built-in Claude tidies. Refresh only recognized built-in tidy schedules, preserving custom routines and paused state. No bear may use refresh to raise its own access.
- Empty unattended runs may quietly leave the inbox; useful reports, questions, failures, user-touched threads and unfinished work stay. A quiet completion must be explicit, not inferred from “needs nothing”.
- A tool card that expired within a day can accept a late allow or deny. Queue the precise request and answer for a new turn, without replaying the old call or changing access. Preserve remote-card restrictions, reject duplicates and unknown cards, and never turn secrets or questions into approval messages.

## Work and verification

- [x] Shared worker routing: bears, schedules, webhooks and mentions tests, including incompatible modes and queued replies.
- [x] Memory rules and tidy refresh: preservation, idempotency, Codex mode, custom routine and caller restrictions.
- [x] Quiet completions: untouched unattended bears only, explicit empty result, no open work or race with a new message.
- [x] Late answers: off by default, expiry, duplicate, permission and remote restrictions, restart behaviour.
- [x] Update guide, protocol, experiments, bears skill, both bears PRDs, task/changelog/files and handoff plan.
- [ ] Runner typecheck and focused tests, then the repository quick check. Exercise runner integration without production backend calls. Land on this fork only after verification.

## GPUI follow-through

Expose the shared worker and folded work in a bear row; show expired tool cards as resumable only when supported. Existing APIs keep their argument forms. No new Swift UI.

## Review corrections (14 Nov 2026)

Review 8f9c5f07 found three valid issues before landing:

- Folded mentions now compare the caller's saved mode and confinement with the worker's current authority. The real CallerGate-to-bears.mention regression rejects a sandbox escape without queuing work.
- Folded routines compare explicit account, pool, model and effort settings. Incompatible work stays due; matching settings still fold. The regression confirms a waiting Codex routine starts with its requested model and effort once the worker finishes.
- A late-answer retry requires a queue entry or a successful turn.started event before treating delivery as accepted. A partially logged message is reused by sequence and turn ID on retry, including across a runner restart. A different answer or note cannot silently replace the first saved one.

All eight targeted regression cases passed at 2026-11-14 23:42 UTC. At 23:46 UTC, all 226 affected tests (shared workers, mentions, late answers, schedules, runner and cross-device scenarios) and runner typecheck passed. Repeat review and the repository quick check still precede landing; the Mac's load was 13 at this check.

## Verification evidence

2026-11-16 13:50 UTC: landed. Loads were 2.11, 2.21 and 3.38 with no competing build. Rebased over 186 commits on main (conflicts in changelog, task, main.ts imports, the routes helper and the queued-send types, where main had dropped TurnMode); the generated context budget and experiments list were regenerated. Typecheck clean and 291 focused tests passed (bears, bears-always-on, late answers, mentions, tables, webhooks, quiet inbox, schedules, scenarios, threadbear tools and the guards). `tb land` ran `./check.sh --quick` and passed in 539 seconds; main moved to 897aa777 and `scripts/sync-main.sh` pushed it. The live runner followed main to 897aa777 by itself. `tb bears refresh-rules` refreshed Head Bear, Researcher, Writer and Designer: every bear.json unchanged, no persona line removed, one managed always-on block added, Head Bear's Chief of staff block intact, memory/told.md and memory/tasks/ added. The four memory tidies kept their ids, schedules and paused state; Head Bear, Researcher and Writer moved to Auto, and Designer (made by an agent) kept Ask. There were no custom routines to preserve; the unit tests cover that path. PR #66's pr/bears branch already holds the adapted fixes (e08c0ac8, 9423e684, 4a98720d); the fork-only research doc stays out of it.

2026-11-15 04:05 UTC: repeat review cb93873e is clean. Started `check.sh --quick` with load 3.67; runner/Convex/web typechecks, app-tool parsing, web tests and site build passed. Load then rose above 45 and process-based unit tests timed out; stopped this check and its descendants after 6,169 seconds. Logs remain in `/tmp/bears-final-quick.log` and `$TMPDIR/threadbear-check.a9AnIm/`. One failure reproduced independently: the tool-schema assertion omitted the new `quiet` field. Corrected the assertion; all 90 threadbear-tools tests pass. No product changes were needed. The broad check is incomplete, so nothing landed. The retry now requires all three load averages below 4, rather than a brief one-minute dip. Existing bear refresh remains pending after landing and automatic runner update.

2026-11-14 23:28 UTC: runner typecheck passed. The focused guards and feature tests passed (71 tests); integrated routing, schedules, bears and quiet-inbox passed (109 tests); late answers, mentions and cross-device scenarios passed (35 tests). These groups overlap. The temporary dev runner accepted `bears new`, `bears refresh-rules`, and `schedules.list`; a Codex bear retained Ask and one managed persona block, with told.md and tasks/ present. The temporary runner was stopped. No real model or production backend was used for these checks. The repository quick check and landing remain pending while the Mac load remains above 30.

Changes from the approved outline: explicit `quiet: true` avoids hiding useful reports; mention waiters track turn IDs; agent-created bears keep Ask tidies until an authorized refresh; repeated crons skip their own busy runs. Late answers use existing events instead of a second 24-hour input store.
