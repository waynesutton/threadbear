# Send a Threadbear task to Linear

## Context

Linear is connected through Connectors (plugbear, `127.0.0.1:8795`, 78 tools). Earlier today another thread gated every `linear__save_*` call behind approval. Wayne wants a control on an open task, "Send to Linear", that creates an issue in the team and project he picks. It must never make a duplicate, never write to the wrong place, and never open a back door around the approval gate for agents.

Decisions (from Wayne):
- **The click is the approval.** Threadbear gets its own Connectors agent. Only that agent may create issues without asking. Updates still ask, and so does everything a thread's agent does.
- **Sync only the tasks you choose, from Linear to Threadbear.** Every linked task has a "Keep in sync with Linear" switch. It's ticked by default in the send sheet, and you can change it any time. For synced tasks, Threadbear checks Linear every hour by default, and again whenever you open the task. You can change the interval in Tasks settings to every hour, 6 hours, 12 hours or once a day. The checks are direct calls from the runner, with no model involved, so they cost no tokens.
  - Copied from Linear: **status, priority, due date, title and comments.** Comments show read-only in Activity.
  - Never copied: the description, so your notes are never overwritten.
  - Threadbear never pushes edits to Linear.
  - A task with sync off keeps its link and status chip from the last check, and nothing runs for it.
  - A Linear webhook can come later and make updates faster without changing anything you see. It's not part of this build.
- **Sub-tasks become a checklist** in the issue description. One issue, one create call.
- **Pictures go too.** A task's pictures (pasted or dropped into the description, or added with Attach image; they're `kind:'image'` links stored in Convex) are uploaded to Linear and shown inline in the issue description. You can untick any of them in the send sheet.

Facts the design rests on:
- Tasks live in Convex (`convex/taskFields.ts`, `convex/tasks.ts`). There is no external-id field. `links` holds URLs but can't be looked up and gets replaced as a whole on update.
- The runner already calls Connectors tools directly for Notion boards: `gatewayCaller` / `helperHeaders` / `findGatewayClient` in `runner/src/notion-tasks.ts:40-120`. Its 30s timeout and its handling of the "Waiting for Wayne" text (returned as a normal result) both need changes for Linear.
- plugbear policy rules accept `agents` and `ifArgs` (`policyRuleSchema`, `connectors.mjs:94274`). Rules run in order and the first match wins.
- Connector status reaches GPUI only (`desktop/core/src/connectors.rs`, `connectors_panel.rs`). It isn't in Convex, so the web app can't tell whether Linear is connected.
- Work happens in `/Users/waynesutton/threadbear` (origin = Wayne's fork). Branch off main, land with `tb land`, no PR to Mike without asking.

## Linear is optional, always
Tasks work exactly as they do today when Linear isn't connected. Nothing new shows: no menu item, no Linear row, no chip, no setting, and no background polling. Nothing nags you to connect it.
- One gate, `linear_ready`, controls every Linear control in the Mac app. It's true only when Linear's connector state is `connected` and the runner's API is 89 or later.
- Signing out of Linear in Connectors hides the create controls again. A task that's already linked keeps its `ENG-123` chip as a plain link and stops refreshing. Nothing is deleted.
- The runner's sweep and the `linear.*` RPCs do nothing when Linear isn't connected. They return a clear "Linear isn't connected" message and never throw into the Tasks page.
- The `linear` field is optional in Convex. Old runners and the web app ignore it, and tasks without it behave the same as today.
- Tests cover both states: Linear connected, and no Connectors gateway at all.

## Design

### 1. Connectors: a `threadbear` agent with create-only rights
Config only, in `~/Library/Application Support/Connectors/config/`. Back up both files first.
- `agents.json`: add `{ "id": "threadbear", "title": "Threadbear app", "upstreams": ["linear"] }`. That agent sees Linear and nothing else.
- Mint its token the same way the `claude` token was minted (check plugbear's CLI for the add-agent or token command, and don't guess). Store it in the Keychain as `-s connectors -a threadbear`. `~/.config/connectors/headers.sh threadbear` already reads tokens by account name.
- `policy.json`: put these two rules above the existing `linear__save_*` ask rule:
  ```json
  { "tool": "linear__save_issue", "agents": ["threadbear"], "ifArgs": ["id"], "action": "ask" },
  { "tool": "linear__save_issue", "agents": ["threadbear"], "action": "allow" }
  ```
  With these, the `threadbear` agent can create issues without asking. Any update (a call that includes `id`) still asks. `claude` and `codex` keep every gate they have today.
- Below those, keep the `threadbear` agent to only the tools the feature uses. Allow `linear__list_teams`, `linear__list_projects`, `linear__list_issues`, `linear__get_issue`, `linear__list_comments`, `linear__get_workspace` and `linear__prepare_file_upload`. Then add `{ "tool": "*", "agents": ["threadbear"], "action": "deny" }`, so a leaked Threadbear token can't delete, comment or merge.
- Add `connectors.linearHeadersHelper` to `runner.json`, defaulting to `headers.sh threadbear`. If the agent is missing, the runner falls back to the `claude` token and the approval path in section 3 still works. So the feature never breaks. It just asks twice.
- Restart plugbear with `launchctl kickstart`. Check `/health` and confirm `linear__save_issue` from `claude` still asks.

### 2. Convex: a `linear` field on tasks (API 105)
`convex/taskFields.ts`: add an optional field:
```ts
linear: v.optional(v.object({
  state: v.union(v.literal('creating'), v.literal('linked'), v.literal('failed')),
  requestId: v.string(), claimedAt: v.number(),
  teamId: v.string(), teamName: v.string(),
  projectId: v.optional(v.string()), projectName: v.optional(v.string()),
  issueId: v.optional(v.string()), identifier: v.optional(v.string()), url: v.optional(v.string()),
  status: v.optional(v.string()), statusType: v.optional(v.string()), // started, completed, canceled...
  syncedAt: v.optional(v.number()), error: v.optional(v.string()),
  sync: v.boolean(),                         // the per-task "Keep in sync" switch
  issueUpdatedAt: v.optional(v.string()),    // Linear's updatedAt last applied; older answers are ignored
  // What Linear said last time, so a field changes here only when it changed in Linear.
  last: v.optional(v.object({ title: v.string(), priority: v.number(), dueDate: v.optional(v.string()), statusType: v.string() })),
  // Pictures already uploaded on an earlier try, so a retry doesn't upload them again.
  uploads: v.optional(v.array(v.object({ storageId: v.id('_storage'), assetUrl: v.string() }))),
}))
```
`taskPreferenceFields`: add `linear: { syncEvery: '1h' | '6h' | '12h' | '1d' (default '1h'), lastSweepAt?, defaults: [{ project: {runnerId,id} | null, teamId, projectId? }] }`. The send sheet uses these to remember the last team and project per Threadbear project.

`taskActivityFields`: add `source: v.optional(v.literal('linear'))` and `externalId: v.optional(v.string())`, the Linear comment id. Add an index `by_external` on `['externalId']` so a comment is never copied twice. `removeComment` refuses entries that came from Linear.

`schema.ts`: add an index `by_linear_sync` on `['linear.sync', 'deleted']` so the sweep reads only the synced tasks, never the whole table.

New mutations in `convex/tasks.ts`. All of them are runner-key only, through `runnerAuth(..., 'nativeTasks')`:
- `linearClaim {id, requestId, teamId, teamName, projectId?, projectName?}`. Refuses if the task is deleted or already `linked`, or if it's `creating` and the claim is under 10 minutes old. Writes `creating`. This is the lock against double clicks and two Macs.
- `linearSettle {id, requestId, ...result | error}`. Only works when `requestId` matches, so a late answer can't overwrite a newer try. Writes `linked` or `failed` and logs an activity event ("sent to Linear as ENG-123").
- `linearApply {id, issueUpdatedAt, title, priority, dueDate?, status, statusType, comments[]}`. Ignored if `issueUpdatedAt` isn't newer than the stored one, which keeps two Macs from racing. It always updates the chip's status text. When `sync` is on, it also does the following:
  - It applies a field only if it differs from `last`, which means Linear changed it. A local edit stays until the same field changes in Linear.
  - Status mapping: `triage` and `backlog` → backlog, `unstarted` → todo, `started` → doing, `completed` → done. `canceled` → done, with a "Canceled in Linear" event. Reopening the issue moves the task back.
  - Priority mapping: 1→urgent, 2→high, 3 or 0→normal, 4→low. `dueDate` and `title` are copied as they are.
  - New comments go in as Activity with `source:'linear'`, skipping any whose `externalId` already exists. The author is the Linear user's name, followed by "in Linear".
  - Each change logs one event, for example "Linear changed status to Done". Then `last` and `syncedAt` are saved.
- `linearSetSync {id, sync}`. The switch.
- `linearUnlink {id}`. Clears the field. It never touches Linear.
Bump `API_VERSION` to 89 in `convex/version.ts` with a note. The runner checks API 105 before it shows the feature, using the existing pattern in `runner/src/tasks.ts:57-61`.

### 3. Runner: `runner/src/linear-tasks.ts`
- Move `gatewayCaller`, `parseRpc`, `helperHeaders` and `findGatewayClient` out of `notion-tasks.ts` into a shared `runner/src/gateway-call.ts`, and point Notion at the new file. Add a per-call `timeoutMs` and an `onWaiting` result. When the text matches `Waiting for … /approvals/(ap_\w+)`, return `{ waiting: { id, url } }` instead of treating it as success.
- RPCs, added to `taskMethods` and the gate in `runner/src/rpc-gate.ts`:
  - `linear.options {refresh?}`: returns `{workspace, teams, projectsByTeam}` from `linear__get_workspace`, `linear__list_teams` and `linear__list_projects {team, fields:[id,name,status,teams]}`. Leaves out archived and completed projects and caches results for 5 minutes. These are reads, so they never ask.
  - `linear.send {taskId, teamId, projectId?, title, description, imageIds[]}`. `imageIds` holds the image links left ticked in the sheet. Ids that aren't on the task are dropped.
    1. Re-read the task. Refuse if it's deleted, done or already linked.
    2. Check that `projectId` is one of the team's projects, using a fresh `list_projects {team}`. This guards against the wrong project.
    3. Call `linearClaim`.
    4. Look for a duplicate. Search `linear__list_issues {query: <marker>, team}` for the marker line `Threadbear task <id>`. If an issue is found, link it instead of creating a new one. This covers a create that worked while the write-back failed.
    5. Upload pictures, one at a time, as Linear's tool asks. Skip any that are already in `linear.uploads`.
       1. Get the bytes from the task's Convex storage URL (`imageUrl`, `convex/tasks.ts:60`). Read the exact size and content type.
       2. Call `linear__prepare_file_upload {filename, contentType, size}`.
       3. Send the raw bytes with `PUT` to `uploadRequest.url`, with its signed headers exactly as given. The URL must be https and redirects are refused. It has to happen within 60 seconds of step 2.
       4. Save `{storageId, assetUrl}` to `linear.uploads` (`linearUploaded` mutation, only accepted with a matching `requestId`).
       5. Add `![<title>](<assetUrl>)` to the description, under the notes.
       
       Limits: at most 10 pictures, each already under 10 MB because Convex checks that, and PNG, JPEG, WebP or GIF. If a picture fails, it's left out with the line "1 picture couldn't be uploaded" in the result. The issue is still created. A picture that uploaded before a failed create stays in Linear's storage with nothing pointing at it. That's harmless, and a retry reuses it.
    6. Call `linear__save_issue {team, project, title, description, priority, dueDate, links}` with no `id`.
    7. If the call comes back waiting for approval (the fallback token), keep `creating`, send `tasks.linearWaiting {url}` to the app, and poll `connectors__await_approval` for up to 24 hours. Then settle.
    8. Call `linearSettle`. Also add a `kind:'link'` entry ("ENG-123 in Linear") so the existing Links UI and the web app show it.
  - `linear.refresh {taskId}`: `linear__get_issue` plus `linear__list_comments {issue}` → `linearApply`. It runs when a task detail opens, or when you click Refresh on the Linear row.
  - **Sync sweep**, run on the `syncEvery` interval, only while at least one task has sync on and Linear is connected. The runner wakes about every 10 minutes and sweeps only when `lastSweepAt + syncEvery` has passed. That way a Mac that slept through a sweep catches up on wake, and changing the setting takes effect right away:
    1. Read the synced tasks with the `by_linear_sync` index, grouped by team.
    2. Per team, call `linear__list_issues {team, updatedAt: <lastSweepAt minus 1 minute>}`. Check that `list_issues` really takes `updatedAt` before relying on it. If it doesn't, fall back to `get_issue` per synced task, still capped., so one call covers every synced task on that team. Match the results against the linked issue ids.
    3. Only issues that actually changed get a `get_issue` and `list_comments` call, then `linearApply`. Most sweeps make one call per team and change nothing.
    4. At most 3 calls at a time, the same cap as `notion-tasks.ts:33`. Only one Mac sweeps at a time: claim the sweep with the `claimReminder` pattern. After a sleep it catches up from the stored sweep time.
    5. If an issue is deleted or no longer reachable, mark the chip "Not found in Linear" and turn sync off for that task. The task itself is untouched.
  - `linear.setSync {taskId, sync}`, `linear.unlink {taskId}`.
  - `linear.unlink {taskId}`.
- Field mapping:
  - Priority: urgent→1, high→2, normal→3, low→4.
  - `dueDate` is passed through as is.
  - `links`: only http(s) links go to Linear. Print and thread links stay local. Pictures go inline in the description, as above, not as links.
  - Status isn't sent, so the issue lands in the team's default state.
  - The description is the notes, then `- [ ]` / `- [x]` lines for sub-tasks, then a separator and `Sent from Threadbear · Threadbear task <id>`.
- Parse Linear's real JSON answers. Capture one sample of each tool's output in a fixture before writing parsers, and don't guess shapes.

### 4. GPUI (the only app that can create)
`linear_ready` is true when Linear's connector state is `connected` and the runner's API is 89 or later.
- `desktop/gpui/src/tasks_page.rs` `more_menu` (L1252): add "Send to Linear…" when `linear_ready` and the task isn't linked. When it is linked, show "Open in Linear" and "Unlink from Linear" instead.
- `desktop/gpui/src/task_detail.rs` properties block (L640-689): add a Linear row. Unlinked, it's a "Send to Linear" button. Linked, it's a chip with `ENG-123 · In Progress` that opens the URL. While creating, it says "Creating…" or "Waiting for approval" with a link to the approval page. Failed shows the error and a Retry button.
- List row and card: a small `ENG-123` meta chip next to the existing chips.
- New `desktop/gpui/src/linear_send.rs` sheet:
  - The workspace name at the top, so it's clear which Linear this goes to.
  - A team picker, then a project picker filtered to that team, with "No project". Both start on the remembered defaults for the task's Threadbear project.
  - A title field and a read-only description preview of exactly what will be sent, plus the count of sub-tasks.
  - A row of picture thumbnails, reusing the description preview images from `task_detail.rs`. Each has a checkbox, ticked by default, with the line "Ticked pictures upload to Linear". Sending shows progress like "Uploading picture 2 of 3".
  - Create and Cancel. Create stays disabled until a team is picked.
  - Esc closes it. Errors show inline.
- The sync switch:
  - In the send sheet, a "Keep in sync with Linear" checkbox, ticked by default.
  - On the task page's Linear row, a toggle and a Refresh button, with "Synced 3 min ago" under the chip.
  - In the ⋯ menu, "Keep in sync" and "Stop syncing".
  - On the list row and card, a small sync mark on the `ENG-123` chip when sync is on.
- Tasks settings: "Check Linear for changes", with a picker for every hour (the default), 6 hours, 12 hours or once a day. It only shows when Linear is connected.
- Activity shows Linear comments with a small Linear mark. They have no delete button.

### 5. Web and docs
- `web/src/tasks.tsx` / `tasks-issue.tsx`: show a read-only Linear chip on the row and in the editor, built from `task.linear`, with no create button. The Linear link also appears in Links.
- Docs:
  - `docs/protocol.md` Native Tasks: the `linear` field, the new RPCs, and API 105.
  - `docs/guide.md` Tasks section, two how-to sections in plain steps:
    - **How tasks work:** turn on the experiment and open Tasks with Command-7, add tasks (including pasted lists and sub-tasks), use the List and Board views, and work with priority, due date, reminders, project, links, comments and Send to Thread.
    - **How tasks work with Linear (optional):** what you need first (Linear connected in Connectors), sending a task (pick a team and project, check the preview and pictures, click Create), adding existing Linear issues to Tasks (Add from Linear, including on an empty board), linking a task to an issue that already exists, what the chip and status mean, keeping a task in sync (what gets copied from Linear, what never does, the "Check Linear for changes" interval, that it costs no tokens, and that the Mac has to be awake), unlinking, what happens if Linear disconnects, and what never leaves your Mac.
  - `changelog.md` under `[Unreleased]` → `### Added`: one line on Send to Linear for tasks, which shows only when Linear is connected, with a link-and-status chip and per-task sync of status, priority, due date, title and comments. If the change shows in the app, also add a card to `docs/changelog.md` the way that file asks (a clip or picture per card).
  - `runner/src/experiments/tasks.md`: the file list.
  - A note on the `threadbear` Connectors agent and its policy rules.
- MCP: no new agent tool for creating issues. Agents already have `linear__save_issue` behind the gate. `list_tasks` returns the `linear` field so agents can see the link.

### 6. Bring Linear issues into Tasks
Works both when you already have a board of your own tasks and when Tasks is empty. Issues only come in when you pick them. Nothing is added on its own. It only shows when Linear is connected, like everything else here.

**Add from Linear** (Mac app):
- Where to find it: the Tasks toolbar, the quick-add box's menu, and the empty state ("Start from your Linear issues") when there are no tasks yet.
- It opens `desktop/gpui/src/linear_import.rs`, a picker built on `linear__list_issues`:
  - Search box. Filters for team, project, "Assigned to me" (on by default) and status (open only by default).
  - Pages through results with the cursor.
  - Each row shows `ENG-123`, the title, status and project. Issues already linked to a task show "In Tasks" and can't be picked again.
- Multi-select, then three choices:
  - **Threadbear project**, defaulting from the remembered team-to-project mapping in `linear.defaults`.
  - **Keep in sync**, ticked by default.
  - **Add**.
- Each picked issue becomes a task, already linked (`linear.state: 'linked'`):
  - Copied: title, priority, due date and mapped status.
  - The description goes into notes once, as a starting copy. After that, notes are yours and sync never touches them.
  - Existing comments go into Activity, read-only.
  - A `kind:'link'` back to the issue is added.
  - Pictures in a Linear description stay as Linear links in the notes. They aren't downloaded in this version, since Linear-hosted files may need a Linear login to open.
- Sub-issues: an "Include sub-issues as sub-tasks" checkbox, off by default. When it's on, the sub-issues come in one level deep, which matches Threadbear's own sub-task limit. Each becomes its own linked task.

**Link to an existing issue** on a task you already have, as an alternative to Send to Linear:
- In the ⋯ menu and on the task page's Linear row: "Link to Linear issue…".
- Paste `ENG-123` or an issue URL, or search with the same picker.
- It links the task without creating anything in Linear and without changing the task's notes. Sync then works as usual.

Backend:
- Convex: add an index `by_linear_issue` on `['linear.issueId']`, plus a runner-key mutation `linearImport {issues[], project, sync, includeSubIssues}`. Both import and link check that index first, so one Linear issue can only ever belong to one task. Import is retry-safe: the task id is made from the issue id, and `create` already returns early on an existing id.
- Runner: `linear.search {query?, team?, project?, assignedToMe, openOnly, cursor?}` and `linear.import {...}` / `linear.link {taskId, issue}`. These only read Linear, so no approval is ever needed. They're already allowed for the `threadbear` agent, though `list_issues`, `get_issue` and `list_comments` still need to be checked against the allow list.
- Agents: `list_tasks` already shows the `linear` field. There's no new agent tool, since an agent can call `linear__list_issues` itself and then `create_task`.

## Safeguards
- No duplicates, guarded in three layers: the Convex claim lock, the marker search before creating, and a settle that only accepts a matching `requestId`.
- Nothing goes to the wrong team or project. The project is checked against the team's own project list right before sending, the workspace name shows in the sheet, and nothing is chosen for you except the remembered defaults.
- The approval gate stays as tight as it is today. Only the `threadbear` agent's create call skips asking. Updates, comments and merges still ask, and other services and agents are unchanged.
- Nothing private goes to Linear without your OK. The sheet previews the exact text and shows every picture that will upload, each with its own checkbox. Print and thread links are never sent. Pictures are uploaded to Linear's own storage, so the private Convex storage URL never ends up in Linear, and removing a picture from the task later doesn't break the issue.
- The Threadbear Connectors login can only read Linear, upload files and create issues. Every other Linear tool is denied for it.
- No sync conflicts:
  - Sync only goes one way, from Linear to Threadbear, so there are no loops and no echoes.
  - A field changes in Threadbear only when it changed in Linear (the `last` check), so your local edits aren't undone by a sweep.
  - Older Linear answers are ignored (`issueUpdatedAt`), so two Macs can't fight.
  - Your notes are never touched.
- Sync is per task and off unless you choose it. The sweep never reads tasks that aren't synced, and it does nothing when none are synced.
- Unlink never deletes anything in Linear.
- One Linear issue can only be linked to one task (`by_linear_issue`), so importing twice or linking twice can't make copies.

## Verification
- Runner unit tests in `runner/test/unit/linear-tasks.test.ts` with a fake `CallTool`. Cover the mapping, the description builder, the project-team check, a marker hit linking instead of creating, the approval-wait path, a stale `requestId` being ignored, uploading pictures (signed headers sent exactly as given, a redirect refused, a retry reusing `uploads`, one failed picture not blocking the issue), and an API below 89 hiding the feature. Run the existing `notion-tasks.test.ts` after the move.
- Convex tests for claim, settle, `linearApply` and `linearSetSync`. Cover only changed fields applying, a stale `issueUpdatedAt` being ignored, the status mapping including canceled and reopened, comment dedupe by `externalId`, sync off meaning only the chip changes, and Linear comments refusing removal.
- A runner test for the sweep: it respects `syncEvery` (no sweep before the interval, a catch-up sweep after a sleep), one `list_issues` call per team, no follow-up calls when nothing changed, and nothing at all with zero synced tasks.
- `cargo test` for the `desktop/core` decode of `linear`.
- Run `check.sh` with `THREADBEAR_NODE` set (see memory) and read only its own log folder.
- By hand in the real app, on a throwaway Linear team or project:
  - Send a task and check the issue's team, project, priority, due date, checklist and marker.
  - Send a task with a pasted screenshot, a dropped PNG and one unticked picture. Check that the issue shows the two ticked pictures inline, that their URLs are on Linear's domain and not Convex's, and that the unticked one isn't there. Kill the runner after the first upload, retry, and check the picture isn't uploaded twice.
  - Double-click Create and check that only one issue is made.
  - Kill the runner mid-send, retry, and check that it links the existing issue.
  - On a synced task, change the issue in Linear: close it, raise its priority, set a due date, rename it and add a comment. Click Refresh, or set the interval and wait for the sweep, then check that the task shows each change, with one Activity event each and the comment read-only. Reopen the issue and check the task moves back.
  - Edit a synced task's title locally, then change only the priority in Linear. Check that the local title stays.
  - Turn sync off on one task. Change its issue and check that nothing changes on the task. Check that the sweep makes no calls once no tasks are synced.
  - From a thread, call `linear__save_issue` and check that it still waits for approval.
  - With an empty Tasks board, use "Add from Linear" and pick 3 of your assigned issues, one with sub-issues and the box ticked. Check the tasks, their sub-tasks, the notes copied once, the comments in Activity and the sync switch on. Open the picker again and check those issues show "In Tasks".
  - On an existing task, "Link to Linear issue…" with a pasted `ENG-123`. Check that nothing is created in Linear and the notes are unchanged.
  - Sign Linear out in Connectors and check that Tasks looks and works exactly as it does today, with no Linear controls and no errors in the runner log.
- Land with `tb land` and push to Wayne's fork only.
