# Only make Prints when I ask

## Why

Prints are on, and agents make them on their own: a progress Print for a long task (the make-print skill says to), a report when a thread thinks one helps. Wayne wants Prints kept, but made only when he asks for one. Turning the `pages` experiment off removes Prints entirely, so it doesn't do that.

## What changes

A setting, off by default: **Only make Prints when I ask**, in Settings > General under a Prints section that shows while the Prints experiment is on. runner.json `printsWhenAsked: true`. A normal setting, not an experiment: some people will want it and some won't.

With it on:

- `create_print`'s description gains one line: make a Print only when the user asks for one. Agents stop reaching for it unprompted.
- The runner checks too, since a description is only advice. On `create_print`, it reads the thread's latest `user.message`. If that message doesn't mention a print, chart, graph, dashboard or diagram, the call puts an approval card up first: "Make the print "<title>"? You asked to approve Prints you didn't ask for." Allow makes it. Deny tells the agent the user said no.
- `update_print`, `publish_print` and the rest are unchanged. They act on a Print that already exists.

With it off, nothing changes.

## Decisions

- **A card, not a refusal.** A refusal would break a request worded in a way the check misses ("put this in something I can keep"). The card costs one click.
- **The words checked** are print, chart, graph, dashboard and diagram, singular or plural, whole words. "Page", "doc" and "report" are left out: they show up in ordinary coding requests and would wave every Print through.
- **The default is off**, so nobody else's copy changes. Wayne turns it on in his own settings.

## Where it lives

| Side | Change |
|---|---|
| Protocol | `Settings.printsWhenAsked` and `settings.update`'s `printsWhenAsked` in `docs/protocol.md` |
| Runner | `config.ts`, `settings.ts`, `cli.ts` (`tb settings`), `main.ts` passes it to the threadbear tools, `threadbear-mcp.ts` gates `create_print` |
| Mac app | `RunnerSettings.swift`, `RunnerSettingsModel.swift`, a toggle in `SettingsView.swift` |
| Docs | the guide's Prints section, `docs/forking.md`, `prds/prints-status.md` |

## Tests

- Runner, first: with the setting on, `create_print` after a message that doesn't ask asks for approval, and Deny makes nothing. After "make a chart of this" it makes the Print without a card. With it off, no card. `settings.update` saves and returns it, and rejects a non-boolean.
- Context budget: the default description is unchanged, so `context-budget.json` shouldn't move.
- Mac app: the settings decode with and without the field. `swift build`, then the focused settings tests.
