# Settings in the main window

Created 2026-11-15T05:20:49Z. Wayne wants GPUI Settings inside the main window.

## Behavior

- The gear, menu, shortcut and section links show the existing Settings form
  in the main content panel, leaving the sidebar and current thread intact.
- A Back button, Go Back, Esc and Close dismiss the page first. Choosing a
  thread, project or sidebar page also leaves Settings.
- Pending instruction edits save on leaving and quitting. Keyboard shortcut
  recording stops when Settings closes. Existing section controls still work.
- No second Settings window is created. Existing scripted Settings commands
  still work, targeting the main window.

## Scope

Reuse the Settings entity, form and theme. Adjust its hosting and lifecycle,
main-panel rendering, navigation and automation. No runner protocol, schema,
Swift UI, package or backend changes.

## Verification

Add behavior scenarios before implementation. Check window identity, repeated
opens, section links, Back/Esc/Close, choosing a thread, draft preservation,
pending instruction saves and keyboard recording. Build once through the
machine build lock; run focused scripted UI checks and inspect screenshots.
Run the repository landing check and verify the installed automatic update.

## Verified 2026-11-15T05:35:00Z

The old build created a different window ID; the regression failed there.
The new automation build passed the same-window check, section links, Back,
Esc, menu Close, Command-comma, selecting the current thread and sidebar page,
and preserving the thread draft. Keyboard recording cancels before Esc leaves
Settings and stops on close. General saves, device revoke and pairing/Done,
and pending instruction saves on close and app quit passed through the real
GPUI driver against the mock runner. The existing Devices spec waits 700 ms
for its moving sheet before clicking Done; the direct check uses that too.

Dark-mode screenshots were inspected. Evidence is private in
`.fixtures/settings-main/` (check.mjs, check.log, general.png and build.log).
The focus guard, feature pairing and step syntax checks pass. Cucumber is not
installed here, so its scenarios were not run; equivalent direct-driver flows
were run without installing packages. Landing and installed verification remain.

## Review follow-up, 2026-11-15 UTC

Preserve failed Instructions edits in the app while it remains open and show
the failure even after Settings closes. Reopening Settings restores the edit
and retries it. Automatic restarts wait while this in-memory draft exists.
The page must follow runner changes only when its text is
clean. Strengthen the embedding regression with rendered page visibility in
the actual main window, rather than comparing two aliases of its handle.
Add a mock save rejection, reproduce the lost-error case before the fix, and
exercise failure, reopen, retry and clean external updates with the GPUI driver.

## Review verification, 2026-11-15T07:28:00Z

The old binary failed the save-error regression. The rebuilt binary passes:
error visible after closing Settings, failed text restored and retried,
automatic restart held until saved, clean runner updates followed, failed
text kept through unrelated app refreshes, and actual Settings page drawn
only while open. The broader navigation, draft, recorder, General and Devices
flows and instruction saves on close/quit pass too.

Evidence: `.fixtures/settings-main/review-before.log`, `review-build.log`,
`review-after.log` and `review-broad.log`. Focus guard, feature pairing,
step/mock syntax and diff checks pass. Cucumber remains absent and unrun.
Full landing check is deferred while Mac load exceeds 100; installed follow-up
will follow landing. No package installs, live settings edits or push.
