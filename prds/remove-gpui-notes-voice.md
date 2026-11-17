# Remove GPUI Notes voice

Created: 2026-11-16 23:58 UTC

## Scope

Wayne wants the Notes voice controls removed unless Mike currently ships them. Fetched upstream/main at 25b7e399: no Notes voice setting or dictation integration exists in app/, desktop/, or the guide. Remove the fork's GPUI Notes microphone, caption, setting, and dedicated listener. Keep thread Voice, keyboard dictation, and the configurable 1.5-second pause.

## Implementation

- Remove the Notes-only additions from 1300ba08, preserving unrelated changes and the pause preference.
- Ignore legacy voiceInNotes JSON through the existing tolerant preference decoder; preserve all remaining preferences.
- Update the guide, feature inventory, scenarios, changelog, files index, and task tracking. Swift code remains outside this GPUI change.

## Verification

- Run core Voice and keyboard dictation tests, including legacy preference decoding.
- Run GPUI cargo check through the machine build lock and verify feature pairing.
- Run the required landing check; verify the resulting installed build before reporting the controls gone from the running app.

## Status

2026-11-17 00:10 UTC: implementation verified. All 61 tests in `cargo test -p threadbear-core --test voice` pass, including keyboard dictation and legacy preference decoding. `cargo check -p threadbear-gpui`, `git diff --check`, and `node features/verify-pairing.mjs` pass (92 feature files). Evidence: `/tmp/threadbear-remove-notes-voice-check.log`, exit 0. Landed as `5592339e` after the combined `./check.sh --quick` passed in 787 seconds.

2026-11-17 02:32 UTC: release build passed in 9m33s and staged `5592339e`. Staged metadata confirms removal ancestry. Binary checks find neither `Use with Notes` nor `settings-voice-notes`; `Pause before sending`, `settings-voice-enabled`, and `settings-voice-hold` remain. Running GPUI is still `1300ba08`; automatic restart is waiting to preserve an unsent draft. Installed verification remains pending. The completion-feature thread is preparing a newer descendant build; avoid duplicate staging.


## Installed verification

2026-11-17 06:05 UTC: complete. Installed GPUI `06337397d683532bd073e4c8030e8aca4fc877d9` includes removal `5592339e`. Process PID 9065 runs the installed GPUI executable; `desktop.log` records `06:03:52.918 app started 06337397`. Installed binary checks confirm `Use with Notes` and `settings-voice-notes` absent; `Pause before sending`, `settings-voice-enabled`, and `settings-voice-hold` present. Corresponding Settings, Notes page and Voice entity source has no dedicated Notes controls/listener. This verifies deployment and compiled controls; it is not a new microphone or visual UI test. No force restart, draft change, push or upstream PR was needed.
