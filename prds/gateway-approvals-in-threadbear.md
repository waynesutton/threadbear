# Gateway approvals in Threadbear

Created 2026-11-20. Wayne's fork only (threadbear and plugbear). No PRs to Mike unless Wayne asks.

## Purpose

When an agent sends something through the Connectors gateway (a Slack post, an email, a Linear write), plugbear holds the call and opens a browser page to approve it. That page is a separate tab, it needs a browser sign-in cookie, and it doesn't show which thread asked. This change puts the approval card in the thread that made the call, using the card Threadbear already shows for Claude's own tool approvals.

## Design

### 1. plugbear: a small approvals API

- `GET /api/approvals`: the waiting calls, each `{ id, agent, upstream, tool, upstreamTool, args, createdAt, expiresAt, url }`.
- `GET /api/approvals/:id`: one call's `{ id, status, ... }`, so the runner can tell when it was decided on the browser page or expired.
- `POST /api/approvals/:id` with `{ "decision": "approve" | "decline", "note"? }`: the same `Approvals.decide` the page uses. A call still holding gets its real result at once.
- Auth: `Authorization: Bearer <key>`, where the key is `secrets/approvals-key` (32 random bytes, written by the gateway at start if missing, mode 0600). Compared in constant time. The Host check stops DNS rebinding. Requests with a browser `Origin` or `Sec-Fetch-Site` are refused, so a web page can never use it even with the key.
- `/health` adds `"approvalsApi": 1` so clients can detect it.

### 2. Runner: raise a card in the thread that made the call

- When a thread calls a gateway tool (`mcp__connectors__<service>__<tool>`, not `connectors__*`), the runner watches `GET /api/approvals` for about 50 seconds (the 45-second hold plus a margin). A waiting call whose `tool` matches and whose `args` deep-equal the call's input is that thread's.
- The tool result's `ap_...` id (the "Waiting for Wayne" text, or a `connectors__await_approval` call) is a second way to link it. The deep-equal match lets the card appear while the call is still held, without waiting for the hold to end.
- The card is an ordinary `approval.requested` with options `allow` and `deny`, name = the tool, title "Slack: send message", input = the arguments. It's `main: false` (the CLI isn't waiting on Threadbear, so a message from Wayne doesn't decline it) and `macOnly: true` (phone approval is a later decision, since it would route through Convex).
- Allow or Deny posts the decision to the gateway, then `approval.resolved`. While a card is open, the runner checks `GET /api/approvals/:id` every few seconds. Approved or declined on the browser page, or expired, closes the card with `answered` or `expired`.
- Only with a local gateway (`gatewayIsLocal`) that reports `approvalsApi`, and a readable key. Anything else: no card, and the browser page works as it does today.

### 3. GPUI

The existing approval card renders `approval.requested`. Check how it looks for a gateway tool and fix the heading and input display if needed. No new card type unless the existing one can't show it well.

## Out of scope

- Phone and web approvals (they'd route the decision through Convex).
- Changing which plugbear commit new installs use (`PLUGBEAR_COMMIT`). Wayne's Mac runs the fork build. Everyone else keeps the browser page until Mike's gateway has the API.
- Showing Slack channel names instead of ids. The card shows `channel_id` as the agent passed it.

## Checks

- plugbear: tests for the API (auth, refusing browser requests, list, decide, already decided, unknown id) and the `/health` flag.
- Runner: unit tests for matching a held call to a thread, the card's lifecycle (allow, deny, decided elsewhere, expired), and no card without the API or key.
- A real run: ask a thread to DM Wayne on Slack, approve it from the card, see the post land.
- Land with `tb land` only.
