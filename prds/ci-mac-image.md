# Restore working macOS system services in desktop CI

2026-11-18 01:05 UTC

CI 44200285491 passes Windows and Linux. Mac passes Rust, native bridge,
file drop and repaint checks, then hangs in two TIFF fixture icon requests
and a Terminal file-link operation; Quick Look also fails to render a docx.
The published macOS 26 Intel IconServices fault matches the icon hangs:
https://github.com/actions/runner-images/issues/14773

Use the supported macos-15-intel runner with the same test commands,
assertions, scenarios and release build. No application behavior changes.
macOS 26 arm64 previously failed allocation four times. Sequoia Intel
avoids both that capacity pool and the Tahoe Intel system-service issue.
Validate workflow diff locally and require the complete new CI run to pass
before normal Tasks landing. This is an isolated CI repair, not feature bypass.
