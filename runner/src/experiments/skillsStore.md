---
id: skillsStore
name: Skills store
title: Skills store: your own skills for every thread, on every login, Claude and Codex
added: 2026-11-15
on: false
reminder: 080eb958
---

## What it does

Skills in the bar on the left opens the Skills page: import a skill from a GitHub link, write your own with New Skill, or add a suggested one. Every thread's agent gets them, on every login, Claude and Codex alike. With Always on an agent uses a skill whenever it fits; with it off, only when you type its slash command. Turning the experiment off takes every skill away from every thread and hides Skills, and keeps your skills for when it's back on. A change reaches each thread when its agent next starts.

## Lives in

`runner/src/skills.ts` (the store, the `threadbear` plugin and the Codex links, and `SKILLS_STORE`, which reads the experiment), the `skills.*` requests in `docs/protocol.md` ("Skills"), the Skills page in `desktop/gpui/src/skills_page.rs` with its icon in the activity bar, and `features/skills-store.feature`. The Mac app's Skills page is `app/Sources/Threadbear/Views/SkillsPage.swift`; its Use the skills store switch flips this experiment through `skills.enable`. Skills you've added live in `skills/` in the runner's data folder.

## Taking it out

Delete `skills_page.rs`, its icon and page in the GPUI app, `features/skills-store.feature`, `runner/src/skills.ts` and `skillMethods` with their wiring in `runner/src/main.ts` (the `SkillStore`, `setSkillsExperiment` and the `onExperimentChange` line), the Swift app's Skills page, `SkillPicker.swift` and `SkillsModel.swift`, and `skills/` in the data folder. Codex logins keep links to skills that are gone until you remove `skills/` there.
