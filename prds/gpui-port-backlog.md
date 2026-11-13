# GPUI port backlog

Status: on hold. Mike froze the Swift Mac app on 12 Nov 2026 and the GPUI app in `desktop/` replaces it. He asked us to wait before porting anything, as big GPUI changes are coming. This file is the list and the prep, so the port can start later without redoing the research.

Written 12 Nov 2026 from Wayne's 136 commits under `app/` on main (`ae1dd28a`). The "In GPUI" column came from a grep of `desktop/` and `docs/gpui-port/PARITY.md` that day, and will be out of date once Mike's update lands. Refresh it first.

## Already in the GPUI app

Mike ported these himself, so they need nothing:

- Back and Forward arrows
- The quit prompt
- The ding when a thread finishes
- ⌘/ to collapse the threads panel
- The Appearance picker
- Experiments switches
- Standing instructions
- The file editor: line numbers, autosave, ⌘S, find, Markdown preview
- New Thread on a project page

## The runner side is mostly fork-only

Mike's upstream runner has none of `skills.ts`, `pages.ts`, `profiles.ts`, `public-profile.ts`, `x-search.ts`, `outside-agents.ts`, `connectors-catalog.ts` or `page-diagram.ts`, and no Ask mode on `threads.send`. Only `instructions.ts` went upstream. Any feature below that needs a runner method also needs that runner code sent to Mike first, or the GPUI port only works in this fork. Prints is already in draft PR mikecann/threadbear#61.

## Every feature

Sizes: S about a day, M a few days, L a week or more. "Spec" means a scenario in `features/*.feature`. "?" means not confirmed. ↑ means Mike already took the commit upstream.

### Prints

| Feature | Swift files | Runner API or keys | In GPUI | Spec | Size | Notes |
|---|---|---|---|---|---|---|
| Prints store, project Prints section, menu (Refresh, Edit with Agent, Edit Text, versions, move, delete) | `Pages.swift`, `Views/ProjectPage.swift` (PagesSection), `PageSheets.swift`, `Protocol/Page.swift` | `pages.list/get/edit/refresh/saveDoc/restore/restoreVersion/setProjects/delete`, `pages.updated`; experiment `pages` | no | partly | L | |
| Print card in replies, preview on top | `Transcript/PageCards.swift`, `RichRenderer.swift`, `Markdown/RichBlock.swift` | `pages.get`; the ```` ```print <id> ```` fence | no | no | M | |
| Download as files or PDF | `Support/PagePDF.swift`, `RichRenderer` | `pages.export` | no | no | S | The app makes the PDF |
| Publish to phone and web | `Pages.swift`, ProjectPage menu | `pages.publish/unpublish`; `convex/prints.ts`, web `/p/` | no | no | M | Backend and web are shared |
| Diagram Prints (Mermaid through Pele) | `Protocol/Page.swift`, `Resources/Rich/pele.js` | `pages.saveDiagram` | no | yes | M | |
| Only make Prints when I ask | `SettingsView.swift` | `settings.update {printsWhenAsked}` | no | yes | S | |
| Open in the side pane, follow versions | `PaneModel`, `Pages.swift` | `pages.get`, `pages.updated` | no | yes | S | |
| Prints in Highlights and on the rail | `HighlightsView`, `PanelsView`, `WindowDock` | `pages.list` | no | yes | S | |

### Looks

All of these are app-only: settings in UserDefaults, pictures in `<dataDir>/looks/`.

| Feature | Swift files | Runner API or keys | In GPUI | Spec | Size | Notes |
|---|---|---|---|---|---|---|
| Customize the Librarian page: picture, background, layout, GitHub and Threadbear activity, quote of the day, greeting by the clock | `Views/LibrarianLookEditor.swift`, `LookEditorSections.swift`, `LibrarianHome.swift`, `Looks.swift`, `GitHubActivityCard.swift`, `ThreadbearActivityCard.swift`, `Support/Quotes.swift` | none (GitHub read by the app) | no | no | L | |
| Customize the Scheduled page | `ScheduledLookEditor.swift`, `Support/ScheduledLook.swift` | none | no | no | M | |
| Customize the Projects page | `ProjectsLookEditor.swift`, `Support/ProjectsLook.swift` | none | no | no | M | |
| Main window background photo, dragged to show another part | `Looks.swift`, `SettingsView` | none | no | no | M | |

### Pages

| Feature | Swift files | Runner API or keys | In GPUI | Spec | Size | Notes |
|---|---|---|---|---|---|---|
| Public profile page (`/profile`, `/@handle`): addresses, Changelog of public commits, View profile page | `ProfilePageModel.swift`, `Views/ProfilePageSection.swift`, `Support/ProfilePage.swift` | `profilePage.get/update/remove`; `convex/profile.ts`, web `profile.tsx` | no | no | L | Needs the Librarian Customize panel first |
| Skills page and store: import, New Skill, Edit, Always, store switch, ⌘5 | `SkillsModel.swift`, `Views/SkillsPage.swift`, `SkillPicker.swift`, `Protocol/Skills.swift` | `skills.list/add/update/remove/save/read/enable`, `skills.updated` | no | no | L | GPUI only lists skills in the slash palette |
| Notes page: encrypted, heading sizes, task boxes, list toggle, ⌘6 | `Notes/NotesPage`, `NoteEditor`, `NotesStore`, Core `Notes/Notes.swift` | none (`<dataDir>/notes/*.tbnote`, `secrets/notes-key`) | no | no | M | Saving fixed in dbf82191 |

### Terminal

| Feature | Swift files | Runner API or keys | In GPUI | Spec | Size | Notes |
|---|---|---|---|---|---|---|
| Terminal on the rail | `Views/TerminalPanel.swift`, `PanelsView` | none | partly (⌘J panel) | partly | S | Off by default since aa01270d |
| Terminal tabs: +, rename, ⌘-click links, link underline | `TerminalPanel.swift` | none | no | no | M | GPUI has one shell per thread |

### Composer

| Feature | Swift files | Runner API or keys | In GPUI | Spec | Size | Notes |
|---|---|---|---|---|---|---|
| Shift-Tab cycles off, Plan first, Ask; plans saved as files | `TurnModes.swift`, `ComposerView`, `ComposerControls` | `threads.send {mode:"ask"}`, `threads.update {permissionMode:"plan"}`, `plan.proposed`, `plans.approve` | partly (Plan first only) | no | M | Ask mode is fork-only runner code |
| Plans open in Threadbear, with a link under the reply | Transcript cells, `PaneModel` | `plans/<date>-<project>.md` | ? | no | S | |
| /skill mentions coloured in the box, links once sent | Core `SkillMentions.swift`, `TextKit.swift`, `ComposerView` | `skills.list`, `threads.commands` | partly | yes | S | |
| Slash palette: search, ←/→ categories, palette or list, stays on screen, X closes, Return fills | `SlashPaletteView`, `SlashPalettePanel`, `SettingsView` | none | partly | yes | M | Return fills clashes with the spec "Return runs the picked command" |
| Return answers the oldest waiting card | `ComposerView` | `threads.send` answers | ? | no | S | |
| Attach Image takes Markdown files ↑ | `ComposerControls` | none | ? | no | S | |

### Files

| Feature | Swift files | Runner API or keys | In GPUI | Spec | Size | Notes |
|---|---|---|---|---|---|---|
| Files on the rail ↑ | `PanelsView`, `WindowDock` | `files.read` | yes | yes | – | |
| Clicking a file reopens it ↑ | `PanelsView` | none | ? | no | S | |
| Open files in Threadbear or in their own app | `FileLinks.swift` | none | ? | no | S | |
| Side pane files stay open, find and replace, search a folder | `PaneModel`, `TextFileView` | none | partly | no | M | |
| New Folder in every folder picker | `Projects.swift`, `ComposerControls`, `ScheduleEditor`, `SettingsView` | none | no | no | S | |

### Settings

| Feature | Swift files | Runner API or keys | In GPUI | Spec | Size | Notes |
|---|---|---|---|---|---|---|
| Settings over the main panel, with X | `Views/SettingsPage.swift`, `SettingsView` | none | no | no | M | GPUI uses a separate window |
| Settings window drags wider ↑ | `SettingsView` | none | ? | no | S | |
| ⌘, in the shortcut list, one Settings… item ↑ | `ThreadbearApp.swift`, `Shortcuts.swift` | none | ? | yes | S | |
| Profiles: Work beside Personal, colour strip | `AppProfile.swift`, `ProfilesSettings.swift`, `RPC/Profiles.swift` | `profiles.list/create/retry/rename/remove` | no | no | L | Needs `profiles.ts` |

### Connectors

| Feature | Swift files | Runner API or keys | In GPUI | Spec | Size | Notes |
|---|---|---|---|---|---|---|
| Set up, Add a service | `ConnectorsPanel.swift`, `ConnectorsModel.swift`, `SetupChecklist` | `connectors.setup`, `connectors.add`; experiment `connectorsSetup` | partly (status only) | partly | M | |
| "This Mac" gateway label, no-backend message | `ConnectorsPanel`, `ConnectorsText` | `connectors.status` | ? | no | S | |
| Connect an agent (Grok Bot, Notion, Slack) | `ConnectorsPanel`, `Threads/OutsideAgentApps.swift` | `agents.list/connect/revoke`; experiment `outsideAgents` | no | no | M | |
| × hides a row, + hidden brings it back | `ConnectorsPanel` | none | no | no | S | |
| Local connectors (Blender, uvx, npx, trust sheet) | `ConnectorsPanel` | `connectors.add {local}`; experiment `localConnectors` | no | no | M | |
| X search row | `ConnectorsPanel` (XSearchSection) | `xSearch.status/forgetKey`; experiment `xSearch` | no | no | S | |

### Voice

| Feature | Swift files | Runner API or keys | In GPUI | Spec | Size | Notes |
|---|---|---|---|---|---|---|
| Voice mode: talk out loud, spoken approvals, bear avatar | `Voice/VoiceMode`, `VoiceAudio`, `VoiceServer`, `VoiceButton`, `VoiceBear` | experiment `voiceMode` through `hello.experiments` | no | no | L | Mike took it out upstream (bbf4bdf4) |
| Fn-key dictation, Shift-Fn hands-free | `Voice/KeyDictation`, `VoiceKeyMonitor` | `voiceKey*` settings | no | no | M | |
| Talk into a note | `Voice/NoteDictation.swift` | `voiceMode` | no | no | S | |
| Bear chat in the sidebar | `Voice/BearChat.swift`, `VoiceBear` | `threads.create` in `~/ThreadbearThreads/Bear`; `bearShown` | no | no | M | Not behind an experiment |

### Transcript and other

| Feature | Swift files | Runner API or keys | In GPUI | Spec | Size | Notes |
|---|---|---|---|---|---|---|
| Your messages show basic Markdown | Core `UserMarkdown.swift` | none | ? | no | S | |
| Web addresses in inline code are links ↑ | Core `CodeWebAddress.swift` | none | ? | no | S | |
| Question card options wrap ↑ | Transcript cells | none | likely | no | S | |
| Panels stay inside a narrow window ↑ | `ActivityBar`, `PanelsView` | none | ? | no | S | |

## Runner methods only Wayne's features use

The GPUI port has to call these:

- `pages.list`, `get`, `edit`, `refresh`, `saveDoc`, `saveDiagram`, `restore`, `restoreVersion`, `setProjects`, `delete`, `export`, `publish`, `unpublish`, and the `pages.updated` event
- `skills.list`, `add`, `update`, `remove`, `save`, `read`, `enable`, and the `skills.updated` event
- `profilePage.get`, `update`, `remove`
- `profiles.list`, `create`, `retry`, `rename`, `remove`
- `connectors.setup`, and `connectors.add` with `{local}`
- `agents.list`, `connect`, `revoke`
- `xSearch.status`, `forgetKey`
- Parameters on existing methods: `threads.send {mode:"ask"}`, `settings.update {printsWhenAsked}`, and `hello.experiments`

## App-only versus runner work

- **App-only:** the three Customize pages and the background photo, Notes, Terminal tabs, New Folder, Settings over the main panel, hiding Connectors rows, the slash palette extras, Fn dictation, the bear chat, the Files and transcript items.
- **Needs the runner or backend too:** Prints, the profile page, Skills, Profiles, Ask mode, Connectors setup, local connectors, outside agents, X search, skill mentions, Voice (only its experiment flag).

## Prep before the port starts

1. Wait for Mike's GPUI update, then read `docs/gpui-port.md` and `docs/gpui-port/PARITY.md` again and refresh the "In GPUI" column here.
2. For each feature that needs the runner, decide: send the runner part to Mike as a PR, or keep it in this fork only. Start with Prints (PR #61).
3. Write a `features/*.feature` spec before building each one. Only `prints.feature` and a few slash palette scenarios exist.
4. Settle the Return clash in the slash palette with Mike's spec.
5. Decide whether Voice ports at all, since Mike took it out upstream.
6. Rust is installed on this Mac (`~/.cargo/bin`). GPUI builds take over an hour here while other work runs, so build on a quiet Mac or a check host.

## Order once it's unblocked

1. Terminal tabs, rename and links
2. Skills page and New Skill
3. Prints, part one: cards in replies, the side pane, Highlights and the rail
4. Notes page, plain editor first
5. Customize for the Librarian page, then Scheduled and Projects

Fit the small ones in around these: the slash palette, Shift-Tab, New Folder, the Connectors rows. Leave Voice, the bear chat and Profiles for last.
