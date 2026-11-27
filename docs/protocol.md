# Runner protocol (v1)

The app and the runner talk JSON-RPC 2.0 over a WebSocket at
`ws://127.0.0.1:8797/rpc`. One JSON object per WebSocket text message.
Both sides must ignore unknown fields and unknown event types so either can be
upgraded first. Times are ISO 8601 strings in UTC. Ids are strings.
Strings are always well-formed: the runner sends half of a character past
U+FFFF (a cut through an emoji, a `\ud83e` escape on its own) as U+FFFD, since
strict JSON parsers (serde_json, Swift's) refuse the whole message over it.

Readers keep valid fields and list entries when another field or entry has
the wrong type. An unreadable identity drops that row. Routing and authorization
fields also fail closed when recovery would change the account being used or
hide what a Trust action or reply button permits. A missing login never selects
a fallback login or guesses its credential folder. Reply buttons need exactly
one readable action object; an unreadable action drops that button rather than
becoming a different action. A run's command and folder stay required and
nonempty. Other runners deliberately redact start/send briefs and destination
ids: those may be empty strings, since a press routes by the owning thread and
button ids to the runner's stored action.
Decode diagnostics name
only the type or position, never values or user text. A failed, empty or partly
unreadable local thread list is not evidence that previously listed threads were
deleted: clients keep those rows and must not run missing-thread cleanup.
Other runners' indexes carry `ready`: a fully readable remote list that is
ready may be empty after the last remote thread or runner was removed. Older
runners omit `ready` once a list has arrived.

<!-- sections: made by scripts/split-protocol.mjs from the headings below and in docs/protocol/; run it again after adding or renaming one -->
## Where each section is

The rest of the protocol is in docs/protocol/, a file per area. A section is cited by its file and name: `docs/protocol/needs-you.md, "Needs you"`.

It was all this one file once. Work made on that comes across by itself in `tb land`, each edit into the file its section went to; by hand, scripts/replay-over-format.sh does the same (docs/bringing-in-updates.md says how with an older copy of it). Never keep the old file in a conflict: put the edit in its section's file.

| File | Sections |
| --- | --- |
| this page | Ordering and reconnects, Older and newer clients, Native Tasks (experiment, backend API 100; Doing and saved orders 101), Tasks RPC contract, Tasks columns and thread launch (API 103), Tasks issue fields, sub-tasks, links and activity (API 104), Task favorites (API 108), Tasks and Linear (API 107) |
| [connection.md](protocol/connection.md) | Auth, Requests (app to runner), Warm start, Models and effort across providers, Notifications (runner to app) |
| [types.md](protocol/types.md) | Types |
| [events.md](protocol/events.md) | Events, Design reviews, Marks: the canvas, Reviews on the phone, Reply buttons |
| [needs-you.md](protocol/needs-you.md) | Needs you, The reply phase, Wait state |
| [updates.md](protocol/updates.md) | Runner updates, Mike's releases, Your changes to the old app, Restarts, App restarts, Handovers, Stopped background work, Stuck background jobs, Background wait loops, A thread's CLI, Frozen CLIs, A dialog on a Mac's screen |
| [settings.md](protocol/settings.md) | ACP agents, Settings and models, Standing instructions, Shared memory, Explain, Hibernate in words, Usage history, Sandboxed threads (taken out), Pools |
| [threads.md](protocol/threads.md) | Pictures in a message, Queued calls, Plan first, Slash commands, Context breakdown, Editing a message, Who started a thread, Who archived a thread, Search, Searching Agent Archive, What Mike last asked, What Mike first asked (taken out), Loose ends, Nested threads, Clearer completion, A quiet inbox, Inbox guess, Secrets |
| [patches.md](protocol/patches.md) | Patches, The Convex panel, The Git panel, Plans as files, Skills |
| [trust.md](protocol/trust.md) | Trusting a folder's patches, What changed, Trusting a folder's Claude Code hooks |
| [panels.md](protocol/panels.md) | Highlights, Worktrees, Showing panels, The Browser panel, Profile page, Pull requests, Connectors, Gateway approvals, Connectors setup, Outside agents, Bears, Tables, Webhooks, Prints |
| [projects.md](protocol/projects.md) | Projects, Project tasks, Schedules, Recipes, Habits, Project coordinators, Tasks and work items, Handing a project to its coordinator |
| [runners.md](protocol/runners.md) | Other runners' threads, Other runners' threads through the backend, Asking another runner, Sharing |
| [moves.md](protocol/moves.md) | Moving a thread to another runner, Sync, Devices |
| [keys.md](protocol/keys.md) | Runner keys, Virtual runners |
| [boxes.md](protocol/boxes.md) | Box keys, Box mode |
| [tools.md](protocol/tools.md) | Tools for agents |
<!-- /sections -->

## Ordering and reconnects

- Persisted events for a thread are delivered in `seq` order with no gaps.
- After a reconnect, the app calls `threads.subscribe` with the last `seq` it
  has; the runner replays the gap, then streams. `inProgress` in
  `threads.events` lets the app show a partially streamed block it missed.
  Deltas sent between a `threads.events` load and the `threads.subscribe`
  never reach the client, so it takes each streaming item's text from the
  subscribe reply's `inProgress` and appends only deltas that arrive after.
- `thread.updated` is level-triggered: the latest summary wins.
- A client that stops reading may miss transient deltas (the persisted
  event replaces them). One that falls far behind is closed with code 4408
  and should reconnect and replay from its last `seq`.

## Older and newer clients

Either side may be upgraded first, so each tolerates the other:

- An app that doesn't know an ACP agent reads `provider: "opencode"` as a
  plain string and shows it under its fallback heading.
- An app older than `hello.providers` ignores it. A newer app talking to a
  runner older than it falls back to what it knows of Claude and Codex, and
  gives a provider it doesn't know no capabilities.
- An app older than Codex support reads `provider: "codex"` as a plain
  string, an unknown tool `kind` (`image`) as `other` and `ultra` as text;
  lists Codex models under its fallback heading; and may still send a
  Claude alias for a Codex thread, which the runner corrects (above).
- A newer app talking to an older runner gets `-32601` from `clis.*` and
  leaves the CLI part out, sees no `usage` on accounts and says the numbers
  need a newer runner, and sees no Codex accounts.
- An app older than Connectors ignores `connectors.updated`, and a
  `show_panel` for `connectors` waits for Mike's next message as an
  unknown panel does. A newer app talking to an older runner gets
  `-32601` from `connectors.status` and leaves the Connectors icon out.
- An app older than Showing panels ignores `panelRequest`, so a request
  waits until Mike's next message clears it, and `show_panel` says it's
  waiting. A newer app never sees one from an older runner.
- An app older than a pin's `highlight` request opens Highlights for it,
  and no longer opens the pin by itself (the runner stopped sending
  `show`). A newer app talking to an older runner never opens a shown pin
  by itself; it's in Highlights.
- An app older than pools sends `accountId` with every `threads.create`,
  so its threads are pinned as before, and it ignores `pools`, `poolId`,
  `waitingForRoom` and `pools.updated`. A newer app talking to a runner
  older than pools sees no `pools` in `hello` (and `-32601` from
  `pools.*`), and shows logins as before.
- An app older than Devices never asks for them. A newer app talking to a
  runner older than Devices gets `-32601` from `devices.*` and says the
  Phone & browser tab needs a newer runner.

## Native Tasks (experiment, backend API 100; Doing and saved orders 101)

Tasks are an account-wide Convex-owned collection, separate from encrypted
Mac Notes and Notion-backed ProjectTasks. The Tasks experiment exposes
list/create/update/move/delete/restore and preferences operations through
the runner, with equivalent paired-browser functions. Changes subscribe
in realtime; browser mutations do not queue behind an online runner.

Each task carries a stable client-generated id (retry-safe creation), title,
notes, status (backlog/todo/doing/done; doing since API 101), optional project reference (runnerId and
project id), created/updated timestamps and manual position. Mutations
patch specified fields only. Deletion can be undone by restoring the same
id. New tasks use timestamp ordering unless create gives a position (API
99), which clients use to put a quick-added task on top. Project deletion leaves tasks
unassigned rather than deleting them.

Preferences include defaultView (list initially since API 101, or board), the
board's column order, the list's own section order (most active first),
project group order, grouping, sorting, background color/photo, photo position/fade and theme.
Changing the current view does not set defaultView: Display's default does.
Desktop Tasks is below Notes, Command-7; website is /tasks. Links require
existing account authentication. Offline views may show cached data but
mutations require connectivity.

### Tasks RPC contract

The `tasks` experiment gates desktop RPCs and agent tools. The website checks
backend API 101 before calling these functions. A runner supplies its own key;
clients cannot override credentials in an RPC. Paired browsers use their token
with the same Convex functions. This is a single-owner deployment collection.

- `tasks.list {after?, limit?}` returns `{items, next}`. Follow `next` as `after`
  until null; limit defaults to 100 and is capped at 250. Ids order pagination,
  while `position` controls display order.
- `tasks.create {id?, title, notes?, status?, project?, position?}` returns the id. Supply
  the same id on retries. Repeating creation never overwrites an existing task.
- `tasks.update {id, title?, notes?, status?, project?, position?}` patches only
  supplied fields. A null project removes the association. Status is `backlog`,
  `todo`, `doing` or `done`; checking completes, unchecking sets `todo`.
- `tasks.remove {id}` and `tasks.restore {id}` soft-delete and restore.
- `tasks.projects {}` returns `{runnerId,id,title}[]`. A project argument can
  also be its exact id or title; ambiguous names fail. `tasks.count {project}`
  and `tasks.projectCount {project}` return `{count,more}` capped at 1000.
- `tasks.preferences {}` returns defaultView, columnOrder, listOrder,
  projectOrder (`runnerId/id` keys, `none` for no project), grouping
  (`status`/`project`), sort (`manual`/`newest`/`oldest`/`title`), backgroundColor,
  backgroundPhoto, photoPosition, fade and theme. `tasks.setPreferences` patches
  those fields. columnOrder and listOrder name each status at most once, and
  come back with any missing status added (Doing after Todo), so an order an
  older client saved with three statuses still works.
- `tasks.photo {}` returns an authenticated storage capability URL or null.
  `tasks.uploadPhoto {}` returns an upload URL. `tasks.photoFile {path}` reads
  a local image, uploads it and selects it. PNG/JPEG/WebP/GIF, at most 10 MB.
- `tasks.updated {}` invalidates loaded tasks/preferences. It is not a row
  snapshot: clients requery, coalesce concurrent refreshes and retain the last
  successful snapshot on failure. There is no offline write queue.

Titles are trimmed, required and at most 500 characters; notes are at most
20,000. Times and positions here are numeric milliseconds, unlike thread ISO
timestamps. Project references include both runner and project identity.
The four agent tools are `list_tasks`, `create_task`, `update_task` and
`delete_task` (with `undo:true` to restore).

### Tasks columns and thread launch (API 103)

Tasks retain canonical `status` for old clients and completion. Optional
`columnId` identifies a renamed or custom board column. Preferences optionally
carry `columns: { id, name, status }[]`. Built-in ids retain their canonical
status, and custom columns default to Todo semantics. A status-only write from
an older client moves the task back to that canonical column. Title/notes-only
writes preserve column membership. Names never serve as ids in storage.

From API 106, a board with no saved columns starts on eight: `backlog`
Backlog, `on-deck` On Deck (todo), `doing` Shaping, `todo` Building,
`observing` Observing (doing), `done` Done, `paused` Paused (backlog) and
`canceled` Canceled (done). `tasks:preferences` always returns `columns`,
`boardColumnOrder` and `sectionOrder` in full (each column once), so clients
draw them as given and never add built-ins back. `tasks:deleteColumn {id}`
removes any column but `todo` and moves its tasks, deleted ones too, to the
status's own built-in column if it's left, else the first column on the board
with that status, else `todo`; it returns `{movedTo}`, or null if the column
was already gone. Over 200 tasks move in scheduled batches; until then
`list` and `get` show them where they're going. `setPreferences` refuses a
`columns` list that drops a column (use deleteColumn) and leaves deleted
built-in ids out of orders an older client sends. A write naming a deleted
built-in column, or only a status, goes to that status's column; a status-only
write keeps a task in its column when that column already has the status.
`columnId` also matches a column's name, case-insensitively, for agents.

Preferences also carry `boardColumnOrder` and `sectionOrder` as stable column ids;
legacy enum orders remain accepted. `tasks.startThread {id}` queues a create
command once per task, using its title, notes and project's runner/folder.
Repeated requests return the same command id. The task's image links (up to 6) go along
as `taskImages: {url}[]`, with a `[Picture N]` marker for each after the
notes. They are not `images`, so finishing the command never deletes the
task's files. The runner downloads them as it does a phone's photos, and
skips one that has gone or isn't an image rather than failing the
command. Tasks expose `threadCommandId`,
then `thread: {runnerId,id}` when the command finishes, or `threadError` on
failure. Web and native both use this backend operation; no agent starts until
the user presses Send to thread. Existing create-command permission defaults
still apply. Key callers also require the existing owner-only queueCommand
authorization; shared task editing never grants a restricted runner permission
to launch work on another machine. Paired browser tokens remain supported.
Agent create/update tools accept optional `columnId`; list_tasks
includes column definitions in preferences.

`tasks.photoData {base64}` uploads image bytes read by the native file picker’s
process. The runner checks the decoded size (10 MB) and image signature before
uploading; no file path permissions must cross between the app and runner.
`tasks.photoFile {path}` remains supported for existing clients.

### Tasks issue fields, sub-tasks, links and activity (API 104)

Linear-style issue properties. Every field is optional, so rows and clients
from before keep working; a runner refuses them (and the new operations)
against a backend older than 104 with "Update the Threadbear backend".

Task rows gain:

- `priority?: 'urgent' | 'high' | 'normal' | 'low' | null`. Missing or null is
  No priority.
- `dueDate?: string | null`, a calendar date `YYYY-MM-DD` with no time zone.
  Other strings, and dates that don't exist, are refused.
- `parentId?: string | null`: a sub-task of that task's `id`. One level deep: a
  task with sub-tasks can't become a sub-task, and a sub-task can't have any.
  Removing a parent removes its sub-tasks; restoring it restores the ones that
  went with it (not one removed before it). A sub-task restored while its parent
  is removed comes back with `parentId: null`.
- `links?: { id, kind: 'link' | 'print' | 'image' | 'thread', title, url?,
  printId?, thread?: { runnerId, id }, storageId? }[]`, at most 50 with unique
  ids. A `link` needs an http(s) `url`, a `print` its `printId`, a `thread`
  its `thread`, an `image` a `storageId` in file storage (PNG/JPEG/WebP/GIF,
  at most 10 MB). An empty title becomes the URL or the kind. List results
  add `imageUrl` to image links. An image link dropped from the array deletes
  its file.
- `commentCount?: number`, the task's comments.
- `createdBy?: { kind: 'user' | 'agent', name }`.

`tasks.create` and `tasks.update` accept `priority`, `dueDate`, `parentId` and
`links` (update's `links` replaces the array). New operations, each also a
Convex function of the same name unless said:

- `tasks.get {id}` returns one task (deleted or not) or null.
- `tasks.activity {id}` returns `{ items: { id, taskId, kind: 'comment' |
  'event', body, author: { kind, name }, createdAt }[] }`, oldest first (the
  newest 300). The backend writes events itself on create ("created the
  task") and on changes of status or column ("moved to Doing"), priority
  ("set priority to Urgent", "removed the priority"), due date ("set the due
  date to Nov 20, 2026", "removed the due date"), project ("moved to project
  Launch", "removed the project") and parent ("made this a sub-task of X",
  "removed this from X" on the sub-task; "added sub-task Y", "removed
  sub-task Y" on the parent). A client shows an item as the author's name
  then the body. Title and note edits make no event.
- `tasks.comment {id, body}` returns the new item (body trimmed, 1 to 10,000
  characters). `tasks.removeComment {id, commentId}` deletes a comment;
  repeating it, or naming an event, does nothing.
- `tasks.imageData {id, base64, name?}` and `tasks.imageFile {id, path}`
  (runner only) check and upload an image as `photoData` and `photoFile` do,
  then call `tasks.addImage {id, storageId, title?}` (Convex), which appends an
  image link and returns it; repeating it with the same file returns the same
  link. A browser uploads to `tasks.uploadPhoto`'s URL and calls `addImage`.
- `tasks.related {id, project?}` (runner only): for a task whose project is
  on this runner, `{ threads: { id, title, status, lastActivityAt }[], prints:
  { id, title, kind }[] }` from this runner's own threads and prints in that
  project, newest first, at most 10 each. Empty arrays for a task with no
  project, another runner's project, or Prints off. `project` saves the
  lookup when the client has the task.

Who did it: a runner key passes `author` on create, update and comment (the
Mac app's calls as the user, by the account's first name; an agent's tools as
`{ kind: 'agent', name: <the thread's title> }`), and never takes one from an
RPC's arguments. A paired browser is always the user, by the name the runner
last gave (`ownerName` in preferences), or "You" before one has. The runner
sends `author` only to a backend it knows is 104 or later.

Agent tools: `create_task` takes `priority` and `dueDate`; `update_task` takes
them too (null clears) and `comment`, which adds a comment as the agent.

### Task favorites (API 108)

A task takes an optional `favorite: boolean`, set and cleared with
`tasks.update { id, favorite }`. It changes nothing else: the task keeps its
column, position and project, and the Mac app also shows it under Favorites
(the list's first section, the board's first column). A runner refuses
`favorite` with a message when the backend is older than 108. No activity is
logged for it.

### Tasks and Linear (API 107)

Optional: nothing here shows or runs unless the `tasks` experiment is on, the
Connectors gateway reports Linear `connected`, and the backend is 107 or later.
`linear.status {}` answers `{ ready, why? }`, and the apps show Linear only
when `ready` (an already linked task keeps its chip as a link).

**Who talks to Linear.** The runner, through the gateway's MCP endpoint, as the
gateway's own `threadbear` agent (`runner/src/linear-tasks.ts`). Its token is
in the Keychain (`security … -s connectors -a threadbear`), read by the main
login's `headers.sh threadbear`; `runner.json` `connectors.linearHeadersHelper`
overrides the command. The gateway's `policy.json` limits that agent to
`linear__list_teams`, `list_projects`, `list_issues`, `get_issue`,
`list_comments`, `get_workspace`, `prepare_file_upload` and a create-only
`save_issue` (a `save_issue` with `id`, an update, asks; everything else is
denied). Without that agent the runner uses the main login, whose creates are
held for approval: it reports `waitingUrl` and waits on
`connectors__await_approval` for up to a day. No model is involved.

**The task's `linear` field** (`convex/taskFields.ts` `taskLinear`): `state`
(`creating`, `linked`, `failed`), `requestId` and `claimedAt` (the send's
claim), `teamId`/`teamName`, `projectId`/`projectName` (Linear uuids),
`issueId` (uuid), `identifier` (ENG-123), `url`, `status`, `statusType`,
`sync`, `issueUpdatedAt`, `syncedAt`, `error`, `waitingUrl`, `missing`, `last`
(what Linear said at the previous sync) and `uploads`. A linked task also has a
`kind: 'link'` link with id `linear-issue`. Activity items may carry
`source: 'linear'` and `externalId` (Linear's comment id); `removeComment`
refuses those. Preferences take `linear { syncEvery: '1h'|'6h'|'12h'|'1d',
lastSweepAt?, defaults: [{ projectKey, teamId, projectId? }] }`.

**RPCs** (app only; a thread's key may call only `linear.status`):

| Method | Params | Result |
|---|---|---|
| `linear.status` | | `{ ready, why? }` |
| `linear.options` | `refresh?` | `{ workspace: { name, url }, teams: [{ id, key, name, displayName }] }` |
| `linear.projects` | `teamId, refresh?` | `{ projects: [{ id, identifier, name, status }] }`, open ones |
| `linear.preview` | `taskId` | `{ title, description, priority, dueDate, subtasks, links, pictures: [{ id, title, imageUrl }] }` |
| `linear.send` | `taskId, teamId, projectId?, title?, imageIds?, sync?` | `{ started, requestId }`; the task's `linear.state` says the rest |
| `linear.refresh` | `taskId` | `{ changed }` |
| `linear.search` | `query?, teamId?, projectId?, assignedToMe?, openOnly?, cursor?` | `{ issues: [… , inTasks?], cursor? }` |
| `linear.import` | `issues: [identifier], project, sync?, includeSubIssues?` | `{ created, skipped }` |
| `linear.link` | `taskId, issue` (key or URL), `sync?` | `{ identifier }` |
| `linear.setSync` | `taskId, sync` | `null` |
| `linear.unlink` | `taskId` | `null`; never touches Linear |
| `linear.sync` | | runs a sweep now |

**Sending** claims the task (`tasksLinear:claim` refuses a done, deleted or
linked task, or one another claim under 10 minutes old holds), checks the
project is still open in the team, then searches the team's issues for the
marker line `Threadbear task <id>` and reads each hit's full description (the
search is fuzzy). A hit is linked rather than made again. Otherwise it uploads
the ticked pictures one at a time (`prepare_file_upload`, then a PUT with the
signed headers as given, no redirects), recording each in `uploads`, and calls
`save_issue` with the team, project, title, description (note, sub-tasks as a
checklist, pictures, the marker), priority (urgent 1, high 2, normal 3, low 4),
due date and web links. `tasksLinear:settle` takes the answer only for its own
`requestId`, and refuses an issue another task already has.

**Sync** reads Linear only for tasks with `sync` on. The runner wakes every 10
minutes; `tasksLinear:claimSweep` hands the sweep to one Mac once `syncEvery`
has passed, with the time of the last one. The sweep makes one `list_issues`
per team with `updatedAt` since then, and reads again (`get_issue`,
`list_comments`) only issues that changed. `tasksLinear:apply` ignores an
answer no newer than `issueUpdatedAt`, always updates the chip, and with sync
on copies a field only when it differs from `last`: title, priority (0 clears
it), due date (clearing it clears the time) and status (`triage`/`backlog` to
backlog, `unstarted` to todo, `started` to doing, `completed` to done,
`canceled` to the Canceled column when the board has one). New comments come in
once, by `externalId`. A missing issue sets `missing` and turns sync off.
Opening a task, Refresh and turning sync on refresh it at once.

**Importing** makes each picked issue a task with id `linear-<uuid>`, linked
and synced, its description copied once into the note, its comments, and its
sub-issues one level deep when asked. `by_linear_issue` keeps one task per
issue, for import and link alike.
