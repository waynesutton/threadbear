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

The full Prints navigation port landed in main at 7f375555; this branch adds panel appearance propagation. Native build passed. Four focused Prints scenarios (41 steps) passed, covering lists, cards, pane opening and the disabled experiment. Actual Chromium checks confirmed Print and Git HTML preferences and backgrounds change dark/light/dark with one page load. Git HTML was served in an isolated test server without the real Git API; repository operations were not tested. The runtime-tagged automated appearance scenario has not been run; manual runtime verification used an isolated installed runtime copy.

Windows CI run 44049768051 exposed a test that compared joined paths as literal Unix strings. The assertion now compares filesystem paths so Windows separators are valid.

Landing checks and installed-app verification remain pending.
