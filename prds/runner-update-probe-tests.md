# Runner update probe tests

2026-11-12 04:53 UTC

## Problem

The update to d3c96861 exhausted its sleep retries. The power log confirms
lid-close sleeps during the run. The network probe test also failed its
1.5-second wall-clock ceiling under load, taking 1.523 seconds. That ceiling
measures scheduling delays as well as the probe's deadline.

## Change

Drive the probe deadline and response-size checks with a controlled socket
and Node's mock clock. Keep the real TCP and TLS integration checks. Leave
the runner's sleep detection and production probe behavior unchanged.

The first full check found a second load-sensitive failure: a real Git
listing hit its 10-second security timeout in the frontmatter trust test.
It passed alone. Cap the updater's npm unit suite at four concurrent test
files so its many subprocesses do not compete across every core. Keep
production timeouts and fail-closed behavior unchanged. The four-worker run
passed all 3,091 active tests without retries. Leave check.sh unchanged:
changing it required unrelated app and iPhone gates that hit existing
compiler-silence and wall-clock test limits on this Mac.

## Verification

Run the focused probe tests and typecheck. Confirm the deadline test rejects
a missing absolute deadline and the size test rejects a missing size limit.
Land through tb land, then watch the automatic update until Following main
reports a built release waiting to start or up to date.
