# Finish verification of the 20 November merge

Created: 2026-11-20 18:23 UTC

## Scope

Finish the approved merge in threadbear-wt-bringin. Preserve Mike's Codex update check and Wayne's features. Main stays unchanged until the required review of changed checks is approved.

## Remaining repairs

- Include the two imported deployment helpers in the isolated deploy test fixture.
- Resolve the duplicate accounts.recheck RPC cases so sign-in discovery and usage refresh both remain reachable. Verify both request forms through the RPC client.
- Review the overlapping release branch without overwriting its work.

## Verification and delivery

The second full check passed every stage except deploy tests. Rerun deploy tests, lint and runner type checking after the fixture repair; run focused RPC and account tests after the dispatch repair. Keep the exact test diff available for review. Do not push or restart the live app.

## Verified 2026-11-20 18:26 UTC

Deploy stage passed in 226 seconds. All 119 focused RPC/account/access tests passed after reproducing the dispatch failure first. Final runner typecheck, lint and git diff checks passed. Earlier full-run results remain green for the unchanged desktop, app, backend and web code. Landing, deployment and installed verification remain pending. Four documented inline lint exceptions accompany the earlier web repairs; the baseline suppression file is unchanged from Mike's release.

## Approved reconciliation 2026-11-20 18:33 UTC

Wayne approved the reviewed test repairs, four inline lint exceptions and login-refresh fix, and asked to land. A separately approved landing in thread 118a19d4 was already checking 9e6db3c94. To avoid a competing release merge, preserve the original branch and apply the approved differences on thread/bring-in-approved-followup based on that commit. Keep its command-machine revocation behavior and Xcode compatibility repair. Restore Mike's original lint suppression baseline, preserving four explained inline exceptions.

The installed GPUI app at ba375628 sends .diff files to the system app under its automatic rule. The incoming release already previews code in the pane. Add exact .diff/.patch regression cases; both focused Rust tests pass. The same review is available as Markdown in Highlights for the currently installed app.

Run focused backend, web and account/access tests plus lint/typechecks, then normal tb land after the first landing finishes. Follow the automatic update without forcing a restart.

2026-11-20 18:33 UTC: Reconciled backend, web and runner typechecks, web tests, lint and focused runner suites passed. Public-profile backend passed 8 tests; exact diff/patch preview and click-routing tests passed. Ready for the normal landing check once the prior landing finishes.
