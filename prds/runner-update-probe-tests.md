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

## Verification

Run the focused probe tests and typecheck. Confirm the deadline test rejects
a missing absolute deadline and the size test rejects a missing size limit.
Land through tb land, then watch the automatic update until Following main
reports a built release waiting to start or up to date.
