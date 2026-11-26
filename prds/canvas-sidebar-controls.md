# Keep the sidebar usable in Canvas

Started 2026-11-26 01:21 UTC.

## Problem

Canvas replaces the normal desktop dock. The bottom activity buttons still change the dock or open Settings inside it, so the user cannot see what the click did.

## Scope

Keep the fork's Canvas experiment. Leave Canvas before showing a bottom activity panel or Settings. If the chosen panel was already open behind Canvas, reveal it rather than closing it. Keep the normal toggle behavior outside Canvas. No changes to the board layout, experiment, runner or sync.

Mike's upstream `883beaeae` removes the Canvas experiment. Send no regression report that implies his current build still has this page.

## Verification

Exercise bottom rail clicks in the mocked desktop app. Cover a closed panel, one already open behind Canvas and the Settings gear. Check that panels still toggle outside Canvas.

The regression failed on the previous build: Updates left Canvas up. With the fix, all ten Canvas scenarios pass (157 steps). The automation build, Rust formatting, TypeScript checks and feature pairing also pass.
