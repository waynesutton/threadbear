# Keep the CI display across app relaunches

2026-11-17 23:15 UTC

Desktop CI 44189837807 passed Windows. Linux passed 680 scenarios but failed
the file-preview relaunch scenario when GPUI could not reconnect to Xvfb.
Keep the virtual display from resetting when its final client disconnects,
using Xvfb -noreset. The existing relaunch scenario verifies the real path.
Run the focused driver tests locally, then the unchanged platform CI suites.
Mac separately exceeded the unchanged 0.5 CPU-second parser limit by 0.0017s;
repeat it without weakening the assertion. Tasks remains gated on green CI.
