# Runner protocol: Patches

Part of the runner protocol. [docs/protocol.md](../protocol.md) has the rules for all of it and which file each section is in.

## Patches

A thread's folder can give it panels (shown beside the transcript) and
actions (chips above the composer); see docs/threadbear-design/README.md.
The runner works them out from the folder (`src/patches.ts`), nothing is
stored, and they come in the summary's `panels`, `actions` and `patches`,
so a client learns of them like any other change. The runner reads the
folder again in the background when the folder changes and otherwise
every 15 seconds at most (on the next summary it sends), and sends a new
summary if anything differs, so a `.env.local` that `convex dev` has just
written, or a patch file the agent has just saved, shows up within
seconds. A patch file that appears, loads again, breaks or is waiting
to be trusted after the first read is said in the transcript as a
`system.note`.
`tb patch-check <file.ts>...` loads patch files in tb's own process and
prints each one's load error (a duplicate id, a bad field, a syntax
error) in the runner's words, or the ids it gives, without waiting for
the runner to read the folder again. It runs the file's top-level code,
as the runner's import does, but no command in it, and skips the trust
check, so a project's patch still waits to be trusted in the app.

Patches are found by walking up from the thread's folder to the home
folder (so a thread in a package of a monorepo gets the project's), plus
`~/.threadbear/patches/` for every folder, plus every folder of every
project the thread is in (see Projects). Two kinds:

**Patch files**: `.threadbear/patches/<name>.ts` (or `.js`, `.mjs`). The
runner imports the module (Node runs TypeScript as it is; a changed mtime
loads it again) and reads its default export:

```ts
// .threadbear/patches/reseed.ts
export default {
  actions: [
    { title: "Reseed dev DB", run: "npx convex run seed:reset && npx convex import --replace fixtures/dev.zip" },
    { title: "Explain the schema", prompt: "Read convex/schema.ts and explain it to me." },
    { title: "App Store Connect", url: "https://appstoreconnect.apple.com/apps" },
  ],
  panels: [{ title: "Storybook", url: "http://localhost:6006", note: "while npm run storybook is up" }],
}
```

A patch file can have an `icon` at the top, an SF Symbol name that goes
on each of its chips (`icon: "pawprint"`), so a row of chips from several
patches shows which came from where; an action's own `icon` wins.
Panels without an icon of their own get it too. Instead of a symbol it
can be a picture: a path, relative to the patch file or from `~/`, to a
`.png`, `.svg`, `.icns`, `.pdf` or `.jpg`, or to a `.app` (its icon), sent
as `iconFile` (absolute) on the patch, its actions and its panels, and
drawn in full colour. A tool patch uses its tool's own icon this way
(`icon: "../../icons/tandem.png"`), so its rail button and chips are
recognisable at a glance. Chips
are grouped by patch, and a chip's tooltip names its patch.

`run` is a command for `actions.run` to run in the thread's folder,
`prompt` a message it sends instead, and `url` a web page (http or https
only) the app opens in the browser when the chip is pressed, with
nothing added to the transcript; exactly one of the three. Use `url`,
not `run: "open https://..."`, for a link: that leaves an `action.run` in
the transcript on every press. A panel needs a
`url` with a scheme, or an `items` function, which makes it a `list`
panel: rows drawn natively, with no page to serve. `{threadId}` in a
panel's `url` is the id of the thread it's shown beside, filled in by
the runner in each thread's summary (`http://127.0.0.1:6427/?thread={threadId}`),
so the page can tell which thread it's in. A link in a `url` panel's page
to another site (another scheme, host or port) opens as a link from a
highlight does, not over the page, and the app pauses a web panel's sound
and video while it's out of sight.

```ts
  panels: [{
    title: "Worktrees", icon: "arrow.triangle.branch",
    // Called by panels.render, in the folder the patch applies from.
    items: async ({ cwd, threadId, sh }) =>
      (await sh("git worktree list")).split("\n").filter(Boolean)
        .map((line) => ({ title: line.split(/\s+/)[0], detail: line })),
  }],
```

`items` gets `{ cwd, threadId, thread, sh }`, where `sh(command)` runs a command
in `cwd` through a login shell and resolves with what it printed (it
rejects on a non-zero exit, and on output over 16 MB rather than hand
back part of it). For a project's patch trusted by its hash,
`sh` runs from the folder holding `.threadbear/` instead, and only what
trusting it covers (see "Trusting a folder's patches"); `cwd` is still
where the thread is. It returns `PanelItem`s (or a promise of
them). Give rows a `section` to group them under headings (rows of one
section next to each other, in the order you want the groups), and a
`trailing` for a date or count at the end of the line. A panel can have an `icon`, an SF Symbol name for its button on
the rail. A hook is a command the runner runs by itself, so in a
project's patch it's one script in the repo and plain words (see
"Trusting a folder's patches"):

```ts
  hooks: [
    { title: "Typecheck", on: "turn.end", run: "./scripts/typecheck.sh", edited: "**/*.{ts,tsx}" },
  ],
```

**Row toggles.** A list panel can also have `toggle`, a function that
gets the same context as `items` plus `id` and `on`. A row that `items`
gives a `toggle` (`{ id, on, icon?, title? }`) gets a switch at the end
of its title's line: the icon, lit when `on` and faint when not, with
`title` as its tooltip. A press sends `panels.toggle`, which calls the
function with the row's `id` and the new `on`, then fills the panel
again. The patch keeps the state itself (a file it writes, say) and
`items` reads it back. A row's `toggle` from a panel with no `toggle`
function is dropped, as pressing it could do nothing. `id` is whatever
the app sent back, so check it before putting it in a command. Watching a PR for auto-fix, for instance:

```ts
  panels: [{
    title: "PRs",
    items: async ({ sh }) => JSON.parse(await sh("node prs.mjs")).map((p) => ({
      title: p.title, url: p.url,
      toggle: { id: p.url, on: p.watched, icon: "wand.and.stars", title: `Auto-fix #${p.number}` },
    })),
    toggle: ({ sh, id, on }) => {
      if (!/^https:\/\/github\.com\/[\w.-]+\/[\w.-]+\/pull\/\d+$/.test(id)) throw new Error(`not a PR: ${id}`);
      return sh(`node prs.mjs --watch ${on ? "on" : "off"} ${id}`);
    },
  }],
```

**Live state.** An action or panel can also have `when` and `badge`,
functions that get the same `{ cwd, threadId, thread, matched, sh }` as `items`.
`thread` is `{ title, projects }`, each project `{ id, title }` with a
`parent: { id, title }` when it's a part, so a patch in a folder many
threads share can turn up in only some of them:
`when: ({ thread }) => thread.projects.some((p) => p.title === "Acceptance testing")`.
A rename or a change to its projects reads them again.
`when` hides it unless it returns something true (a chip that only turns
up when there's something to do); `badge` returns a count (a number, a
numeric string or a list, whose length counts), sent as the item's
`badge` when it's 1 or more and drawn on the chip or rail button. An
action can also have `label`, which returns the chip's title for now (a
string, at most 60 characters; "Merge #707"), so the title can carry what
the chip acts on; nothing, or a throw, keeps the title it was given, and
its id still comes from that title. The
runner calls them each time it reads the patches, after every turn, and
every 20 seconds for an inbox thread that has any. Commands that two of
them run in the same folder run once per read, and the output is shared
with every other thread that runs the same command in the same folder
for 10 seconds (a run still going is joined, not started again), so live
state should be about the folder, not the thread. One that throws or takes
over 10 seconds hides its item (a `when`) or gives no badge, and its
patch's `liveError` says why. A panel can also have `summary`, which
returns a line or a list of lines (at most four, 120 characters each),
sent as the panel's `summary`: a tool patch's panels get a card at the
top of Highlights with the tool's icon, the panel's title and that
summary (or its `note`), which opens the panel, so a tool's state is in
the pane that's open by default rather than only on the rail.

```ts
  actions: [{
    title: "Your turn",
    prompt: "Do my comments in Tandem, then hand back.",
    when: async ({ sh }) => JSON.parse(await sh("tandem comments --json")).comments.length > 0,
    badge: async ({ sh }) => JSON.parse(await sh("tandem comments --json")).comments,
  }],
```

**Tool patches.** A patch with `appliesTo` (a name glob, or a list of
them, like `"*.tandem"`) belongs to a tool, not a place. It applies to a
thread only where a folder from the thread's own up to home (else from
one of its projects' folders) holds a matching name, and runs in that
folder, whichever folder the patch file came from. Besides the usual
places, the runner looks for tool patches in the `.threadbear/patches/`
of every project's folders, so a tool's repo can carry its patch: Tandem's
lives in Tandem's repo, and any video folder with a `.tandem` file gets
it. The patch's `matched` lists the paths that made it apply, and
functions get them as `matched` (a folder with two `.tandem` projects
has two).

`on: "turn.end"` (the one trigger so far) runs it after each turn the
agent completed, in the thread's folder like an action, and its output
goes into the transcript as an `action.run` with `trigger`. With
`edited`, only after a turn that edited or wrote a file matching that
glob (from `edit.diff` paths and Edit and Write calls). `tell` says when
the result also goes to the agent, as a `user.message` from the runner
(`origin: "runner"`) that starts a turn: `"failures"` (the default) when
the command failed, `"always"`, or `"never"`. The message names the hook
and the patch, carries the last 8 KB of output in a code block, and asks
the agent to fix it if it's theirs. One hook is told at most twice in a
row (a hook that keeps failing is then only posted) and a message from
anyone else starts the count again, so a hook and an agent can't go round
for ever. A thread archived as its turn ended (`end_turn`'s `archive`, or
by hand while the hook runs) keeps its worktree until its hooks finish. A
failure told then brings the thread back, in that worktree. A pass stays
quiet, whatever `tell` says, and a failure that isn't told (`"never"`, or past
the limit) is only posted: either way the thread stays archived and the
cleanup goes ahead. A hook stops at the action timeout (SIGTERM, then SIGKILL
to its process group), and anything it left running in that group is stopped
too. A restart
waits for running hooks, and a thread's hooks hold it on a draining runner. A hook whose folder has gone by the time it finishes gets a
`system.note` rather than an `action.run`, and isn't told to the agent, as
its result says nothing about the code.

Ids are the file's name plus a slug of the title
(`reseed.reseed-dev-db`), and two entries in one file can't share one.
Panels from different places that would share a title, like the Convex
dashboards of two of a project's folders, each get their folder's name
after it (`Convex · web`, or `Convex · apps/web` when the folder names are
the same too), so their tabs and rail buttons say which is which. A
file whose export doesn't fit is listed in `patches` with an `error` and
gives nothing. A project's file only loads once it's trusted (below).

### The Convex panel

Convex's own dashboard wants Mike's Convex login, a browser session the
panel's web view doesn't have. So a client that can shows Convex's
[embedded dashboard](https://docs.convex.dev/platform-apis/embedded-dashboard)
instead, `https://dashboard-embedded.convex.dev`, which takes a deployment
and a deploy key rather than a login: the deployment's pages (data,
functions, files, schedules, logs, history, settings), without the team's.
The client gets the key with `panels.convexKey` and answers the page's
`dashboard-credentials-request` message with `{ type:
"dashboard-credentials", adminKey, deploymentUrl, deploymentName }`. The
Mac app does it from a script in that web view alone, which only answers
on that origin, with a website data store that isn't saved to disk. Open
in browser opens the selected deployment's full dashboard.

The Mac panel's deployment menu lists the configured deployment plus
Development and Production from `panels.convexDeployments`, showing both
the label and deployment name, after the Convex project's name when the
panel has one. It remembers the choice on this Mac by
the configured deployment, so threads in the same project share it.
Switching changes only the panel; `.env.local` and the runner's mirror
settings stay as they are. Older runners without the listing method
keep showing the configured deployment.

A deploy key is admin of its one deployment, so it never goes into a
thread's events, the summary, the log or the Convex mirror: only
`panels.convexKey`'s answer, on the local connection. The runner makes
one per deployment, named `threadbear-panel-<host>-<6 hex>` (names are unique on a deployment), and Mike can revoke
it in the deployment's settings on the Convex dashboard (the runner notices
the next time the panel opens and makes another; the panel's "New deploy
key" button does it at once).

The runner's Convex CLI is signed in once by setup, but a login can be
missing (a fresh Mac, another runner) or expire. Then `panels.convexKey`
fails with `-32040` and the Mac panel offers Sign in to Convex instead of
the CLI's error: it calls `panels.convexLogin`, opens the `url` it answers
with, shows the `code`, and polls with `check` until it's done, then loads
the dashboard. The app opens `url` only if it's Convex's own device page
(`https://auth.convex.dev/device`, no other port or user name): a runner,
maybe another Mac's, sends it, and any other answer, first or on a later
`check`, stops the sign-in with that said rather than opening it. `-32041` (the login can't reach the deployment) gets Sign in
as someone else, the same with `force`. No terminal either way.

### The Git panel

The `git` built-in's panel is a page the runner serves from its own
port, the one `/rpc` is on, under `/git/` (`src/git-panel.ts`, its page
`src/git-panel/page.html`). No new port, and nothing in the app: a client
shows it as any `url` panel. Its address is

```
http://127.0.0.1:<port>/git/?repo=<the repo's folder>&k=<key>
```

where `key` is an HMAC-SHA256 of `git-panel\0<folder>` under the
runner's token, base64url: only the runner can make one, and it opens
that one repo. The panel is in the summary, which only goes to clients
on the local connection (it isn't part of the mirror). The runner
answers `/git/` only:

- from the loopback address, with a `Host` of `127.0.0.1:<port>` or
  `localhost:<port>`, so a runner listening on the tailnet doesn't serve
  it there and a page can't reach it by DNS rebinding;
- for a repo a thread of this runner has had the panel for since it
  started, with that repo's key;
- for the API, with the key in an `X-Threadbear-Git` header, and a POST
  only as `application/json` and from no other `Origin`, which another
  page can't send without a preflight the runner never answers.

The page itself (`GET /git/`) is sent with a Content-Security-Policy that
lets it talk only to its own origin. The API, under `/git/api/`:

| call | body | does |
|---|---|---|
| `GET state?repo=` | | the branch, upstream, remote, ahead and behind (as of the last fetch), commits no remote has, the staged, changed and conflicting files, the last 8 commits (a pushed one with its web link), a rebase or merge in progress, when it last fetched, `push` (`{ ok }` or `{ ok: false, why }`) and whether `generate` is there. `{ problem }` instead for a repo the panel won't read |
| `POST stage`, `unstage` | `{ repo, files }` or `{ repo, all: true }` | `git add` or `git restore --staged` (`git rm --cached` before the first commit). A file is a path in the repo: one that's absolute or has `..` is dropped |
| `POST commit` | `{ repo, message }` | commits what's staged, with the repo's hooks |
| `POST push` | `{ repo }` | pushes to the upstream, or publishes the branch to its remote with `-u` |
| `POST sync` | `{ repo }` | fetch, `pull --rebase --autostash`, then push. On a clash it aborts the rebase and lists the files |
| `POST fetch` | `{ repo }` | `fetch --prune` |
| `POST generate` | `{ repo }` | `{ ok, text }`: a commit message for the staged diff (the first 60,000 characters) and the last 12 subjects, from the runner's Models (`generate`, Haiku on the pool, feature `git-panel`), never for a private folder |

Each answers `{ ok, message }`. Git runs with `execFile`, never a shell,
and paths after `--`. What the runner reads by itself (state, the chips'
and badge's counts, the diff for generate) runs with `liveCommandEnv`'s
settings (Shell allowlist), so the repo's own config can't make it run a
hook, fsmonitor or anything else, and not at all in a repo whose own
config sets a `filter`, `diff` or `merge` driver. What's pressed runs as
git in a terminal would, hooks and all, with no prompt or editor. Nothing
force-pushes or passes `--no-verify`.

A push (and Sync's) goes only to a remote whose owner, from its URL, is
allowed: `runner.json`'s `gitPanel.pushOwners` (case aside; `"*"` for
any), or with none set, the GitHub login `gh config get user -h
github.com` names, for a `github.com` remote. Anything else is refused
with why, and the chips' prompts tell the agent the same rule and to ask
first. `gitPanel.chips` is `"when-needed"` (the default), `"always"` or
`"off"`.

## Plans as files

Each Plan first `plan.proposed` (see Plan first) is also saved as
`plans/<YYYY-MM-DD>-<project or folder>.md` in the thread's folder (`-2`,
`-3` for more the same day), under a short header, pinned in Highlights
with `show` (so the app opens it in the side pane), with a `system.note`
"Saved the plan in [plans/…](<path>)." whose link opens it there. A
revision before Go ahead overwrites the same file ("Updated the plan in
…"; `planFile` on the thread record); after Go ahead the next plan is a
new file. A note with a Markdown link stays out of its turn's folded work.

## Skills

Agent skills (a folder with a `SKILL.md`, as Claude Code and Codex read
them) that Threadbear keeps for every thread, on every login, whatever the
folder. The runner owns them (`src/skills.ts`); the app's Skills page lists,
imports and changes them through `skills.*`.

```ts
type Skill = {
  name: string          // the SKILL.md's `name` (lower case, digits, -): its slash command, /name
  description: string   // the SKILL.md's description
  always: boolean       // true: agents use it whenever it fits; false: only /name runs it
  source: { kind: "github", repo: string /* owner/name */, ref?: string, path?: string, url: string }
        | { kind: "builtin" }  // shipped in the runner (runner/skills/)
        | { kind: "custom" }   // written in the app (skills.save); commit "local-<hash 7>"
  commit: string        // what it's pinned to: a git sha, or "builtin-<runner version>"
  dir: string           // its folder in the store, absolute
  addedAt: string
}
type SuggestedSkill = { id: string, name: string, description: string, url?: string }
```

**The store** is `<data dir>/skills/`: `skills.json` (the list), each
skill's files in `<name>@<commit 7>/` exactly as fetched, and `plugin/`,
made from them. A new version of a skill is a new folder; the old one goes
once nothing points at it.

**Claude.** `plugin/` is a Claude Code plugin (`.claude-plugin/plugin.json`,
name `threadbear`) with one folder per skill under `skills/`: its
`SKILL.md` copied with `disable-model-invocation: true` added when `always`
is false, and every other file a link into the store. The copy keeps only
the frontmatter keys that describe the skill (`SKILL_KEYS_KEPT` in
`src/skills.ts`: name, description, when_to_use, argument-hint, license,
metadata and the like); `hooks`, `allowed-tools`, `mcpServers`,
`permissionMode` and anything else are left out, in every spelling, as
the plugin is outside folder trust (Trusting a folder's Claude Code
hooks). Frontmatter that isn't plain YAML with one key to a line (a quoted
key, a flow mapping, an anchor, alias or tag) or that never closes is
replaced by just its name and description. Every Claude session
gets it (the SDK's `plugins`, i.e. `--plugin-dir`), whichever login and
`CLAUDE_CONFIG_DIR` it runs on, so a skill reaches every login. `/name`
and `/threadbear:name` both run it.

**Codex.** Codex reads `$CODEX_HOME/skills/<name>`, one home per login,
so the runner links each skill's plugin folder there for every Codex login
(that folder also has an `agents/openai.yaml` with
`policy.allow_implicit_invocation: false` when `always` is false). It only
ever adds, changes or removes links that point into its own store; a real
folder or someone else's link of the same name is left alone and the
skill is skipped for that login (said in the log).

**Importing** resolves the link to a repo, a ref and a path: the ref to a
commit through `api.github.com/repos/<repo>/commits/<ref>`, then
`codeload.github.com/<repo>/tar.gz/<commit>`, unpacked to a temp folder.
A path with a `SKILL.md` is one skill; otherwise every `SKILL.md` under it
(at most three folders deep, `.git` and `node_modules` skipped) is one each.
Nothing it fetches is run. A page that isn't GitHub is read for an
`npx skills add owner/repo` line or a `github.com/owner/repo` link.

**Built-in skills** live in `runner/skills/<name>/`: Pele, make-print and bundle-report (the Prints recipes; make-print keeps its saved style in `~/.config/threadbear/print-style.json`, outside its folder). Adding one copies it
into the store and, if it has a `package-lock.json`, runs `npm ci
--omit=dev --ignore-scripts` there. **Suggested** are the built-in ones plus
a short list in `src/skills.ts` (Anatomy), each until it's added.

**Mentions.** Mike's `/name` for one of them anywhere in a message is
made to work in the thread's CLI before the agent gets it
(`src/skill-mentions.ts`; the transcript keeps what he typed): Claude Code
runs a slash command only as a message's first word, so one later on gets
a line after the message asking the agent to use that skill; Codex names
skills as `$name`, so `/name` becomes `$name`. Nothing for the store's
own while it's off. Skills the CLI finds itself (`~/.claude/skills`, the
folder's, plugins') count too, as the command list (`threads.commands`,
`folders.commands`) last found them for the thread's login and folder
(`listedSkills` in `src/runner/commands.ts`), except ones that run only
as the first word (`userOnly`). The app asks for that list when a
thread's box opens, so it's there before the first message.

A change reaches running threads too: every live CLI is given a fresh one
before its next message (the same conversation, resumed, with the new
skills), and an idle one with nothing running in the background is let
go straight away, as for a CLI update.
