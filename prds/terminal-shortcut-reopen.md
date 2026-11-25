# Reopen Terminal from the keyboard

Created: 2026-11-25 22:33 UTC

Issue: https://github.com/mikecann/threadbear/issues/112

## Problem

Pressing Command-J opens Terminal and pressing it again hides it. A third
press does nothing while the shell keeps running. Closing the panel leaves
focus on the hidden Terminal, outside the app's key dispatch tree.

## Change

Return keyboard focus to the app when its shortcut hides Terminal. Keep the
shell alive and reuse it on reopening. Preserve the existing layout and
shortcuts; no protocol or data changes are needed.

## Verification

- Add a feature scenario for repeated Command-J cycles and retained shell state.
- Run it before the fix to confirm the failure, then after the fix.
- Run the Terminal feature scenarios, Rust formatting and Clippy checks.
- Commit and land through `tb land` after its checks pass.
- Wait for the automatic app update and verify the installed revision separately.

## Status

The regression failed at the first reopen on the previous debug build.
With focus returned to the app, all 28 steps pass, including five shortcut
presses and a shell variable retained across reopening. The automation build,
Rust formatting, feature pairing and feature TypeScript checks pass.
All 35 Terminal scenarios pass (664 steps). A separate headless run confirmed
the visible/hidden sequence across five presses and a screenshot showed the
reopened panel below the conversation.

The first landing check caught the missing AppKit keyboard handoff after the
new GPUI focus request. The fix now calls `keyboard_to_app` like the existing
Terminal focus path. Desktop lint, core/client tests and both desktop build
checks passed in that run. The final automation build, focus guard and four
affected Terminal scenarios (79 steps) pass with the handoff in place.

## Landing and installation

Verified 2026-11-25 22:55 UTC:

- Fix `2f0ef927f046c633d15b03e14a582368c1362e9e` landed on local main after
  the normal `tb land` check passed in 216 seconds.
- The installed release's `build.json` identifies that exact commit.
- The running process uses the installed app bundle, started at 22:55:27 UTC;
  `desktop.log` records `app started 2f0ef927f` at 22:55:28 UTC.
- The release has no scripted-control endpoint. Shortcut behavior was verified
  on the same source in the headless automation build, not by injecting keys
  into the live app. No live keyboard, drafts or shell sessions were changed.
- Nothing was pushed or posted upstream.
