# Prints navigation and panel appearance

Started 2026-11-17T07:54:32Z.

## Problem

The installed GPUI app renders dark Print cards but opens Print and Git web panels in light mode. Embedded pages do not receive the app appearance. The GPUI port also lacks the Prints rail button and Highlights list documented in the guide.

## Scope

- Pass the resolved app appearance into embedded web panels and local Print previews, including already open pages when appearance changes.
- Restore the Prints button below Files, visible when Prints are enabled and at least one exists.
- List the current thread's Prints in Highlights and all Prints in the rail section, with kind, version and project information. Open through the existing Print preview path.
- Preserve browser permissions, script restrictions and existing navigation.

## Verification

- Focused regression tests for appearance propagation and Print listing/filtering.
- Behaviour specs for both lists and opening a Print.
- Headless native checks of light/dark Git and Print panels and appearance changes while open.
- Verify rail placement, Highlights list and empty/disabled states.
- Run required landing checks and verify installed app after automatic update before reporting completion.

## Status

Panel appearance patch implemented with a behaviour scenario for dark/light changes without reload. Pairing and diff checks pass; native compilation and runtime verification remain pending.

The requested navigation already exists in the Mac mini commits 0908dc2f and f36b14b5. Thread c67a712f is merging that complete port into main. Avoid a duplicate port here; rebase onto its landing and verify the requested rail/Highlights behaviour along with this theme fix.
