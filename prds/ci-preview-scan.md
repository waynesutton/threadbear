# Keep bounded preview scans fast on Intel

2026-11-18 01:40 UTC

The existing 4 MiB malicious-page regression exceeded its unchanged 0.5 CPU
second limit on two Intel runners (0.5017 and 0.5398 seconds). Keep the
assertion and all security bounds intact. Use the workspace existing memchr
2.8.3 to find ASCII case-insensitive starting-byte candidates in bulk instead
of running a Rust debug iterator and lowercase conversion for every byte.
Full candidate matching stays unchanged, including overlapping matches.
Run the local_preview tests, then the complete platform CI before Tasks lands.
