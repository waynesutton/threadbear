# Runner protocol: Highlights, panels and connectors

Part of the runner protocol. [docs/protocol.md](../protocol.md) has the rules for all of it and which file each section is in.

## Highlights

The Mac app's right-hand rail (like VS Code's secondary side bar) has a
button per panel, Patches at its foot, and Highlights at its head: what a
thread has made, shared and pinned for Mike, so it doesn't scroll away
with the transcript.

```ts
type HighlightKind = "file" | "link" | "pr" | "commit"
type Highlight = {
  id: string            // "<kind>:<path, url or sha>"
  kind: HighlightKind
  title: string         // the file's name, a link's text (else host and
                        // path), "owner/repo#12", a commit's first line
  url?: string          // link, pr
  path?: string         // file: absolute
  how: "pinned" | "written" | "opened" | "mentioned" | "created" | "committed"
  note?: string         // a pin's reason, in the agent's words
  show?: true           // no longer sent: an older runner's pin asked to be shown
  sha?: string          // commit: short sha
  branch?: string       // commit
  pushed?: true         // commit: a later git push in the thread sent its branch
  seq: number           // the event it was first seen in (a pin: lastSeq then)
  lastSeq: number       // the latest event that mentioned it
  at: string            // when it was first seen
}
type ThreadHighlights = {
  highlights: Highlight[] // pins first, then the rest newest first: at
                        // most 80 (pins are always kept)
  edited: string[]      // files the thread edited, absolute, newest first,
                        // at most 200
  brief?: { text: string; at: string } // the librarian's line on where
                        // the thread is up to (see Projects)
  completion?: ThreadCompletion // the agent's task ledger (Clearer
                        // completion), while that experiment is on: the
                        // same list the done card shows, kept after Mike
                        // replies and after archiving, for the Tasks section
}
```

The runner finds them in the thread's events, with no model, the first
time a client asks and then from where it got to:

- **Files written**: a `Write` (or Codex file change) of something Mike
  would look at rather than code: `.md`, `.html`, `.pdf`, images, video,
  `.csv`, `.txt`. Code goes in `edited`.
- **Opened**: each file or URL a command passed to `open` (`open -a App`
  skips the app). A relative path is taken from the thread's folder and
  must exist.
- **Mentioned**: links in an `assistant.text` (Markdown links and bare
  URLs) and absolute or `~/` paths of files that exist (in a link or in
  backticks). A GitHub pull request's URL is a `pr`. A URL over 2,000
  characters isn't one.
- **PRs created**: pull request URLs in the output of a command that ran
  `gh pr create`.
- **Commits**: the `[branch sha] message` line `git commit` prints. A
  later `git push` that prints `a..b branch -> branch` (or `[new branch]`)
  marks that branch's commits `pushed`.

**Pins.** An agent pins with the `pin` tool: `{ path? | url?, title?,
note?, show? }` (a path relative to its folder is fine; it must exist;
a url is at most 2,000 characters).
A pin of something already found is the same item, now pinned. `show`
asks the app to open it in the pane, so an agent shows Mike a mockup
there instead of running `open`. It's a `panelRequest` (see Showing
panels) with `panel: "highlights"` and `highlight` naming the pin, so it
opens when Mike is looking at the thread, now or later, and every pin
with `show` asks again: the same page pinned after an edit comes up
again. (It was a `show` flag on the item until 8 Nov 2026, which the app
only acted on the first time it saw it, and never for a thread it hadn't
fetched yet.) The app opens the pin over Highlights, fetching the
thread's highlights first if it hasn't got that one, then calls
`threads.panelShown`; one that's gone opens Highlights. The tool waits
up to 2 seconds for that, as `show_panel` does, and says which. Only the pane: a file shown this
way never opens in its app (a project file included), and a link only
opens by itself if it's a web page, where a click on it would go. `unpin { id }` takes one off
again. The tools' descriptions say when to use them.

The Windows desktop app refuses network shares and device paths before
probing a local file, including a hover card or file search. Its resolver
checks each link or junction without following it, refuses mapped network
drives, and holds checked names against replacement while
resolving. It normalizes dot segments before following links and refuses
ambiguous names ending in a dot or space. Local links to local files keep
working. Windows reads its local file previews and folder listings under
this guard, without asking the runner to resolve the raw path again; unsafe
child links remain visible by name without probing their targets. Local
pictures are decoded from a guarded byte snapshot in the background. Reads,
directory enumeration and editor saves reuse checked handles; ordinary
writers can keep a log or document open while it is previewed.
Folder thumbnails use guarded bytes too, and omit rows whose size couldn't
be checked. A Windows drag also checks the volume and file ID, so a hard
link swapped in at the same path isn't the file that was grabbed.

**The brief.** The librarian's look (see Projects) also answers with a
line or two on where the thread itself is up to, kept on the thread and
sent as `brief`. No extra model call: it's the same look.

### Worktrees

A repo can ask for each new thread to get a git worktree of its own
(docs/thread-worktrees.md has the why and the details). A patch file's
default export says so, beside its actions:

```ts
export default {
  worktree: {
    setup: "./scripts/setup.sh",    // run in the new worktree before the first turn; optional
                                    // (one script and plain words: see "Trusting a folder's patches")
    check: "./check.sh --quick",    // run by `tb land` before main moves; optional
    teardown: "./scripts/teardown.sh", // run in the worktree's root when the thread is archived
                                    // or deleted, before the cleanup; optional (as setup)
  },
}
```

The first patch in the thread's own folder chain with a `worktree` wins;
`~/.threadbear/patches/` and project folders never count. The repo is the
git toplevel of the folder holding `.threadbear/`, and its main checkout
is the one with `main` checked out. A malformed `worktree` (not an object,
or a `setup`, `check` or `teardown` that isn't a command) is the file's `error`.

A new thread in such a repo (from `threads.create`, `create_thread` or a
schedule, unless `worktree: false`) gets, before its first turn starts:
`git worktree add -b thread/<id8> <parent>/<repo>-threads/<id8> main` from
the main checkout (`<id8>` is the thread id's first 8 characters), a `cwd`
in the same subfolder of it, `setup` run in its root (an `action.run` with
`trigger: "worktree.setup"`, under the first turn; a failure is shown and
the turn goes on), and a few lines in the agent's appended system prompt
saying where it is and that `tb land` puts it on main. If the worktree
can't be made (a leftover folder or branch of that name, no checkout on
`main`), a `system.note` says why and the thread starts in the folder
asked for. The summary carries `worktree` from then on.

`tb land` rebases first (refusing straight away on a conflict), then takes
an OS lock at `threadbear-land.lock` in the repo's common git directory,
rebases again on the main it finds, and holds the lock through the check
and fast-forward. The CLI reads the patch's check after rebasing.
Other landings of that repo wait, even from
other CLI processes; exiting releases it. Those waiting are queued in
`threadbear-land-queue/` there, and the next holder checks them all
together with its own and lands them at once (docs/thread-worktrees.md,
"Landing together"). A failed grouped check prints its exit code and the
last 40 lines of output (up to 8,000 characters) before retrying smaller
groups; it only refuses an individual landing if that landing's own check
fails. A patch load error,
or a check that changes the branch or leaves changed or untracked files,
stops the landing.
The Land command action has an hour for its wait and check, rather than
the usual ten-minute command limit. The fast-forward names the checked
commit, so it cannot pick up a later, unchecked branch tip.

Hand-merges from borrowed worktrees also take this same lock before
rebasing and checking, and keep it through the fast-forward. A plain git
merge does not take the lock; it can invalidate a queued landing's check.
Rebased combinations always need validation again, even if the changed
files do not overlap. See docs/thread-worktrees.md for the command.

`tb land`, in the worktree, refuses with uncommitted changes, rebases on
main (a conflict aborts and names the files), runs `check`, then
`git merge --ff-only` in the main checkout (refused, and nothing changed,
if main's own uncommitted changes are in the way) and prints what landed.
It pushes only in a repo listed in runner.json's `pushAfterLanding`
(docs/thread-worktrees.md, "Syncing main with GitHub"). `--dry-run` stops
before main moves.

When the thread is archived or deleted, `teardown` runs first in the
worktree's root, whether or not the branch landed (an `action.run` with
`trigger: "worktree.teardown"`, outside any turn; for a deleted thread only
a notice if it didn't run or failed). It has the action timeout, past
which it's ended (SIGKILL if SIGTERM doesn't do it), and a failure is
shown but doesn't stop the cleanup. It runs once per archive, and only
while the folder is still the thread's own worktree on its branch, not a
link to somewhere else. Then a
worktree whose branch is on main
(or has nothing of its own) is removed with its branch; otherwise both stay,
with a `system.note` saying how to land or drop them. A thread whose
worktree was removed gets a fresh one with its next message, made from main
as a new thread's is (same branch name, the patch's setup run), with a note;
only if none can be made (no patch asks for one, or making it fails, which
is said) does it carry on in the main checkout. One whose
worktree stayed carries on in it, and its next turn says (to Mike and the
agent) that the teardown ran, so what setup made may be gone.

## Showing panels

An agent puts one of the app's panels in front of Mike with the
`show_panel` tool, `{ panel, where? }`, when what he should look at is a
panel rather than a file or a link (those are `pin` with `show`). It opens
the panel, or brings it to the front wherever Mike put it. It never moves
or resizes a panel that's open: Mike's layout is his.

`panel` is `highlights`, `browser` (`threadbear.browser`) or `patches` (the thread's own; Patches only when
it has patches), one of the thread's `panels` by id or title (any case),
or `usage`, `updates` or `connectors` (the app's Usage, Updates and
Connectors panels). `reload: true` loads a
web panel's page afresh as it's shown, for an agent that has just changed
it (a patch's page served from its folder, say). `where`
says where a closed one opens: `tab` (a tab beside the thread, which
closes again when the main panel moves on, as a review's does), `left`,
`right`, `top` or `bottom` (along that side of the window), or `window`
(a window of its own). Without it, a closed panel opens where the app
would put it: a thread's panels beside its other open ones, else down
the right; Usage, Updates and Connectors under the threads list. An open panel
ignores `where`.

```ts
type PanelRequest = {
  id: string            // the request's own id
  panel: string         // "highlights", "patches", "usage", "updates", "connectors", or one of the thread's panels' ids
  title: string         // the panel's name: "Highlights", "Convex"
  where?: "tab" | "left" | "right" | "top" | "bottom" | "window"
  highlight?: string    // pin with show: the pin to open (panel is "highlights")
  reload?: true         // show_panel's reload: a url panel's page is loaded
                        // afresh (from its own address) as it's shown
  at: string            // when the agent asked
}
```

The runner checks the name (an unknown one is the tool's error, listing
what the thread has), keeps the request on the thread as `panelRequest`,
replacing an older one, and sends the summary. The app shows it once Mike
is looking at the thread: the app is in front and the thread is the one
in the main panel (a thread open in a panel of its own doesn't count, as
the side panels follow the main one). That's straight away if he already
is, or else when he next opens the thread or comes back to the app. Then
the app calls `threads.panelShown`, which clears it. A message from Mike
(typed, or from his phone; not a runner, schedule or agent message)
clears it too: he has seen the reply it went with. The tool waits up to 2
seconds for `threads.panelShown`, so its answer says whether Mike has the
panel in front of him now or will when he next looks.

A request naming a pin waits for a successful highlights answer requested
for that pin before a missing pin falls back to Highlights. Other pane or
thread updates and failed fetches leave the request waiting; later fetches
from a busy turn do not hold back the answer it needed.
Transient fetch errors are retried while Mike is looking at that thread.

During a handover the request stays with the thread on the runner that
has it, and the primary passes `threads.panelShown` on as it does any
thread's request.

`tb show-panel <threadId> <panel> --json` exposes this to local tools through
the authenticated `threads.showPanel` RPC. The primary checks the Mac app's
connection before forwarding to a draining runner. An unavailable app leaves
no request. An accepted request can be pending without changing the selected
thread, and its `shown` field says whether the app already acknowledged it.
The Mac app advertises `browserPanel: true` in `hello`, as does the GPUI app
on a Mac; a Browser request
returns unavailable for older apps that cannot promise this handoff path.

BrowserBear uses this only for `handoff` of a page in the agents' browser, which
has no in-page pill: the page's window comes to the front and the Browser panel
shows it marked Your turn with Done, including when the panel wasn't previously
open. The Mac app also opens Browser when its selected thread has a tab marked
Your turn. That waiting tab survives the one-shot request being cleared by a
message, so a later return to the thread still shows the handoff.

### The Browser panel

The Mac apps' Browser panel (Swift's `AgentBrowserPanel.swift`, GPUI's
`desktop/gpui/src/agent_browser.rs`) uses BrowserBear's existing private Unix socket
(`~/Library/Application Support/BrowserBear/browserbear.sock`). It lists the
pages a thread is using, each with a still and a Show window button. It doesn't
stream pages or send input to them (since 8 Nov 2026): the page is used in its
own real window, in the agents' browser (Helium) or in one of the user's Chrome
profiles.

- `panelState` reports connected Chrome profiles and the pages in them that
  ordinary BrowserBear commands associated with threads. Pages in the agents'
  browser come from `agent/tabs.json` in BrowserBear's folder, which also names
  the browser (`browser`).
- `{cmd:"panelStill",thread,args:{tab,profile,width}}` returns
  `{data,at}`: a small base64 JPEG of the page, or `data: null` when there's
  none just now. BrowserBear refreshes it at most every 3 s in the agents'
  browser and every 20 s in a Chrome profile (its debugger keeps Chrome's
  "started debugging" bar up), and never takes one of a page handed to the user
  in their own Chrome.
- `{cmd:"panelShow",thread,args:{tab,profile}}` brings the page's real window
  to the front.
- `markDone {tab}` is the Done button for a page waiting on `handoff --wait`.

BrowserBear checks that the thread owns the page in that exact profile (the agents'
browser is `agent-chrome`), as the same tab number can be in two. Replies use
`{id,ok,result|error}`. No cookies, passwords, OAuth tokens or browser profile
files are read or imported, and stills stay in memory on this Mac. Both sides
ignore fields they don't know.

## Profile page

A public page about Mike at `<site>/profile`, and at `<site>/` for
visitors when `home` is on, on his own Convex deployment's site. A
deployment is one person's, so there's no handle (one sent by an older app
is ignored, and old `/@<name>` links open the page) (`runner/src/public-profile.ts`, `convex/profile.ts`,
`web/src/profile.tsx`). The app's Customize panel (Librarian page) edits
it, saving a moment after each change (a change made while one is on its
way goes once it lands); the runner keeps it in `profile.json` in its data
dir and writes the backend's one `profile` row with the owner key, at once
on an update and then whenever what it would send has changed, checked
hourly. A runner
key can't write it.

| Method | Params | Result |
|---|---|---|
| `profilePage.get` | `{}` | `ProfilePageStatus` |
| `profilePage.update` | `ProfilePageSettings` | `ProfilePageStatus`. Saved and sent now. `-32602` for more than 4 links or one that isn't http(s); `-32000` with the reason when there's no backend or it refused |
| `profilePage.remove` | `{}` | `ProfilePageStatus`. Deletes the row and its pictures from the backend, and forgets the settings |

```ts
ProfilePageSettings = {
  published: boolean      // false: only Mike's signed-in browser sees it
  home: boolean           // visitors at the site's front page see it; only while published
  name: string; bio: string
  links: { title: string; url: string }[]   // at most 4
  show: ("picture" | "background" | "activity" | "github" | "numbers" | "projects" | "memberSince" | "changelog")[]
  numbers: ("threads" | "activeDays" | "streak" | "projects" | "models")[]
  projectIds: string[]    // at most 12
  order: ("numbers" | "activity" | "github" | "projects")[]  // the page's cards, top to bottom
  picturePath?: string; backgroundPath?: string; backgroundDim?: number  // files on this Mac
  githubUser?: string
  changelogRepo?: string  // "owner/repo" on GitHub; the changelog needs it and githubUser
}
ProfilePageStatus = { profile: ProfilePageSettings | null; url?: string /* <site>/profile, or <site>/ with home */; sentAt?: string; problem?: string }
```

Only parts in `show` are sent. Activity is the count of threads started
each local day for 372 days; numbers are counts; a project is its title,
SF Symbol and thread count. Thread titles, prompts, replies, folders and
project goals, state and notes are never sent, and threads in a private
folder (`models.privateFolders`) aren't counted. The changelog sends only
`{ repo, author }`: the visitor's browser lists that author's commits on the
repo's default branch from GitHub's public API, so only public commits can
show and nothing about them is stored. Both sides ignore fields
they don't know; an older runner answers `-32601` and the app says to
update.

## Pull requests

With the `inlinePrs` experiment, a github.com PR link clicked in a reply or
in Highlights opens as a tab in the side pane rather than the browser. The
runner reads it with the gh CLI (`runner/src/github-pr.ts`), signed in as
whoever `gh auth` is on this Mac: `github.pr` runs `gh pr view`, `gh pr
diff` and `gh repo view` with `-R owner/repo` from the temp folder, never
inside a checkout, so no repo's git config or hooks run.

```ts
type PrView = {
  url, owner, repo: string; number: number; title, author: string;
  state: 'open' | 'draft' | 'merged' | 'closed';
  head, base: string; additions, deletions, changedFiles: number;
  body: string;            // markdown, as written
  createdAt: string; mergedAt?: string; closedAt?: string; mergedBy?: string;
  review: '' | 'approved' | 'changes' | 'required';
  mergeable: 'yes' | 'conflicts' | 'unknown';
  canMerge: boolean;       // gh's account has write access and it's open
  mergeMethods: ('rebase' | 'squash' | 'merge')[];   // what the repo allows
  checks: { name: string; state: 'passed' | 'failed' | 'running' | 'pending' | 'skipped'; url?: string; seconds?: number }[];  // failed first
  activity: { kind: 'comment' | 'review'; author, body, at: string; review?: 'approved' | 'changes' | 'commented' }[];  // oldest first
  files: { path: string; from?: string; additions, deletions: number; hunks: Hunk[]; binary?: boolean }[];
  filesError?: string;     // the diff couldn't be read
  filesCut?: number;       // lines left out past 20,000
  filesLater?: boolean;    // asked for with files: false
};
```

A big PR's diff is the slowest read (two seconds for 600 KB), so the app
asks `github.pr` with `files: false` and `github.prFiles` together, and
shows the PR while its files are on their way. A runner older than that
ignores `files` and answers `github.prFiles` with `-32601`, and the app
keeps the files the first answer brought.

`github.prStates` gives the coloured state of each PR link in a transcript,
one `gh pr list` per repo (its latest 300), kept a minute; `github.prAct`
forgets that repo's states so the icons catch up. None of these is in a
thread key's reads (rpc-gate.ts): a thread can't merge or comment through
the app's connection. Another runner's thread still opens its PRs through
this Mac's gh.

## Connectors

Mike's agents get MCP services from his Connectors gateway on Bruce
(~/dev/me/connectors) and from claude.ai's connectors on each login.
Either can need signing in again. The runner watches the gateway so the
app can show what's signed in, and turns a sign-in error in a thread into
an ask (below).

**Where the gateway is.** runner.json's `connectors.url` (any URL on it;
the base is used), else the `connectors` MCP server's `url` in the main
login's `~/.claude.json` (only that field is read). `connectors.url: ""`
or `connectors.enabled: false` turns it off, as does
`THREADBEAR_CONNECTORS_URL=off` (tests); `THREADBEAR_CONNECTORS_URL`
set to a URL wins over both. Its calls carry the gateway's token, so the
URL must be `https:`, or plain `http:` only to this machine (`localhost`,
`127.0.0.0/8`, `[::1]`), with no name or password in it
(`runner/src/secure-url.ts`). One that isn't is not used, and doesn't
fall back on the next place: the gateway stays off, `connectors.status`
says why (`error` with state `off`) and so does the log. Every request to
it (`/health` too) refuses redirects, so a 307 can't pass the call on.

**Local services.** With Local connectors on and the gateway on this
Mac, the runner reads the ids of the `type: "stdio"` upstreams in
plugbear's `config/upstreams.json` (or the example it falls back to; only
`id` and `type`), as `/health` doesn't say. Those services carry `local:
true` and never a `connectUrl` (there's nothing to sign in to); one in
`error`, `connecting` or `unknown` carries `stateNote`, the catalog's
line for it or a plain "Not running". The app shows no Connect button for
them. Older apps ignore both.

**Status.** The runner asks the gateway's open `GET <base>/health` every
3 minutes, and when the app passes `refresh` to `connectors.status`
(the panel opening, or its refresh button). One ask at a time, 5
seconds at most. The app never calls the gateway itself.

```ts
type ConnectorsStatus = {
  // off: no gateway set up. checking: no answer yet. unreachable: the
  // last ask failed, and error says how in a few words.
  state: "ok" | "unreachable" | "off" | "checking"
  url?: string          // the gateway's base, https://host:port
  checkedAt?: string    // when the last ask finished
  error?: string
  version?: string      // the gateway's, from /health
  release?: string
  services: ConnectorService[]   // empty unless ok
}

type ConnectorService = {
  id: string            // "notion"
  title: string         // "Notion"; from the id when the gateway sends none
  // unknown: a state word the runner doesn't know (message says more).
  state: "connected" | "signed_out" | "connecting" | "error" | "disabled" | "unknown"
  message?: string
  connectUrl?: string   // where Mike signs it in; set on anything not connected or disabled
}
```

`/health` is read leniently: `upstreams` may be a map of id to state
word (the first gateway: `ready`, `needs_sign_in`, `idle`, `connecting`,
`error`, `disabled`) or a list of `{ id, title?, state, message?,
connectUrl? }`, and unknown fields are ignored. A service with no
`connectUrl` of its own gets `<base>/admin/connect/<id>`. A relative one
is made absolute against the base; anything but http(s) is dropped.

The app's Connectors panel lists the services with their state and a
Connect button (Reconnect for one in `error`) that opens `connectUrl` in
the browser. Its icon in the activity bar gets the accent dot while a
service is `signed_out` (only Mike can fix that), and a grey dot while the
gateway is unreachable or a service is connecting or in error (those
usually sort themselves out).

**Sign-in errors in a thread.** A sign-in problem comes from where a tool
result came from and whether the call failed, never from what the result
says: what a connector reads (an email, a Notion page) is anyone's words,
and an email saying "Gmail needs signing in again" made an ask until 5
Nov. So (`connectors.ts`, `signInSignal`):

- claude.ai's: a failed call to one of that server's own tools
  (`mcp__claude_ai_Gmail__*`) whose result starts with the CLI's auth
  error, `MCP server "claude.ai Gmail" needs you to sign in again`
  (optionally after `Error:`). The service is the name without
  `claude.ai`; the page is `https://claude.ai/settings/connectors`,
  signed in as the thread's login. A result that worked never counts.
- the gateway's: a call to a signed-out service's stub tool
  `mcp__connectors__<prefix>__sign_in`, or a failed call to any of a
  service's tools (`mcp__connectors__<prefix>__*`, not the gateway's own
  `connectors__*`). That only names candidates: the prefix, and the id in
  any `/admin/connect/<id>` link in the result (a prefix can differ from
  its id, `cf` for `cloudflare`). The runner then asks the gateway's
  `/health` afresh, and only a candidate it lists as `signed_out` is a
  problem, with `/health`'s title for it and its connect page. No gateway
  set up, or no answer from `/health`: no problem, and the agent's own
  reply is all Mike sees.

When the thread's agent calls `end_turn` and the turn so far has a
sign-in problem that no ask covers (an ask covers it when one of its
links is the page, or its action or why names the service), `end_turn`
is refused once that turn, with an error result that starts `Not ended
yet: this turn ran into`, names each service and its page, and asks for
an ask with that page as a link if the work needs it. Called again, it's
taken as it is. Nothing new reaches the app: the ask is an ordinary one.

### Gateway approvals

A call the gateway holds for approval (its policy says `ask`: a Slack
post, an email) gets a card in the thread that made it, when the gateway is
on this Mac (`gatewayIsLocal`) and has `/api/approvals` (plugbear's local
mode; its `/health` says `"approvalsApi": 1`). The runner reads the key from
the gateway's `secrets/approvals-key` itself, sends it only to that loopback
address, and never gives it to an agent (gateway-approvals.ts). Without the
API or the key, nothing changes: the gateway's own page is the only way.

- Which thread: when a thread calls a gateway tool
  (`mcp__connectors__<service>__<tool>`, not `connectors__*`), the runner
  reads `GET /api/approvals` every 1.5 seconds for up to 60 (the gateway
  holds a call 45 seconds before answering), and a waiting call with the
  same tool and arguments (a string the `tool.call` cut at 16 KB matches by
  its start) is that thread's. An approval id in a tool result, in a link on
  the gateway's own address (the "Waiting for ..." text), is matched too.
  Each gateway approval gets at most one card.
- The card is an ordinary `approval.requested`: `approvalId` `gw-<id>`,
  `name` the tool, `title` "Slack: send message", `input` the call's
  arguments whole, `options: ["allow", "deny"]`, and a `reason` saying the
  gateway is holding it, after where it goes when the service can say
  ("Goes to #general (C03NM8C7Z)."). For Slack, a channel id is named from
  the gateway's own `slack_read_channel` (one message, only its
  `Channel: #name (id)` line is used) as the runner, and a user or DM id is
  called a direct message without reading anything; a lookup that fails or
  takes over 3 seconds leaves it out. Clients head it from the tool's name ("Slack: send
  message?"), never from `title`. It isn't the agent's own call (its CLI
  isn't waiting on Threadbear), so a message from Mike leaves it open, and
  it outlives the turn, as a subagent's card does.
- `approvals.respond` with `allow` approves it at the gateway and sends it
  there and then (`POST /api/approvals/<id>` with `"run": true`: the gateway
  runs it as the agent that made it, once, so an Allow after the agent
  stopped waiting still sends, and an agent that checks later gets the same
  result); `deny` declines it. The card stays until the gateway answers (up
  to a minute for a send), and an error (`-32000`) leaves it up to try
  again. It closes with `allow` and "Approved and sent.", or "Approved, but
  sending it failed: <the result's first line>", or, from a gateway without
  `run`, "Approved. It sends when the agent picks it up." Mac
  only: `remote` is refused, even Deny, as the phone's answer would come
  through the backend.
- Decided on the gateway's page, or expired there, the card closes with
  `allow`, `deny` or `expired` at the next read. A runner restart closes it
  (`expired`) and leaves the call waiting at the gateway, for its page.

### Connectors setup

An experiment (`connectorsSetup`, docs/experiments.md). With it on,
`connectors.status` also carries `canSetUp: true` while there's no gateway
(`state: "off"`), and `catalog`, the services from
`runner/src/connectors-catalog.ts` the gateway doesn't list yet
(`{ id, title, url, auth, blurb, setupNote? }`), while it's `ok`. Older
apps ignore both. The app shows the plug icon with `canSetUp` too.

`connectors.setup` and `connectors.add` start a thread (`startedBy` the
runner, Full access, as Mike pressed the button) whose first message the
runner writes from the catalog, the pinned plugbear commit
(`PLUGBEAR_COMMIT`) and, for a custom server, an address it has checked
(public `https:`, no credentials, no LAN, tailnet or IP hosts) and a name
cut to plain words. Nothing else the app sends reaches the message. The
message tells the thread: never read plugbear's secrets, take anything
secret through `ask_secret` straight to its file, add only `http`
upstreams (never `stdio`), and say what it'll change first. One thread
each at a time: a second call while it's open answers `already: true`
with the same id. plugbear has no admin API, so its config files and a
restart are the thread's work; the gateway is picked up by the runner's
next restart (`connectors.url` in runner.json).

**Local servers.** A second experiment, `localConnectors`, on top of this
one. Catalog entries carry `kind`: `"remote"` (the above) or `"local"`, a
command plugbear runs on this Mac (`{ kind, id, title, launcher, package,
version, env?, ask?, blurb, setupNote?, command, asks }`; `command` is the
command line as it will run with the bare launcher name, `asks` the tools,
prefixed, that the add puts an `ask` rule on). Local entries are in
`catalog` only with it on, and so is `canAddLocal: true`. Older apps read
every entry as remote; an app that doesn't know `kind` was never sent one.
`connectors.add` takes a local entry's `id`, or `{ local: { launcher,
package, version, args?, title? } }`, checked by `localServerProblem`:
`uvx` or `npx` only, a package name its registry would take, one exact
version (no ranges, tags, git or URL specs), args from a plain charset,
and no `env`. The runner finds the launcher's absolute path itself (the
usual folders, then the login shell's `command -v`), as plugbear's
LaunchAgent PATH has no `~/.local/bin`. The thread's message carries one
`stdio` upstream (`{ id, title, prefix, type: "stdio", command, args,
env }`, the args the exact pin, with `-y` before it for npx) and its
`ask` rules, put first in `policy.json`, and allows exactly that command,
args and env and nothing more. Every remote add's message still forbids
`stdio`.

**An app of your own.** A service plugbear signs in to with a static
OAuth client (`auth: "oauth-static"`, today Slack) needs an app the user
makes, as it has no dynamic client registration.
`runner/src/static-clients.ts` and `STATIC_CLIENTS` in
`runner/src/connectors-catalog.ts` turn that into a button and a paste
form. When the gateway is this Mac's (plugbear's
`~/Library/Application Support/Connectors/config` exists and its
`settings.json` puts it at the runner's gateway address; Tailscale mode on
this Mac counts) and `secrets/clients/<id>.json` doesn't exist (only its
existence is checked), `connectors.status` puts `clientSetup: { createUrl,
redirectUrl, help }` on that service, and on its `catalog` entry. This
part isn't behind the experiment. `createUrl` for Slack is
`https://api.slack.com/apps?new_app=1&manifest_json=<manifest>`: an
internal app named Threadbear with `settings.is_mcp_enabled`, the redirect
`<publicUrl>/oauth/callback`, and the user scopes from the gateway's own
`upstreams.json` (else the catalog's), so the app and the sign-in agree. A
service with `clientSetup` gets Create, not Connect. The app's form checks
the usual formats (Slack: digits with a dot, 32 hex characters) only to
warn. `connectors.saveClient` refuses only blank, spaced or absurd keys,
writes the client by a temp file and a rename (`0600`, in a `0700`
folder), sets `enabled: true` on the upstream in `upstreams.json` (copied
from the example if missing, the upstream added from the catalog if it
isn't there, every other field kept), puts the service's own `ask` rules
first in `policy.json` (Slack: `slack__*schedule*`, which the catch-all
rules miss), and runs `launchctl kickstart -k gui/<uid>/local.connectors`.
The RPC's params aren't logged. A thread below Full access can't call it
(rpc-gate.ts; one on Full access could write those files anyway), and the
phone and website have no way to. A gateway on another Mac gets no
`clientSetup`: the Add thread's Slack steps open the same link and take
the keys through `ask_secret`.

### Outside agents

An experiment (`outsideAgents`). An agent that runs in the cloud, like
Grok Bot, reaches Threadbear as a remote MCP server on the backend:
`POST https://<deployment>.convex.site/mcp` (docs/convex-backend.md,
"Outside agents"). Its key comes from `agents.connect` and lives in the
backend's `agentKeys`, never `tokens`, so it opens no `phone:*` function.

Its commands reach the runner as `create` and `send` with
`args.agent: { keyId, name }`, which the backend sets. The runner
(`runner/src/outside-agents.ts`) takes nothing else from an agent, and
checks again: the experiment on here; a `create` always starts on `ask`
with the default folder, login and model, and `startedBy: { kind: "tool",
label: <name>, why: "started from <name>" }`; a `send` only to a thread
that key started (`<home>/runner/agent-threads.json`) that's still on
`ask`. The text arrives with `origin: "runner"`, quoted under a first line
starting `[Sent by an outside agent, ` (`OUTSIDE_AGENT_PREFIX`), so it
never counts as Mike's (`sentByAgent`). A command with a malformed
`agent` is refused, never run as his. Turning the experiment off revokes
every key.

`AgentKey`: `{ id, name, scope: "read" | "write", readAll, createdAt,
expiresAt, lastUsedAt?, expired }`.

Outside-agent `list_prints { limit?, cursor? }` and `read_print { slug, offset?, limit? }` read only published Prints, and only for keys with `readAll: true`. A restricted key gets an empty list and not-found reads. Listing returns public metadata and a cursor in newest creation order; reading returns public metadata and a bounded HTML slice as data, with a next offset when truncated. Slugs identify a published Print across runners. No local source file, recipe or path is mirrored or returned. The existing key checks, expiry, revoke and rate limits apply, including for discovery; there are no Print write tools. Unpublish or delete ends access on the next call.

### Bears

**Late tool answers** (separate `lateAnswers` experiment, off by default).
`approvals.respond` also accepts allow/deny for a tool card whose latest
resolution is `expired` within the last 24 hours. `allowAlways` means one
allow here. The runner queues the original request and the user's answer
as a runner-origin message, then resolves the card. It does not replay the
call, save an allow rule or widen the thread's mode. Already answered,
rewound, unknown and older cards are refused. Questions, secrets, plan
cards and cross-runner start requests keep their existing handling.
Remote allow retains the card's visibility checks; a card with no item id
can only be allowed on its owning Mac. The event log supplies the original
request and expiry across restarts. GPUI's deliberate late-answer control
is tracked in prds/bears-gpui.md; this also handles a click already in flight.

**Always-on runs.** With `bears` on, routines, folder webhooks and mentions
share a busy unattended thread for the bear. Running, starting, waiting,
queued and background work count; a chat the user started does not. Work
queues after the current turn. Modes and working folders must match when folding work; a
routine retries, a webhook holds its delivery, and a mention asks the
caller to retry when the running job has different access or a different folder. A mention's
reply is handed back when that mention's own queued turn finishes. Repeating routines skip while their preceding run is still busy.

Folding a mention checks the sender's saved authority against the worker's
current authority, including sandbox confinement, as direct messages do.
A routine also waits when its explicit account, pool, model or effort
differs from the worker's settings; a pinned account never folds into a
pooled thread. A late-answer retry counts as delivered only when queued or
when `turn.started` exists. If only its user message was saved, retry starts
the turn using that same row and original input, including across restarts.

`end_turn { needs: "nothing", quiet: true }` marks a bear's unattended run
as having nothing new to report. Once the turn is finished, the runner's
loose-end check may archive it. Questions, suggested follow-ups, unfinished
work, user messages, answered cards, stars and manual unarchives keep it
visible. The `quietInbox` experiment need not be on. Other threads ignore
`quiet`, and a useful bear report without it stays visible.

`bears.refreshRules { bear? }` (all unarchived bears when omitted) refreshes
only the marked always-on block in CLAUDE.md and AGENTS.md, and recognized
built-in memory tidy routines. It returns `{ bears: string[] }`. It needs
Full access and is refused from any bear's folder. `tb bears refresh-rules
[bear]` is the CLI. User text and custom routines stay intact. New and
refreshed bears read `memory/told.md`, record dated summaries of what they
told the user, and keep unfinished tasks in `memory/tasks/`, linked from
`memory/summary.md`. Directly created Claude memory tidies use Auto; Codex and agent-created bears use Ask. An authorized refresh moves built-in Claude tidies to Auto. The
tidy changes memory files only and drops completed task files.

An experiment (`bears`, `runner/src/bears.ts`). A bear is a folder,
`<defaults.cwd>/Bears/<id>/` (`~/ThreadbearThreads/Bears` when the default
folder is the home folder). The id (`researcher-7f3a`) never changes. The
folder holds `CLAUDE.md` and `AGENTS.md` (the persona, written once), a
git repo, `memory/summary.md` with `memory/{people,facts,projects}/`, and
`bear.json`, the fields its own agent may change:

```
{ name, icon?, does?, personality?, model?, effort?, avatar?, color?,
  quietHours? ("19:30-08:30"), notes? ("off"|"cheers"|"ideas"|"both") }
```

`avatar` is an Open Bot Avatar definition (`open-bot-avatar`), checked
with `validateAvatar`. `Bears/memory/` is shared by every bear. What
widens a bear, and Mike's arrangement of the list, live in the runner's
`<data dir>/runner/bears.json`, keyed by id: `permissionMode`, `poolId`,
`runner`, `pinned`, `order`, `awayUntil`, `archivedAt`, `projects`, plus
the top-level `head` (the chief of staff's id) and `seeded`.

A new thread in a bear's folder takes the bear's model and effort, and its
pool and mode from `bears.json`, where the call didn't say. One nobody is
watching (a schedule, or one with `startedBy`) starts in Ask unless
`bears.json` says otherwise. It joins the bear's `projects`. A schedule
whose folder is an away or archived bear's skips its run, and a one-off
time a bear's thread books in its quiet hours moves to their end.

| Method | Params | Result |
|---|---|---|
| `bears.list` | `{ archived?, faces? }` | `{ on, root?, bears: Bear[] }`. Makes Head Bear the first time, once. Each bear has its fields, `id`, `folder`, `state` (its `bears.json` row), `head`, `away`, `archived`, `routines` (schedules aimed at its folder, with `nextRunAt`) and `faces` (`idle`, `thinking`, `happy` SVG stills) |
| `bears.get` | `{ bear }` (id or name) | `{ bear }` |
| `bears.faces` | `{}` | `{ faces: { id, color, avatar, faces }[] }`, the presets |
| `bears.create` | `{ name, does?, personality?, icon?, model?, effort?, avatar? \| preset?, color?, quietHours?, tidy? }` | `{ bear }` |
| `bears.update` | `{ bear, ...bear.json fields }` (null clears one) | `{ bear }`; an access field is `-32602` |
| `bears.setAccess` | `{ bear, permissionMode?, poolId?, runner? }` | `{ bear }`; `-32060` from any bear's own thread |
| `bears.setHead` | `{ bear \| null }` | `{ bears }`; `-32060` from a bear's thread |
| `bears.pin`, `bears.move`, `bears.hibernate`, `bears.archive`, `bears.setProjects` | `{ bear, pinned? \| index \| until \| archived? \| add?, remove? }` | `{ bear }` (`move`: `{ bears }`). Archive pauses its routines, revokes its webhooks and takes it from its tables |
| `bears.delete` | `{ bear }` | `{ trashed }`: the folder's place in `<data dir>/trash`. Its routines are deleted and its threads archived |
| `bears.mention` | `{ bear, text, fromThreadId? }` | `{ threadId?, bear, note? }` (`note`: it's away) |

While bears are on, a thread in a bear's folder has `bear: { id, name,
icon?, color?, head? }` on its summary, for the sidebar's tile: the bear's
SF Symbol (a crown for the head bear when it has none) in white on its
colour. `bears.update` and `bears.setHead` send those threads' summaries
again, so the tile follows. The Mac app's Bear Colour menu on a bear's
thread row calls `bears.update { bear, color }`. Its composer's @ list
calls `bears.list { faces: false }` on this Mac's own connection, at most
every 20 seconds, and offers the unarchived bears (head bear first) only
for a message going to this Mac's runner: another runner resolves a
mention against its own bears. Picking one writes `@<name> `; nothing new
goes on the wire.

**Codex bears** (observed 13 Nov 2026). `permissionMode: "auto"` is
Claude Code's; a Codex thread set to it runs as Ask, so `tb bears access
--auto` on a Codex bear says so rather than refusing. Codex's built-in
`image_gen` writes into `~/.codex/generated_images` with no approval
request, in any mode (SECURITY.md).

A thread's connection (A thread's own key) may call the reads and
`create`, `update`, `pin`, `move`, `hibernate`, `setProjects` and
`mention` below Full access; the gate adds its id as `_caller`.

**Mentions** (`runner/src/mentions.ts`). A message Mike sends (no
`origin`, or `phone`) naming `@<bear name>` or `@bear:<id>` asks up to
three bears, and gets a `[Threadbear: Asked ...]` line added for the
agent. Each ask makes a thread in the bear's folder in the lower of the
bear's mode and the asking thread's, joined to the asking thread's
projects, with `startedBy { kind: "thread", label: "Mentioned by <title>" }`.
A bear with an unattended thread still running gets the next ask queued
there when access matches. When that queued turn ends, its reply goes to
that turn's askers as a message
starting `[Bear: <name> replied in thread:<id>. It's a bear's answer,
not Wayne's words.]`, quoted. A bear's own thread may ask another bear
only from a table they share, or when it's the head bear.

### Tables

An experiment (`tables`, needs `bears`; `runner/src/tables.ts`). A table
is a project with `table: { bears: string[], maxTurns? }` (projects.json,
`Project.table`), never sent to the Convex mirror. Its first folder,
`<bears root>/../Tables/<slug>-<hex>/`, has the host's `CLAUDE.md`. Its
brief has a `Bears:` line. A table with a `parentId` is a Part. The
librarian is shown a table only for a thread already in it. A thread in a
bear's folder that joins a table (`changeMembership`, `Projects.onJoined`)
seats that bear; leaving doesn't unseat it. `tables.create` sets the
project's icon to the `person.3` symbol, as the user's.

| Method | Params | Result |
|---|---|---|
| `tables.list` | `{ archived? }` | `{ on, tables: Project[] }` |
| `tables.create` | `{ title, goal?, bears?, parent?, maxTurns? }` | `{ table }` |
| `tables.seat` | `{ table, add?, remove?, maxTurns? }` | `{ table }` |

A mention from a table's thread must name a bear at it, and each of Mike's
messages there allows `maxTurns` (default 6) of them; past that it's
refused until he writes again.

### Webhooks

An experiment (`webhooks`; `convex/webhooks.ts`, `convex/hookHttp.ts`,
`runner/src/webhooks.ts`, API 99). `POST <site>/hook/tbh_<64 hex>` takes a
delivery: 405 for another method, 403 with an `Origin` header, 413 past
64 KB, 404 for a key that's unknown, revoked or expired, 429 with
`Retry-After` past 30 a minute or 50 a day per hook (repeats count), 120 a
minute in all, or 20 bad keys from one address in ten minutes. 202 takes
it; 200 `{ duplicate: true }` drops a repeat `Idempotency-Key` or
`X-GitHub-Delivery` within a day. Deliveries within the hook's
`batchSeconds` (default 10) go as one command, `create` (folder) or
`send` (thread), with `args.webhook: { hookId, name }` and
`args.payloads: { at, contentType, body }[]`, bodies scrubbed.

The runner takes such a command only for a hook in its own
`<data dir>/runner/webhooks.json`, aimed where that says, with the
experiment on. A folder run starts in Ask, or Auto if both the record and
the command say so, with `startedBy { kind: "tool", icon: "bolt.horizontal" }`.
A thread run needs that thread on Ask or Auto. The text starts
`[Webhook: "<name>"]` and says it's data, then quotes each payload,
pretty-printed, 16 KB in all. A bear that's away holds them in
`webhook-held.json` until it's back.

| Method | Params | Result |
|---|---|---|
| `webhooks.list` | `{}` | `{ on, reason?, url?, hooks }` |
| `webhooks.create` | `{ name, bear? \| cwd? \| threadId?, permissionMode? ("ask"\|"auto"), batchSeconds?, days? }` | `{ hook, url }`, the URL shown once |
| `webhooks.revoke` | `{ id? }` (none: all) | `{ revoked }` |

`webhooks.create` and `revoke` need Full access from a thread. Turning the
experiment off revokes every hook.
