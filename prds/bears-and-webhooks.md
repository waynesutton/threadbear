# Bears, tables and webhooks

Plan: `~/.claude/plans/threadbear-project-this-thread-flickering-globe.md` (10 to 13 Nov 2026). Built on Wayne's fork on 13 Nov, runner side first.

## Problem

Threadbear has one chat that remembers you, the bear. It has no named teammates with their own persona, memory and defaults, no way for several of them to work on one goal where Wayne can read it, and nothing that starts a run from outside except time (schedules) and outside agents over `/mcp`.

## Scope

- **Bears** (experiment `bears`): a bear is a folder, `<threads dir>/Bears/<id>/`, with `CLAUDE.md`, `AGENTS.md`, `bear.json` and `memory/`. The id never changes; the name lives in `bear.json`. Access fields (`permissionMode`, `poolId`, `runner`) and Wayne-only state (pin, order, away, archived, projects) live in the runner's `bears.json`, where the bear can't write.
- **Avatars**: Open Bot Avatar presets, rendered to SVG stills by the runner (`open-bot-avatar` 0.2.1, MIT, no dependencies).
- **Managing bears**: rename, edit, pin, sort, hibernate, archive, remove from a project, delete to the runner trash.
- **Webhooks** (experiment `webhooks`): a secret `/hook/<key>` URL on Wayne's Convex site that starts a thread in a folder or messages a thread. Hashed keys, body cap, hand-kept limits, dedupe, batching, untrusted quoting, a runner-side recheck.
- **Mentions**: `@Name` or `@bear:<id>` in a message, or `tb bears ask`, hands the message to a bear and brings its reply back.
- **Tables** (experiment `tables`): a project with a `table` marker, a host folder and a loop guard.
- **Bear notes** (experiment `bearNotes`): the librarian's end-of-turn call may add one cheer or idea from an opted-in bear.
- A home patch for the UI until the GPUI app has it.

## Non-goals

Group channels, Prints, per-bear sandboxes or computers, phone or web bear UI (bears show there as ordinary threads), HMAC signatures for specific providers, AI Gateway, `@convex-dev/rate-limiter`, new Swift features (the SwiftUI app is frozen; app work goes in `prds/bears-gpui.md`).

## Decisions

- A bear is a folder, generalising the bear. Routines are ordinary schedules aimed at the folder.
- A bear can't widen its own access: those fields are in the runner's data dir and change only from the app, `tb` outside a thread, or a Full access thread that isn't the bear's own.
- Unattended runs (routines, webhooks, mentions) start in Ask unless Wayne raised the bear's mode. A mention never widens access: the lower of the two modes.
- Webhook threads run in Ask or Auto, never Full access.
- Limits are counted by hand, as `convex/mcp.ts` does.
- Every feature is behind its own experiment, off by default, so today's Threadbear behaves as before.

## Risks

- Merge surface with Mike's code in `sync.ts`, `projects.ts`, `librarian.ts`, `convex/http.ts` and `convex/schema.ts`. Kept to small hooks.
- A webhook URL ends up in callers' logs. Revoke and make a new one; payloads are always quoted as data.
- Mentions between bears could loop. Capped by `maxTurns` per Wayne message at a table, and by one unattended job per bear.
