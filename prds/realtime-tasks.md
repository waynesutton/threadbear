# Realtime Tasks

Created: 2026-11-16T09:14:50.172751+00:00

## Approved experience

Tasks below Notes, opened with Command-7. Native GPUI desktop and authenticated mobile web /tasks share one Convex collection. The simple quick-add list is the default. Board has Backlog, Todo, Doing and Done. List/Board switch has an explicit Set as default action, synced across devices. Switching the current view does not change the default.

16 November refinement: enter a task in the list and press Return; check it to hide it in Done, with Undo. Each task's menu opens notes, changes status or moves it to a project. Cards omit status dropdowns. Drag columns, list sections and project groups to reorder them, with separate saved list/board orders. Keep the background file picker consistent with other controls. The supplied Linear PRD and better-todo are visual references; no extra issue-tracker features are in scope.

Tasks have title, optional notes, status, optional existing project, timestamps and manual order. Inline add, detail editing, checkbox completion, status menus, card and column dragging, delete with Undo. New tasks default to Todo or the column they were added in; unchecking returns to Todo. Filter by project or No project; group by status or project; sort manually, newest, oldest or title. Manual drag ordering only in manual sort.

Project pages show Tasks and count beside Decisions when tasks exist, and offer Add task when empty. Project picker supports the existing Create project flow. Notion tasks remain separate. No task is required to belong to a project.

Theme follows app appearance; background color or uploaded photo supports position and fade. Synced preferences. Mobile has touch targets, horizontal board scrolling, persistent Add and status-menu alternatives to drag.

Online editing only, cached viewing offline, explicit connection/error states and retained failed drafts. Web edits must work while Mac is off. No due dates, priorities, subtasks, public sharing or native iPhone screen in v1.

## List and board verification checkpoint 2026-11-16 20:44 UTC

The refinement adds Doing, independent column/list/project order, quick-add, hidden completed rows with Undo, and task menus on desktop and web. Backend API 84 is required by the new clients. Existing three-column preferences normalize to include Doing.

- Final native regression run: 10 scenarios, 131 steps passed, including failed editor save, quick-add retry across page close, concurrent edits, completion, deletion and ordering.
- Seventeen focused backend/bridge/context/experiment checks and eight web draft/order tests pass. The five Rust task-order unit tests pass. Runner, backend and web typechecks pass; production web build passes with the existing chunk-size warning.
- Real disposable Convex backend: runner/browser writes and realtime subscription, preferences, remove/restore and bad token checks pass. Headless Chrome exercised quick-add, completion/Undo, column/list/project ordering and server-rejected draft retry. Light/dark desktop browser and 390px mobile images inspected; browser access host is unavailable, so these are isolated test-browser checks.
- Fixed native editor height, opaque sheet/toast/background panel, and the frame-read hook so editor errors and Undo text are actually asserted. Quick-add inputs remain until save succeeds; a failed draft survives leaving Tasks.
- Diff security review traced all 12 public Tasks operations through capability authentication; scanner auth candidates are false positives. Full-tree scanner also reports unchanged files outside this diff, which were not audited. No clean full-repository security result is claimed.

Landing and production/installed verification of this refinement remain pending. The earlier Tasks deployment authorization applies; no new approval is needed.

Landing checkpoint 2026-11-16 20:47 UTC: `tb land` rebased the feature to `4eeb6ce2` on main `1300ba08`, then stopped at the Desktop CI gate. Last completed run `43922230506` passed Linux and Windows but cancelled Mac before any steps; newer run `43948148837` remains pending. Do not bypass that gate. The standalone `./check.sh --quick` is running in `.fixtures/tasks/v2/quick-check.log` (runner/backend/web typechecks, web tests, feature pairing and site step passed so far). The headless dev app, disposable backend and Vite server are stopped. No refinement deployed, pushed or landed. Keep the rollout together because the old desktop labels a Doing column as Todo. Once the gate clears, land, deploy API 84 plus web, set Wayne's default view to list, and verify the automatic desktop update using the existing restart workflow.

Verification follow-up 2026-11-16 21:10 UTC: the broad check caught four new Tasks focus paths that did not return AppKit's keyboard from a web panel. Added the existing `keyboard_to_app` handoff after composer, title, notes and column-input focus. `gpui-focus-keyboard.test.ts` reproduced all four missing calls and now passes. The serialized GPUI automation cargo check also passes. The broad suite is still progressing, with remote-host withdrawal and dev-app lifecycle tests also awaiting its automatic retry. GitHub run `43948148837` has again cancelled Mac without acquiring a runner; Linux and Windows remain in progress. No deployment or landing yet.

## Native verification checkpoint 2026-11-17 00:56 UTC

Accessibility fix `90a429b3` passes all four source-role tests and the serialized GPUI cargo check. The automation app rebuild finished successfully; the final Tasks run passed all 10 scenarios and 131 steps in 1m 47s (`.fixtures/tasks/v2/a11y-native-tests.log`). Its app processes have exited. The broad quick check's only remaining failure is now fixed and verified; do not repeat passing suites before the required landing check.

CI timeout fixes `7f2ba41e` and `9d1311f9` are on main. The CI owner reports fresh Desktop run `43988092381` has acquired runners on Mac, Linux and Windows and all three are running specs. Landing remains gated until it succeeds. Thread `1b046499` owns reruns and pushes; this thread must not duplicate them or bypass the gate. Tasks refinement remains unlanded and undeployed. Next: normal `tb land`, coordinated API 84/web rollout, list default, automatic restart watch and installed/live verification under the existing approval.

## Architecture

Verification follow-up 2026-11-16 21:46 UTC: the broad quick check finished. Runner unit tests passed after the built-in retry of 52 tests; typechecks, web tests, feature pairing, desktop app check and mock tests passed. The only remaining failure was `a11y_roles_source`: seven custom Tasks controls/containers lacked roles. Added labelled button roles for opening tasks/cards, folding sections/projects, showing Done and choosing colors; fold state is exposed, the editor is a labelled dialog, and its dismiss backdrop is explicitly a container. All four accessibility source tests now pass. GPUI compilation is queued behind the machine build lock (`.fixtures/tasks/v2/a11y-cargo-check.log`, exec99227). No full-suite rerun is needed before this focused result; landing will run its required checks. CI remains held: Linux timed out, Mac lacked capacity, Windows still running. CI helper thread1b046499 is landing the Linux timeout fix.

New task and preference tables, authenticated with existing paired browser tokens and runner authentication. Deployment is the existing single-owner account boundary. Project identity includes runnerId plus project id. Convex owns tasks, browser subscribes directly, desktop uses runner subscription bridge. Keep existing project mirror ownership intact. Field-specific updates, retry-safe creates, timestamp-based initial ordering and persistent manual order.

Agent tools list/create/update/move/delete tasks and resolve projects without silently selecting ambiguous names. Uploaded backgrounds require authenticated access. Additive protocol changes; old clients remain compatible. New experiment off by default.

## Verification

Focused backend tests for auth, retries, updates, project ownership, ordering, Undo and preferences. Runner bridge and agent tool tests. Web typecheck, backend typecheck, focused web tests, GPUI core/client checks and cargo check. Development UI evidence for board/list, add/edit/check, project filtering, default selection, shortcut, dragging and light/dark mobile layouts. Document unverified live behavior accurately.

## Delivery constraints

Work only in this borrowed worktree. Wayne's deployment approval recorded below authorizes landing and coordinated backend, web and desktop rollout. No publishing or push from this thread; the CI helper owns its authorized CI pushes. No changes to Swift app or ios. No package additions without approval. Record a motion preview before UI implementation and validate completed UI against it. Stop own development processes before ending work.

## Implementation checkpoint 2026-11-16T09:28:00Z

Implemented locally, uncommitted: convex/tasks.ts + taskFields.ts and additive schema, authenticated list/create/update/remove/restore/preferences/photo upload/photo URL/revision; timestamps strictly increase for reliable invalidations; paginated list. Eight focused tests pass (tasks-backend.test.ts and tasks.test.ts). Backend, web and runner typechecks pass. Runner tasks.ts bridge and tasks.* RPCs wired, MCP tools wired to Claude and Codex; subscription starts lazily and publishes tasks.updated. Web /tasks page built with editing, board/list, defaults, background, filters/grouping/sort, native HTML dragging. A first GPUI tasks_page.rs and Cmd7/navigation wiring exist but HAVE NOT BEEN COMPILED. No production changes, no push, no commits.

Remaining work is substantial; do not report this feature finished:
- Compile and repair GPUI when machine load is safe (current all load averages around 46). No checkHosts/updateBuildHosts configured. Do not use Mike's machines. Current page may contain type/API errors and is incomplete; cargo check required.
- Native desktop: project picker/filter/grouping/sort, multiline notes, true checkbox and status menu, column reordering, background color/photo/fade/position controls, explicit default indicator/menu, Open on web, task detail editing, stable retry id and clear/new draft after success, protected draft transitions, async refresh coalescing. Current TaskRow needs createdAt for sorting. New task currently no explicit retry id and retains fields after create; fix before testing.
- Project page Tasks count/tab beside Decisions and add task when empty, both surfaces. Existing Notion board must remain separate. Add/create project from picker via existing runner flow; web offline runner message.
- Experiment file/tasks gate, generated docs and check-in schedule; web visibility must honor capability/experiment and older backend rollout (currently unconditional route/tool references). Native uses experiment_on but file not created yet.
- MCP project resolution: list_projects doesn't currently give runnerId in its prose, so tools need usable project lookup/options and ambiguity rejection. Agent tests/context-budget regenerate needed.
- Runner auth gates test for read/write caller permissions; bridge subscription errors currently just notify and subscription lifecycle needs reconnect/config disable handling. Ensure startup callback cannot access nativeTasks before initialization.
- Data: orphaned project references should read unassigned or cleanup on deletion; valid projects checked now but no cleanup. Storage uploads need cleanup/replacement lifecycle and tests, privacy is account auth with storage capability URL returned. No real dev backend deployed/tested. Existing key-scoped runner auth nativeTasks shared policy needs review against project visibility expectations.
- Web: focus trap/restore in custom dialog, keyboard and touch drag support (HTML DnD only currently), project grouping drop must update project membership, inline add, reconnect retry query, synced theme (prefs.theme exists but not applied), matching actual app palette (updated --panel to --raised), cancellation errors and column self-drop guard, responsive visual verification.
- Protocol currently sketch; finish exact RPC/event/type contracts. Add features/tasks.feature + mock runner handlers; tests for defaults/shortcut/status/drag/project filters. Update guide/on-your-mac/forking/task.md/files.md/changelog (git log first).
- Need backend function integration against a dev/local deployment, only if safe without changing production config; no production publish/deploy authorized.
- Motion study generated with Swift/AppKit and ffmpeg in .fixtures/tasks (ignored), pinned and image inspected. Initial failed ffmpeg drawtext attempt was replaced with successful 72-frame render. Clip is early study, not final implementation. Avoid claiming its appearance matches actual build yet.
- All development subprocesses ended. No heavyweight builds launched due load. Main checkout belongs to other threads; stay in slot-23.


## Implementation checkpoint 2026-11-16T10:05:00Z

Still a local, uncommitted draft. No production deployment, push or landing.

Since the last checkpoint:
- Native page now has project selection/filter/grouping/sorting, multiline notes, status checkboxes, card and column drags, color/photo/fade/position controls, default selection, Open on web, stable create ids, retained failed drafts and refresh coalescing. Fixed refresh to actually apply returned projects/photo. Added dormant draft page when navigating away and update holds while drafts/saves remain. Native appearance is sent on opening Tasks and on app appearance changes. Photo crop uses existing BackdropFocus and the image loader. ALL THIS REMAINS UNCOMPILED; inspect/fix actual Rust diagnostics next.
- Project page Tasks links/counts, mock runner Tasks operations behind TB_MOCK_TASKS, native automation command and tasksPage dump, features/tasks.feature + steps and world env wiring added. Core keyboard group and shortcut count updated.
- Runner project-name/id resolution rejects ambiguity; connection identity/key changes replace subscriptions; disabled/offline calls stop watchers. Startup nativeTasks variable declared before lazy tool callbacks. MCP list/create/update/delete tools tested and list is a never-ask read tool. Context budget now measures Tasks and was regenerated.
- Web has focus trap/restore, retained failed drafts with errors inside dialog, touch card/column handles and keyboard reorder, safe busy state, grouped adds and drop membership only in project grouping, online reconnect refresh, themes and project links. Production build and TypeScript checks pass. Browser automation could not start: CUA host unavailable. No actual visual check yet.
- Protocol RPC contract, guide/on-your-mac/forking/files/task/changelog documentation updated as draft. Runner-access policy test recognizes the shared nativeTasks helper and protocol access table lists it. A separate exhaustive backend test checks every public Tasks endpoint rejects both invalid runner key and browser token.

Verification:
- 14 task tests (9 backend, 4 bridge, 1 MCP) pass; 9 context-budget/experiment checks pass. Runner-access 17 and thread parity 4 pass. Backend/runner/web typechecks and Vite build pass (existing bundle size warning).
- Deployed the complete schema/functions ONLY to a disposable existing convex-local-backend on localhost 3294/3295. Real HTTP runner/browser edits, WebSocket invalidation, default preference sync, soft-delete/Undo, bad token, real photo upload/metadata validation/lookup/reset all passed. Scripts and private disposable auth are in ignored .fixtures/tasks/backend. No production config touched. Convex CLI made an ignored REGULAR .env.local in this worktree only; not a symlink. Build with VITE_CONVEX_URL=http://127.0.0.1:3294 explicitly.
- Installed existing locked features dependencies with npm ci (no dependency changes). Feature pairing passes. Tasks-only Cucumber dry run resolves both scenarios. Full dry run finds pre-existing duplicate approval steps in cards/editing; not changed here.
- First serialized offline cargo check finally ran and failed on two core integration errors: ShortcutCommand ALL array length 29 vs30, and missing keyboard group arm. Both FIXED. Second check with --features automation is currently queued behind another build; unified session 36859, output .fixtures/tasks/cargo-check.log. Poll it, then fix all compiler errors. Original session54894 completed. Do not kill other builds or bypass lock.
- Local Convex PID19585 and Vite PID27831 stopped before this checkpoint. No app/dev runner running. Only our queued cargo check remains active.

Next work:
1. Poll cargo check session36859/log. Finish native compilation, focused core shortcut tests, then serialized automation build and headless TB_MOCK_TASKS=1 dev app. Need actual board/list/default/add/edit/check/drag/project/background/appearance/shortcut UI evidence. Browser host may come back; retry actual web/mobile visual check if available. Never claim mock motion-study clip is final build.
2. Native code still needs visual polish and any compile repairs. Check card widths/compact list, form screen use, tooltips on checkboxes/inputs, background loader type, nonmanual drag handling, retained draft after success while page closed, and app update hold. Native create-project flow works via existing app dialog but has not been exercised. Web project picker selects synced existing projects; web create-project flow is still absent (runner owns project creation). Decide the smallest safe integration if required by accepted scope.
3. Storage replacement/orphan cleanup still absent; files retained. Subscription disable only checked on calls; configuration-change teardown and user-visible connection state should be reviewed. Web experiment is capability gated (API83), independent of runner toggle, documented. Backend functions are one shared account collection.
4. Add/strengthen native behavioral assertions on drawn content (current dump checks are state-only), run UI specs and inspect screenshots. Existing full dry-run ambiguity is unrelated. Check all affected tests/docs after repairs. No commits yet; stage only own paths, and do not land/push because that would update live app/runner without the matching authorized backend deployment.

## Implementation checkpoint 2026-11-16T11:03:00Z

Local implementation is built and behaviorally verified. No production deploy, push or landing.

- Fixed native parent/child entity borrow panic by giving TasksPage its own runner client. Full automation builds pass. Core shortcut tests: 10 pass. Cmd7 opens Tasks below Notes.
- Native editor is expandable, retains drafts when hidden, and closes after a successful save. Toolbar and form options wrap in narrow panes. Actual board/list, project assignment/filter, default choice, completion, delete/Undo, light/dark screenshots inspected.
- Native saves use an editing baseline and patch only fields changed by the user. Web does the same with two focused regression tests. A remote completion survives a concurrent title edit.
- Native screen assertions now inspect visible native controls and recorded heading/error text, independently of stored page state. Six Cucumber scenarios, 77 steps pass: Cmd7, add, explicit default, failure/leave/retry, concurrent edit, completion/Undo. Generated parity reflects this targeted run (unrelated previously missing rows also regenerated).
- Native scripted pointer drag moved a card Done -> Backlog and reordered columns Backlog/Done/Todo, confirmed in actual screenshot. No system pointer or other app was used.
- Runner stops Tasks subscriptions immediately when its experiment changes. The three subscriptions cover revisions, preferences and project changes.
- CUA host was unavailable. Used isolated headless Chrome with a disposable profile and the disposable local Convex backend instead. Found and fixed the mobile rail/layout conflict: phone now uses full width and bottom navigation. Actual 390x844 light list, task editor and dark board inspected. Browser task creation saved to real local Convex and rendered. CDP touch events dragged a task between columns. Authenticated browser photo upload, returned image loading and reset passed. No browser libraries/dependencies added.
- Latest checks: 14 backend/runner/MCP tests; two web draft tests; runner/web typechecks; Vite build. Existing bundle-size warning remains. Native full build passes; rustfmt is not installed, so no rustfmt claim. Previously verified exhaustive auth and real two-client/WebSocket sync remain valid.
- Evidence is ignored in .fixtures/tasks: native-features.log, native-list.png, native-light.png, native-drag.png, mobile-board.png, mobile-editor.png, browser-smoke.mjs and browser-photo.mjs. Native images use only mock sample threads. Browser uses disposable local data.

Delivery limitations: creating a project uses the existing native Create project dialog; browser selects existing synced projects. Background uploads remain in storage when replaced/reset (no garbage collector). Native UI was tested against the mock bridge; backend/website were tested against real local Convex separately. No installed-app, production-backend or physical-phone verification is claimed. The live /tasks URL requires deploying the matching backend API83 and website and enabling the desktop experiment.

## Deployment checkpoint 2026-11-16T14:40:00Z

Wayne explicitly approved Deploy Tasks. No more approval is needed to deploy, land, enable or verify this feature.

- Rebased on main897aa777; regenerated context budget. Feature commit is e968d8b4.
- Deployed backend and website successfully with deploy-backend.mjs to the configured owner deployment uncommon-seal-914. Live https://uncommon-seal-914.convex.site/tasks returns200. Authenticated create/read/update passed; verification task removed. Default view is board.
- Landing is held by fork GitHub run43876632987. Windows and Linux pass. Mac has twice been canceled before any step ran: hosted macOS arm64 capacity unavailable. Retried failed job again at14:39Z, currently queued. Do not mistake this for a code failure or report desktop installed.
- Local ./check.sh --quick completed: runner/backend/web typechecks, web tests, core tests, desktop check and app mock tests pass. Two failures fixed: site docs icon missing (2007ae6c, focused site build passes) and keyboard ownership on Tasks open/close (fdbbd92c, focused focus-contract test passes). Latest serialized cargo check with automation passes too. Full check log .fixtures/tasks/prelanding-check.log; focus check .fixtures/tasks/focus-check.log.
- No dev apps, local backend, browser or check processes remain. Next: once CI clears, tb land with cargo PATH and THREADBEAR_NODE set. Let runner/desktop update safely, enable tasks through settings.update, verify Tasks RPC and installed Cmd7; register wait_for_app_restart after landing. Do not ask approval again or use Mike's hosts.
