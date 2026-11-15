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
