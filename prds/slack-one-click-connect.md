# Slack one-click connect

Created 2026-11-19. Built 2026-11-20 (`runner/src/static-clients.ts`, the GPUI Connectors panel). Wayne asked for a PR to mikecann/threadbear too: it works on Mike's plugbear as is.

## Purpose

Slack is the only catalog connector that fails on Connect out of the box: "slack uses a static OAuth client, but secrets/clients/slack.json is missing." Slack has no dynamic client registration, so plugbear needs a Slack app the user owns. Today that means about 10 minutes of clicking through api.slack.com, guided by a thread, with three places to get it wrong (redirect URL, user vs bot scopes, the MCP toggle). This change cuts it to one Create click on Slack and one paste in Threadbear.

## What's wrong today

- The Slack steps in `addMessage` (`runner/src/connectors-catalog.ts`) never mention the MCP toggle (Features, Agents & AI Apps, Model Context Protocol). Without it the app signs in and `mcp.slack.com` still refuses it.
- They tell the user to add `search:read`. The upstream config in `upstreams.json` asks for `search:read.public`. The two should come from one place.
- The Client ID and Secret need a thread and two `ask_secret` prompts, even though the user is already looking at the Connectors panel.
- The Slack upstream ships with `"enabled": false`, and nothing in the panel says so.

## Scope

### 1. Manifest link (runner)

Slack creates an app from a prefilled manifest at `https://api.slack.com/apps?new_app=1&manifest_json=<url-encoded JSON>`. Add `slackManifest(redirectUrl, userScopes)` and `slackCreateUrl(...)` to `connectors-catalog.ts`:

```json
{
  "display_information": { "name": "Threadbear" },
  "oauth_config": {
    "redirect_urls": ["http://127.0.0.1:8795/oauth/callback"],
    "scopes": { "user": ["channels:history", "channels:read", "search:read.public", "users:read", "chat:write"] }
  },
  "settings": {
    "is_mcp_enabled": true,
    "org_deploy_enabled": false,
    "token_rotation_enabled": false
  }
}
```

- The redirect is plugbear's `publicUrl` from `settings.json` plus `/oauth/callback`, falling back to `PLUGBEAR_LOCAL_URL`. Tailscale mode gets its https address for free.
- User scopes are read from the Slack entry in `upstreams.json`, falling back to `upstreams.example.json`. One source of truth, so the manifest and the sign-in request can't drift.
- No public distribution. Slack only lets internal apps and Marketplace-listed apps use its MCP server, and an internal app is the right fit for one person's workspace.
- `connectors.status` returns `slackCreateUrl` on the Slack service while the client file is missing. The runner only checks that the file exists (`fs.stat`). It never reads it.

### 2. Paste form (GPUI Connectors panel)

On the Slack row in `desktop/gpui/src/connectors_panel.rs`, when the client is missing:

- A "Create Slack app" button opens `slackCreateUrl` in the browser.
- One line of help: "Create it, then copy Client ID and Client Secret from Basic Information."
- Two fields, Client ID (plain) and Client Secret (masked), and a Save button. Save stays disabled until both fields look right (Client ID is digits and a dot, the Secret is 32 hex characters). Don't block on that if Slack changes its formats. Show a warning instead.
- After Save, the row switches to the normal Connect button.

New runner RPC `connectors.saveClient { id, clientId, clientSecret }`:

- Only accepts an `id` whose catalog entry is `auth: 'oauth-static'`. Today that's only `slack`.
- Writes `${PLUGBEAR_DATA}/secrets/clients/<id>.json` as `{"client_id","client_secret"}`: write to a temp file with mode 0600 in the same folder, then rename. Creates `secrets/clients/` with mode 0700 if it's missing.
- Sets the upstream to `"enabled": true` in `upstreams.json`, keeping every other field as it was.
- Restarts plugbear (`launchctl kickstart -k gui/$(id -u)/local.connectors`) and waits for `/health` to list the id, the same way the add thread does.
- Leaves the secret out of RPC logs, errors and crash reports. Check the runner's RPC logging redacts `clientSecret` before this ships.
- Desktop only. The iPhone and web views must not get this form, so the secret never goes through Convex.

### 3. Thread fallback

Keep the add thread for anything the panel can't do (a remote gateway, a failed restart). Change its Slack block to open `slackCreateUrl` instead of walking through steps a to c, and add the MCP toggle check to what it verifies. Keep `ask_secret` there.

## Out of scope

- A shared Threadbear Slack app run by Mike. A desktop app can't keep a client secret, and a distributed app needs Slack Marketplace review before it can use MCP.
- The frozen SwiftUI app. GPUI only.
- Other static-client services (Google uses `secrets/clients/google.json` and has its own README flow). The RPC is generic so they can join later.

## Open questions, to settle before building

1. **Does Slack accept `http://127.0.0.1` as a redirect URL?** Yes, settled 2026-11-19. The prefilled link (`new_app=1&manifest_json=...`) created the app with the `http://127.0.0.1:8795/oauth/callback` redirect, `is_mcp_enabled: true` and the five user scopes, and Slack didn't object to any of them. Local mode works.
2. **Does Slack's MCP server want PKCE with a confidential client?** Settled 2026-11-20: plugbear's sign-in went through with no PKCE error. The gateway lists Slack as connected with 7 tools, and `slack_search_channels` returned #general. The 7 tools include `slack_schedule_message`, which no "ask" rule covered, so the setup now has to add `{"tool": "slack__*schedule*", "action": "ask"}` to `policy.json` as well. `send_message` and `send_message_draft` are already covered by `*__*send*`.
3. **Should posting be optional?** Slack manifests may support optional user scopes, so `chat:write` could be declined when the app is installed. Unverified. Until it is, keep `chat:write` in and keep plugbear's "ask" policy on posting tools.
4. **How long can the manifest URL be?** The manifest is small (well under 1 KB encoded), so this shouldn't come up, but test it in Safari and Chrome.

## Checks

- Runner unit tests in `runner/test/unit/connectors.test.ts`: the manifest uses `publicUrl` when it's set, scopes come from config, the URL encodes correctly, `saveClient` rejects non-static ids, writes mode 0600 atomically, and leaves the secret out of logs.
- Rust unit tests for the paste-form validation and row state.
- Behavior spec for the Slack row (missing client, saved, connected) and spec pairing.
- One real run against Wayne's workspace: create from the link, paste, Connect, read a channel, then try a post and confirm plugbear asks first.
- Land with `tb land` only.

## Sources

- Slack app manifest reference: https://docs.slack.dev/reference/app-manifest (`settings.is_mcp_enabled`, `oauth_config.redirect_urls`, `oauth_config.scopes.user`)
- Prefilled create link: https://docs.slack.dev/app-manifests/configuring-apps-with-app-manifests
- plugbear static clients: plugbear README, `auth.client: "static"` and `secrets/clients/<upstream>.json`

## As built, 2026-11-20

- The form is on the Slack row whenever the gateway lacks `secrets/clients/slack.json`, whatever the row's state, and on Add a service's Slack entry. It isn't behind the Connectors setup experiment.
- "This Mac's gateway" is plugbear's config folder existing here and its `settings.json` address matching the runner's gateway, so Tailscale mode on the same Mac gets it too.
- The table of such services is `STATIC_CLIENTS` in `connectors-catalog.ts`: scopes, the create link, a line of help, extra ask rules, and the usual key formats. Another service joins by adding an entry, plus its format check in `paste_check` (core) if it wants a warning.
- The add thread for Slack opens the same link and checks the MCP switch and the schedule rule.
