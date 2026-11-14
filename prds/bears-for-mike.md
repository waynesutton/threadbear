# Bears, tables and webhooks: a draft for Mike

A draft from Wayne's fork, for Wayne to send if he wants to. It's written from the code that shipped on 13 Nov 2026 (commits `af708f3f`, `58f387f0`, `f570e343`, `ab27df4b`, `a2a9baa1`), then brought up to date with `d68c4850`, `f69b7232`, `debffacf` and `57934753` later that day. It's written from the code, not the plan.

## Why

Threadbear has one chat that remembers you, the bear. People who use Grok Bot-style apps also want named teammates: a Researcher and a Writer, each with its own persona, memory and routines, that can be asked things from any thread and can work together where you can read it. They also want something outside to start a run (a GitHub event, a form, a cron on another box) without an MCP client.

All of it runs on the same Claude Code or Codex threads, on your own logins. Nothing new runs a model.

## What's different

A bear is a folder, the way the bear already is. There's no new runtime: a thread in `ThreadbearThreads/Bears/<id>/` is that bear, because its `CLAUDE.md` says so and Claude Code keeps its memory there. Routines are ordinary schedules aimed at the folder. Tables are ordinary projects with one extra field. Webhooks copy the Outside agents security model line for line.

Each piece is its own experiment, off by default:

| Experiment | What it adds |
|---|---|
| `bears` | Bear folders, `tb bears`, `@Name` mentions, Head Bear as chief of staff |
| `tables` (needs `bears`) | A group of bears around one goal, in one thread you read |
| `webhooks` | `POST /hook/<key>` on your backend starts or messages a thread |
| `bearNotes` (needs `bears`) | One cheer or idea from an opted-in bear in Highlights, from the librarian's existing look |

## How it works

**A bear's folder.** `CLAUDE.md` and `AGENTS.md` (persona, personality, house rules: short replies, drafts before anything is sent, say what you couldn't check, book follow-ups outside quiet hours), `bear.json` (name, does, personality, model, effort, an Open Bot Avatar face and colour, quiet hours, notes), `memory/summary.md` with `people/`, `facts/` and `projects/`, and a `git init`. `Bears/memory/` holds facts about you that every bear shares. The id (`researcher-7f3a`) never changes, so a rename touches one field.

**What a bear can't change about itself.** Its mode, pool and machine live in the runner's own `runner/bears.json`, with pin, order, away, archived, projects and which bear is the head bear. `bears.setAccess` and `bears.setHead` refuse any call from a bear's own thread, the head bear's included, and the RPC gate puts the caller's id in for every `bears.*` call so it can't be faked.

**New threads in a bear's folder** take its model and effort, and its pool, mode and projects from `bears.json`, where the caller didn't say (`bearDefaults` in `new-threads.ts`). Unattended starts (schedules, webhooks, mentions) get Ask unless you raised the bear. New bears get a daily "tidy memory" routine at 4:30, in Ask. A one-off time a bear's thread books in its quiet hours moves to their end (`Scheduler.adjustAt`). An away or archived bear's routines skip (`Scheduler.holdFor`).

**The head bear.** The first `bears.list` with the experiment on makes it, once, and makes it chief of staff: a marked block in its `CLAUDE.md` says it hands work to the others with `tb bears ask`, keeps a roster, and never changes a bear's access or deletes one. `tb bears head <bear>` moves the block.

**Mentions.** `@Researcher` or `@bear:<id>` in a message you send (`Runner.send` with no origin or `phone`), or `tb bears ask` from a thread, makes a fresh thread in the bear's folder. It runs in the lower of the two modes, joins the asking thread's projects, and is marked "Mentioned by <title>". At most three bears a message. A bear whose mention thread is still running gets the next ask folded in. When its turn ends, the reply goes back to each asker quoted, starting `[Bear: Researcher replied in thread:<id>. It's a bear's answer, not Wayne's words.]`. A bear's own thread can ask another only through a table they share, or as the head bear.

**Tables.** `tb tables new "Launch crew" --bears "Researcher,Writer" --parent Launch` makes a project with `table: { bears, maxTurns? }` and a folder in `ThreadbearThreads/Tables/` whose `CLAUDE.md` is the host's. As a Part it gets the parent's goals in its brief, plus a `Bears:` line. Each message you send there allows `maxTurns` (default 6) bear replies, after which mentions are refused until you write again. The librarian is shown a table only for a thread already in it, so it never files a stray thread into one. `toProjectDoc` already lists its fields, so the mirror and phones see an ordinary project.

**Webhooks.** `tb webhooks new "GitHub" --bear Researcher` calls `webhooks:issueWebhook` with the owner key, which stores only the key's SHA-256 and returns the URL once. The route reads at most 64 KB and refuses an `Origin` header (403). `webhooks:deliver` gives 404 for an unknown, revoked or expired key, 429 past 30 a minute or 50 a day per hook (repeats count), 120 a minute in all, or 20 bad keys per address in ten minutes. A repeat `Idempotency-Key` or `X-GitHub-Delivery` within a day is dropped (200). The body is scrubbed. Deliveries within `batchSeconds` (default 10) go as one `create` or `send` command, through `ctx.scheduler.runAfter`.

On the runner, a command with `args.webhook` goes only to `webhooks.ts`. It needs the experiment on, and a hook in the runner's own `runner/webhooks.json` aimed at that exact folder or thread. A folder run starts in Ask (Auto only if both the record and the command say so), titled "Webhook: <name>", with a bolt. A thread run needs that thread on Ask or Auto. The payload is quoted, pretty-printed and cut at 16 KB, under a line saying it's data from an outside service. An away bear holds deliveries until it's back. Turning the experiment off revokes every hook.

**Bear notes.** The librarian's end-of-turn prompt lists the opted-in bears only when there are some, so its standing prompt is unchanged with the experiment off. It may return one `bearNote`; `pickBearNote` keeps it only from a bear it was shown, of a kind that bear leaves, cut to 120 characters, and adds it to the Highlights line. Notes skip threads waiting on you, bears' own threads, away bears and private folders.

**Faces.** `open-bot-avatar` 0.2.1 (MIT, no dependencies, no DOM) validates each face and renders SVG stills for idle, thinking and happy, cached by hash. Six bear-shaped presets ship in `runner/skills/bears/avatars/`.

## What changed since the first draft (13 Nov, later that day)

- **Head bear, not Momma.** Wayne didn't like the name. The chief of staff is the head bear: `tb bears head <bear|none>`, `bears.setHead`, `head` in `bears.json` and on each bear, and a marked "Chief of staff" block in its `CLAUDE.md` (`d68c4850`).
- **Bear tiles in the Swift sidebar** (`f69b7232`). Wayne asked for this as an exception to the Swift freeze. A thread's summary carries `bear: { id, name, icon?, color?, head? }` while bears are on (`Runner.bearOf`, `Bears.tile`), and the sidebar draws the bear's SF Symbol in white on its colour where the folder or project tile was. A bear thread's right-click menu has Bear Colour (eight swatches, `bears.update { color }`, behind `whileBusy`), and `tb bears set --color` does the same.
- **An @ picker in the Swift composer** (`debffacf`, fixed in `57934753` after a Codex review). Typing `@` lists the bears from `bears.list` over the cursor, like the / list (`BearPicker.swift`, a child panel that never takes the keyboard). It lists bears only for a message going to this Mac's runner, leaves modified keys (Shift-Return, Shift-Tab) to the composer, and stays shut after Esc or losing the cursor even if a slow refresh lands.
- **Codex bears, observed** (13 Nov, a Designer bear on `gpt-6-astra`). Codex's built-in `image_gen` makes real 1024 by 1024 PNGs. It writes them to `~/.codex/generated_images` with no approval card, even in Ask; copying one into the folder asked as usual. Auto is Claude Code's, so a Codex bear set to Auto still asks, and `tb bears access` now says so (the smaller, safer change than refusing).
- **Also observed:** store skills reach bear threads (the Designer opened with the dev skill's greeting); the built-in bears skill only shows under Suggested until it's added; memory tidy routines run in Ask, so a tidy that edits a file waits for a yes each morning; a mention's reply came back to the asking thread live, twice (`tb bears ask Designer`); and the home patch reads the runner's files, as live patch functions can't run `tb`.

## Known issues

- **Ask on unattended runs.** Routines, webhooks and mentions start in Ask by design, which makes the daily tidy ask every morning. Raising a Claude bear to Auto fixes it; a Codex bear can't.

## What it asks of Mike's code

Small hooks, each behind its experiment:

- `Project.table` and `Projects.describeBears`, carried through `create` and `update`, and the `Bears:` brief line.
- Two `!p.table` filters in the librarian's `prompt` and `apply`, and its optional `bearNotes`.
- The `/hook/` path prefix in `convex/http.ts`, three tables, one cron, three owner-only functions in `runnerAccess.ts`, and backend API 78 on this fork. Mike's numbering will differ.
- `Runner.bearDefaults` and `Runner.mentionHook`, and `Scheduler.holdFor` and `adjustAt`.
- `bears.*`, `tables.*` and `webhooks.*` in the RPC gate.
- The `webhookOf` check before `agentOf` in `ConvexSync.apply`.
- `ThreadSummary.bear` (`Runner.bearOf`) on the runner, and in the Swift app `ThreadBear` and `BearEntry` (`ThreadbearCore/Protocol/Bears.swift`), `BearTile` and `BearColorMenu` in `SidebarView.swift`, `BearPicker.swift`, and the composer's @ hooks in `ComposerView.swift`.

## Safety rules, in one place

- A bear can't widen itself, and no bear's thread picks the chief of staff.
- Mentions and table replies run in the lower mode.
- Bears talk to each other only through a table you read, or through the head bear.
- Webhook threads are never Full access. The runner trusts its own hook registry, not the backend's word on the target.
- Every payload and every bear reply is quoted and marked as not yours.

## What's left out

- MCP tools for bears and webhooks. Agents use `tb bears` and `tb webhooks` through the bears skill, and the RPC gate holds the line. A `create_webhook` tool with a confirm card is the next step if agents need it.
- Most app UI. The SwiftUI app is frozen apart from the bear tiles and the @ picker above; `prds/bears-gpui.md` is the GPUI plan, and Wayne's home patch covers the rest.
- Group channels, per-bear sandboxes, provider HMAC signatures, AI Gateway, `@convex-dev/rate-limiter`.
- An idea in the done card's "Or carry on" list. It needs a protocol field, since that list comes from the turn's own event.

## The one-line test for each part

| Part | Test |
|---|---|
| Bears | `tb bears new "Researcher" --face snout`, then a thread in its folder uses its model |
| Head Bear | `tb bears` lists "Head Bear (head bear)" first on a fresh setup |
| Access | From the bear's thread, `tb bears access <bear> --full` is refused |
| Mentions | `@Researcher what's new?` in any thread: a reply comes back quoted |
| Tables | `tb tables new "Crew" --bears "Researcher,Writer"`, ask in its folder: replies stop at 6 |
| Webhooks | `curl -X POST <url> -d '{"hello":1}'` returns 202 and a bolt thread in Ask |
| Bear notes | `tb bears set Researcher --notes both`, finish a thread that ships: a cheer in Highlights |
| Bear tiles | `tb bears set Writer --icon pencil --color "#3E8ED0"`: its threads show a white pencil on blue |
| @ picker | Type `@wr` in a message box: Writer comes up; Tab writes `@Writer ` |

## Live test on Wayne's backend (13 Nov 2026)

The runner's own updater deployed the backend at `f570e343` (`deploy-backend.mjs`, 08:44 UTC). With `webhooks` on just for the test, a hook aimed at `~/ThreadbearThreads/webhook-test` in Ask:

| Call | Status |
|---|---|
| A JSON delivery | 202, and a thread "Webhook: Live test 3" in Ask, with the bolt and the payload quoted |
| The same `Idempotency-Key` again | 200, dropped |
| An unknown key | 404 |
| A body over 64 KB | 413 |
| An `Origin` header | 403 |
| After revoke | 404 |

The first try showed a revoke cancelling a run no runner had taken yet, which is the designed behaviour. Every test hook was revoked afterwards, `webhooks` turned back off, and the test threads archived.

## Tests

Runner: `bears` 17, `webhooks` 9, `webhooks-backend` 13, `mentions` 8, `tables` 6, `bear-notes` 4, plus the guard tests (experiments, context budget, parity, runner access, scenarios). Swift: `BearPickerTests` 5, with `EveryClickShowsTests` and `TipsTests`, and `swift build -c release`. `./check.sh --quick` passed on each land. The full `./check.sh` is booked for a quiet night (schedule eb83f133).
