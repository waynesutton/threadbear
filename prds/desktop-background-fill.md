# Fill the desktop background at every zoom

Created 2026-11-26 01:21 UTC.

## Problem

At 90% zoom, custom backgrounds on New Thread and Librarian leave a strip of the window unpainted. Scheduled and Projects use the same renderer. Tasks has its own renderer with the same sizing mistake.

## Change

Size background photos in measured window coordinates. Preserve the photo's aspect ratio, saved position, fade, and animated playback. Keep the sidebar and title bar unchanged.

## Verification

Use headless screenshots to check the top, bottom and right edges at 90% and after zooming to 125% and resizing. Run the page customization specs, Rust formatting and the normal landing check. Verify the installed build separately.

## Results

The old build left a bottom gap on New Thread, Librarian, Scheduled, Projects and Tasks at 90%. The fixed build passed all 14 customization scenarios (208 steps), including edge checks on those five pages at 90% and 125%. Rust formatting, feature pairing and the step type check passed.
