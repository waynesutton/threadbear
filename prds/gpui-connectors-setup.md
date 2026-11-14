# Connectors setup in the GPUI app

Created 2026-11-14 23:30 UTC.

## Purpose

Bring the existing Swift Connectors flows into the desktop app without changing the runner contract or enabling experiments.

## Scope

- Show Set up without a gateway and open the setup thread.
- List catalog suggestions, add a public HTTPS server, and open the returned thread.
- Connect Grok Bot or Notion with explicit permissions. Explain why Slack cannot use a key. Show the new key once in copyable, masked form fields, and keep revoke available.
- Hide services, suggestions and the agent offer locally; restore them through the hidden menu. Exclude hidden services from rail indicators.
- Respect localConnectors: show pinned local packages, review the exact command and trust wording before adding, and label running local services.

## Checks

Port the pure Swift rules with Rust unit tests. Add Connectors behavior specs, verify pairing, run the focused scenarios, and inspect dark-mode before and after screenshots in a private debug app. Run the repository landing check with tb land. Do not edit the frozen Swift app.

## Result

Implemented 2026-11-14 23:55 UTC. The panel uses the existing RPCs and preserves `connectorsHidden` from Swift settings. Local server setup still depends on the runner's experiment flag. No protocol or backend changes were needed.

Verified 1,022 core tests, the automation build, the focus guard and spec pairing. A direct run of the existing GPUI driver passed setup/catalog/custom HTTPS thread opening, Grok/Notion permissions and masked key copying, Slack's explanation, revoke and revoke all, one-time-key cleanup, hide/restore with rail filtering, and local trust/cancel/add with exact-version validation. The custom trust buttons explicitly clear their state because closing a dialog directly does not invoke its close callback.

Before/after dark-mode shots and the driver script/log are retained locally in `.fixtures/connectors-port/`. Cucumber itself was not run because its packages are absent; no packages were installed. The installed app will be checked after its automatic update.
