# GPUI port backlog for Wayne's copy


This replaces the list of 12 Nov 2026 (its last version is in git, `git show 3ab72108:prds/gpui-port-backlog.md`), refreshed after Mike's 14 Nov release and with Ask mode and Profiles gone from this copy. Two things from that list still hold:

- Mike asked forks to wait before porting big things to the GPUI app, as big GPUI changes are still coming. Check after each bring-in what he's landed before starting a slice.
- Most of Wayne's runner side is fork-only (`skills.ts`, `pages.ts`, `public-profile.ts`, `x-search.ts`, `outside-agents.ts`, `connectors-catalog.ts`, `page-diagram.ts`, bears, tables and webhooks). A GPUI view over one of those works only in this fork until that runner code goes to Mike (Prints is draft PR mikecann/threadbear#61; `prds/bears-for-mike.md` is the draft for Bears).

## Summary

The GPUI app in `desktop/` is what Mike uses every day since 12 Nov, and the Swift app in `app/` is frozen. This copy just brought in Mike's release of 14 Nov (merge 9254b47c over 35906e10). Mike's side of the port is nearly done: the parity table has a handful of pending or partly rows, and `docs/gpui-port/whats-left.md` has a short list of open lines, a few of them already done in the log but not ticked yet. The bigger gap is Wayne's own work. About 165 of his commits since Mike's release live only in the Swift app and the runner, and the GPUI app has none of them: Prints, Skills, Notes, Voice, the bear, Customize and the public profile page, the Connectors additions, and a set of smaller changes to Files, the Terminal, the slash palette and Settings.

The runner side of Wayne's features already works for any client. Most of the GPUI work is views over runner methods that exist (`pages.*`, `skills.*`, `profilePage.*`, `agents.*`, `connectors.*`). The exceptions are Notes (the app keeps them itself, encrypted) and Voice (audio, speech and a local TTS server, all in the app today).

This plan is in two parts. Part 1 is what Mike's release ships that the GPUI app still lacks. Part 2 is Wayne's features, each with where it lives, what GPUI needs, a size, what it depends on, and whether it's behind an experiment.

Sizes follow `docs/gpui-port/features.md`: S under 200 lines of Rust, M 200 to 800, L 800 to 2500, XL more.

### Suggested first slices

1. **Mike's small leftovers.** Narrow-panel composer chips that shrink to fit, only the main panel's thread putting commands in the Terminal, the switch sizes on the Librarian and Usage pages, and ticking the whats-left lines the log already covers. S. Mike may land some of these upstream first, so check after each bring-in.
2. **A Print in a reply, opened in the side pane.** The Print card in the transcript, Open to the side pane, and the pane following new versions (`prints.pane`). Behind `pages`. M. It needs `pages.*` in the mock runner first.
3. **Prints lists and menus.** Highlights' Prints section, the rail's Prints list, the project page's Prints section with its menu, and the Settings switch Only make Prints when I ask (`prints.list`, `prints.whenasked`). M.
4. **Files, Terminal and slash palette changes.** Side pane files that stay open, find and replace, search inside a folder's files, the Files switches in Settings, Terminal tabs with rename, and the palette's search field and Return behaviour. M. This slice also checks that the Git panel already shows (it's a runner patch page).
5. **Skills page (⌘5)** and the small skill list at the cursor. M to L.

Then Notes, the Connectors additions, Customize and the profile page, the bear, and Voice last (it's the biggest and the most Mac-bound).

### How each slice is checked

`docs/gpui-port/slices.md` has the method: write the scenario in `features/` first, pass it on the Swift app, build it in `desktop/`, pass it on the GPUI app on Linux and on a real Mac, compare the pictures, and log it in `PARITY.md` with a `Verification:` line. The commands:

```bash
# core logic, safe on this Mac too with taskpolicy -b
desktop/scripts/remote.sh cloud -- bash -c 'cd desktop && cargo test -p threadbear-core'
# a build with the automation commands
desktop/scripts/remote.sh cloud -- bash -c 'cd desktop && cargo build -p threadbear-gpui --features automation 2>&1 | tail -5'
# a feature's scenarios, GPUI on Linux, then Swift and GPUI on a Mac's desktop
desktop/scripts/remote.sh cloud -- desktop/scripts/spec.sh '@prints.pane'
desktop/scripts/remote.sh tinker-desk --desk -- desktop/scripts/spec.sh --swift '@prints.pane'
desktop/scripts/remote.sh tinker-desk --desk -- desktop/scripts/spec.sh '@prints.pane'
# a picture of one thread for the side-by-side
desktop/scripts/remote.sh tinker-desk --desk -- desktop/scripts/spec.sh --shot "<thread title>" out.png
```

`cloud` and `tinker-desk` are Mike's machines, and this copy's origin is waynesutton/threadbear, so AGENTS.md says not to reach for them. Wayne needs his own: a Linux box for `cloud` and a Mac with a desktop for `--desk`, set up the way `remote.sh` expects (named in `~/.ssh/config`). Until then, the Desktop app workflow (`.github/workflows/desktop.yml`) runs the GPUI specs on GitHub's Mac, Windows and Linux runners on a push to his fork. Rust builds and spec runs never go on his working Mac.

## Part 1: Mike's features the GPUI app still lacks

Mike is closing these himself, and his releases come in here every few days. For each, check after a bring-in whether it's landed upstream before starting on it: doing it twice makes a conflict at the next merge.

### Checked on 14 Nov, after the bring-in

Mike's Mac leftovers are done in the code; only some `whats-left.md` lines aren't ticked yet (his doc, so they're left for him, to keep his next release from clashing here):

- Narrow-panel chips: done by Mike on 12 Nov (`c49323c8`). Checked in the scripted GPUI app at 560 points: `chipsFit` is true and Send stays in the box.
- Only the main panel's thread puts commands in the Terminal: done (`threadbear_core::terminal::can_put`, Mike's 10 Nov fix), with unit tests. Its scenario is still `@manual`.
- Switch sizes on the Librarian page and Usage: done by Mike on 13 Nov (`86086fe2`, `MacSwitch` with `outside_form` on Usage).
- Look Up, Translate, Writing Tools and Services on selected words: Mike's thread 0ff64630 is on it (`text_services.rs`). Don't start it here.
- What's still open on Mike's list is Windows and Linux (1.2), a VoiceOver pass by ear, and the row press GPUI can't do.

So the first slice of real work in this copy is Prints (2.1).

### 1.1 You'd notice these in a day

**Narrow-panel composer chips** (`composer.modelpicker`, `composer.permission`)
- In a thread in a panel of its own or a narrow window, the chips under the message box run past its edge. The Swift app shrinks their labels to fit.
- Swift: `Views/ComposerControls.swift`, `Views/ComposerView.swift`. GPUI: `desktop/gpui/src/composer.rs`, `composer_account.rs`.
- Size S. Spec: `features/composer-chips.feature` exists; add a narrow-width scenario.

**Only the main panel's thread fills the Terminal** (`terminal.putcommand`)
- The play icon on a shell block should only type into the Terminal of the thread in the main panel. The scenario is `@manual @gpui-pending` in `features/reading-a-thread.feature`.
- Swift: `ShellBlockIcons` in the transcript, `Views/TerminalPanel.swift`. GPUI: `desktop/gpui/src/shell_blocks.rs`, `terminal.rs`.
- Size S.

**Switch sizes on the Librarian page and Usage** (whats-left, Smaller things)
- Settings uses macOS's size now (`desktop/gpui/src/mac_switch.rs`). The Librarian page and Usage still have the kit's. Measure the Swift ones first.
- GPUI: `librarian.rs`, `usage.rs`. Size S. No spec needed: a picture beside the Swift one.

**The context ring** (`composer.contextring`, partly on all three)
- The breakdown landed on 11 Nov. Read that log entry for what's left before starting.
- GPUI: `composer_context.rs`, `desktop/core/src/context_breakdown.rs`. Size S.

**Text services on selected words** (whats-left, Smaller things)
- Spelling in the message box landed on 13 Nov (`composer.spelling`). Look Up, Translate, Writing Tools and Services on the right-click menu are still missing, as GPUI's input isn't an AppKit text view.
- GPUI: `composer_spelling.rs`, `text_services.rs`. Size M, Mac shim. Spec: `features/spelling.feature`.

**Lines in whats-left that the log already covers**
- `review.media` (pages on the review stage) and `file.htmlpreview` (.html in the pane): the 13 Nov log entry "Local pages in the pane and on the review stage" covers both.
- `panel.image`, `patches.icons`: covered by "Office previews, patch icons and Simulator" (13 Nov).
- `banner.refusal`: covered by the 12 Nov composer gaps entry.
- `system.handover`: covered by 13 Nov's handover entry, apart from minimised, full screen, Windows and Linux.
- Run each one's spec, then tick the line. Size: none.

**Rows with no passing spec run yet**
- `projects.decisions` passes on Swift and is pending on Mac: run `@projects.decisions` on a Mac's desktop.
- `dock.frame` (window frame memory) is built (`desktop/gpui/src/main.rs` restores `window_frame`) but has no scenario. Write one.
- `sidebar.hibernate.pick` (Pick a time) is built (`sidebar.rs`); its scenario is `@manual` in `features/sidebar-actions.feature`.
- `terminal.panel`: several scenarios are `@swift-pending` or `@windows-pending`.

**Not portable as it is**
- A row pressing in while the mouse is down: GPUI can't scale a row. Leave it out, or ask Mike for a stand-in.

### 1.2 Before other people move over

**Windows and Linux: window chrome, menu bar, notifications and badge** (`dock.titlebar`, `menu.*`)
- Traffic lights in the top bar, dragging the bar to move, double-click to zoom, and client-side decorations off the Mac. The pending columns in PARITY.md for Windows (Librarian, Scheduled, Settings, the Browser pane) mostly come from here and from the web runtime being Mac first.
- Swift: `WindowDock`, `ActivityBar`. GPUI: `shell.rs`, `window_menu.rs`, `notify.rs`, the `desktop/dock` crate.
- Size L. Spec: write `dock.titlebar` scenarios in `features/window.feature`.

**The web runtime on Windows and Linux** (`foundation.webruntime`)
- Mermaid, html blocks, web panels and the Browser pane draw through CEF on the Mac only. Wayne's Prints and Git panel lean on this too.
- GPUI: `web_runtime.rs`, `desktop/web`. Size L.

**Accessibility below the window** (`foundation.accessibility`)
- Every control has a role since 13 Nov. A full VoiceOver pass by ear is still to do.
- Size M. Spec: `features/accessibility.feature`.

**Torn-out panel windows** (`dock.detached`)
- In `desktop/dock` for the Mac, with fallbacks on Windows and X11 and none on Wayland (`desktop/dock/README.md`). Check it against the Swift app once Windows and Linux matter.

## Part 2: Wayne's features

Each GPUI feature that's behind an experiment reads `hello.experiments` and shows nothing while it's off, as the Swift app does. None of Wayne's features has a row in `features.md` yet, apart from the four Prints rows. Each needs its rows there first, then a `features/<area>.feature` with scenarios tagged by those ids, and `node features/verify-pairing.mjs` passing.

Features Wayne made that Mike took into his release are in the GPUI app already and need nothing: the Quit prompt, ⌘/ Collapse Threads, the ding when a thread finishes, the Appearance picker, standing instructions, the Experiments switches, Back and Forward beside Share, Markdown files drawn with a Preview / Markdown menu, editing files with line numbers and ⌘S, and New Thread on a project's page.

### 2.1 Prints

**What it is.** A thread makes a chart, dashboard, report, doc or diagram you keep, with versions, projects, a PDF download and Publish to the phone and website.
- Experiment: `pages`.
- Runner: `runner/src/pages.ts`, `page-doc.ts`, `page-diagram.ts`, the Print tools in `threadbear-mcp.ts`, the make-print skill in `runner/skills/make-print`. Backend: `convex/prints.ts`. All in place.
- Swift: `Pages.swift`, `PageSheets.swift`, `Transcript/PageCards.swift`, `ThreadbearCore/Protocol/Page.swift`, `ThreadbearCore/Support/PagePDF.swift`, the Prints lines in `Views/HighlightsView.swift`, `Views/PanelsView.swift`, `Views/ProjectPage.swift` and `Views/SettingsView.swift`. About 920 lines, plus its share of those views.
- Protocol: `pages.list`, `get`, `refresh`, `edit`, `export`, `delete`, `restore`, `restoreVersion`, `setProjects`, `publish`, `unpublish`, `saveDoc`, `saveDiagram`, and the `pages.updated` notification.
- Spec: `features/prints.feature` has six manual scenarios, all `@gpui-pending`. Its note says the demo runner doesn't seed Prints, and `app/Tools/mock-runner/server.mjs` has no `pages.*` yet. Adding that comes first, so the scenarios can run on both apps.
- Prereq: the mock runner's `pages.*` with a seeded chart and diagram Print.

Split into four slices:

**2.1a Print card and the side pane** (`prints.card`, new id; `prints.pane`)
- The card in a reply: a picture of the top, title, kind, version, age, Open, Refresh, the ⋯ menu. Open goes to the side pane, which swaps to the newest version on `pages.updated`.
- GPUI: the protocol type in `desktop/core/src/` (a new `prints.rs`), the row in `desktop/gpui/src/rows/`, the pane in `files.rs` / `pane_store.rs` through the local-page path that `file.htmlpreview` uses. Size M.
- Depends on the web runtime for the page itself; without it, show the picture and offer the browser, as other local pages do.

**2.1b Prints lists** (`prints.list`)
- Highlights' Prints section (kind, version, projects or "No project"), the Prints button on the right-hand rail under Files, and Prints on a project's page.
- GPUI: `highlights.rs`, `rails.rs`, `project_page.rs`. Size M.

**2.1c The Print menu** (`prints.menu`, new id)
- Refresh, Edit with Agent, Edit Text for a doc or diagram (`prints.diagram`), Download, Download as PDF, versions with Open and Restore, Projects, Publish and Unpublish, Delete with Recently deleted.
- GPUI: a menu in the shared menu helpers, a text editor reusing `file_edit.rs`, a save dialog. PDF export in Swift uses WebKit's print; GPUI needs CEF's `PrintToPDF` or the runner's `export-pdf.mjs` in the make-print skill. Ask the runner if it can do it: it works on every OS. Size M to L.

**2.1d Settings: Only make Prints when I ask** (`prints.whenasked`)
- A switch in Settings > General > Prints, shown while `pages` is on, writing `printsWhenAsked` through `settings.update`. The approval card is the one GPUI already draws.
- GPUI: `settings_window.rs`, `desktop/core/src/runner_settings.rs`. Size S.

### 2.2 Files, the Terminal and the slash palette

Changes Wayne made to features Mike already ported. Each needs new scenarios in the spec file that's there.

**Side pane files stay open, find and replace, search inside a folder** (`file.keepopen`, `file.replace`, `folder.search`, new ids)
- A file in the side pane stays until its back arrow, across panels and threads, at the same scroll. ⌘F in a file has Replace and All while editing. A folder on this Mac has a box that searches its files' lines, skipping `.git`, `node_modules`, build output, big and binary files, and a click opens the file at that line.
- Swift: `Views/FolderSearchView.swift`, `ThreadbearCore/Threads/FolderSearch.swift`, `PaneModel.swift`, `Views/TextFileView.swift` (commit 3708b1c7). About 320 lines new.
- GPUI: the search logic in `desktop/core/src/` (next to `find.rs` and `folders.rs`, with tests), views in `folders.rs`, `file_find.rs`, `pane_store.rs`. Size M.
- Spec: `features/file-editing.feature`, `features/folders.feature`.

**Files settings** (`settings.files`, new id)
- Settings > General > Files: Save files automatically, Edit files like an editor (both on by default), and Open files in (Threadbear or Its own app). With autosave off, leaving a changed file asks Save, Don't Save or Cancel.
- Swift: `Views/EditedFiles.swift`, the Files section in `Views/SettingsView.swift`. GPUI: `settings_window.rs`, `file_edit.rs`, `open_gate.rs`. Size S to M.

**Terminal tabs** (`terminal.tabs`, new id)
- + above the Terminal for another shell in the same folder, each a tab with ✕, double-click or right-click Rename…, links underlined under the pointer, ⌘-click opens a web link. Settings > General > Terminal adds a Terminal button on the rail beside Files.
- Swift: `Views/TerminalPanel.swift` (commits bd5ed3f6, d47ee37e, aa01270d). GPUI: `terminal.rs`, `terminal_element.rs`, `desktop/core/src/terminal.rs`, `rails.rs`. Size M.
- Spec: `features/terminal.feature`.

**The slash palette's search and Return** (`composer.slash`, extending it)
- A search field along the palette's top that finds by any word in a name or description, with X to close. Left and Right step through the categories. Return or Tab puts the command in the box and Return again sends it. A Run button sends at once. Settings > General > Slash commands has Return sends a command right away, and a choice of the small list at the cursor in place of the palette mid-message.
- Swift: `Views/SlashPaletteView.swift`, `Views/SlashPalettePanel.swift`, `Views/SkillPicker.swift`, `Views/ComposerView.swift` (commits aa01270d, 5e07e0c4, 7cd127cd). GPUI: `slash_palette.rs`, `desktop/core/src/slash.rs`. Size M.
- Spec: `features/slash-palette.feature`. Its "Return runs the picked command" scenario now means fill, then send. Change it in both apps' terms first.

**Your messages in basic Markdown** (`transcript.user`, extending it)
- Lists, bold, italics, code, links, headings and quotes in your bubbles, line breaks kept, plain text left as typed.
- Swift: `ThreadbearCore/Transcript/UserMarkdown.swift` (57 lines). GPUI: `desktop/core/src/` for the parse, the user row in `rows/`. Size S.
- Spec: `features/reading-a-thread.feature`.

**Return allows the oldest card** (`card.returnflow`, extending it)
- From an empty box or after a click in the thread, Return allows the card showing ⏎; with several waiting, the oldest, and Return again takes the next.
- Swift: `ThreadSession.swift`, `Views/ComposerView.swift` (commit 59336820). GPUI: `composer_card.rs`, `composer_send.rs`. Size S.
- Spec: `features/composer-cards.feature`.

**New Folder in folder pickers** (`foundation.filepicker`, extending it)
- Every folder picker (new thread, project folders, schedules, Settings) can make a folder.
- Swift: `Projects.swift`, `Views/ComposerControls.swift`, `Views/ScheduleEditor.swift`, `Views/SettingsView.swift`. GPUI: `folders.rs`, `new_thread.rs`. Check first whether GPUI's path prompt already lets the Mac's panel make folders. Size S.

**Git panel** (`gitPanel` experiment)
- VS Code style Source Control beside a thread in a git repo, with two chips above the box.
- Runner: `runner/src/git-panel.ts` and `git-panel/page.html`. It's a page the runner serves, handed to clients as a patch panel (`patches.ts`), so GPUI should show it through its web panels already.
- GPUI: likely nothing. Turn it on with a dev runner, open a thread in a repo, check the panel and its chips. Size S, check only. Spec: a scenario in `features/patches.feature` or `web-panels.feature`.

### 2.3 Skills (⌘5)

**What it is.** A page of agent skills for every thread, login and CLI: import from a GitHub link, Suggested with Add, Update to the Latest, remove, and New Skill to write one. A `/skill` list as you type, in Claude and Codex, and every listed skill counted as a mention (shown as links in the words).
- Not an experiment. Settings > General > Skills shows or hides its bar button.
- Runner: `runner/src/skills.ts`, `skill-mentions.ts`. In place.
- Swift: `SkillsModel.swift`, `Views/SkillsPage.swift`, `Views/SkillPicker.swift`, `ThreadbearCore/Protocol/Skills.swift`, `ThreadbearCore/Voice/SkillMentions.swift`, skill links in `Transcript/TextKit.swift`. About 840 lines.
- Protocol: `skills.list`, `add`, `remove`, `update`, `save`, `read`, `enable`, and `skills.updated`. The mock runner has all of them.
- GPUI: types and mention matching in `desktop/core/src/` (a new `skills.rs`, beside `slash.rs`), the page as a main-panel page like the Librarian's (`home_page.rs`, `activity_panels.rs`), the activity bar button in `rails.rs`, ⌘5 in `desktop/core/src/shortcuts.rs`, the cursor list in `composer_inline.rs`. Back puts the page away first (`navigation.rs`).
- Size L. Spec: new `features/skills.feature`.
- Depends on the slash palette work in 2.2 for the Skills category and the cursor list.

### 2.4 Notes (⌘6)

**What it is.** Your own notes, on this Mac only, each file sealed with AES-GCM under a key in the data folder's `secrets/notes-key`. A code-editor style box with line numbers, Markdown coloured as you type, bigger headings by level, task boxes you can tick (⌘Z undoes), Return carrying lists on, a list you can fold away, and Esc or any other view to leave.
- Not an experiment. Settings > General > Notes shows or hides its bar button. Talking into a note is under Voice (2.8).
- Runner: none. The app does it all.
- Swift: `Notes/NotesStore.swift`, `Notes/NotesPage.swift`, `Notes/NoteEditor.swift`, `ThreadbearCore/Notes/Notes.swift`, `Views/LineNumberRuler.swift`. About 850 lines.
- GPUI: the store in `desktop/core/src/` (a new `notes.rs`), reading and writing the Swift app's exact file format and key so the two apps share notes while both are in use, with a test that opens a note the Swift app sealed. The page in `desktop/gpui/src/` (a new `notes_page.rs`), its editor built on `file_edit.rs`'s. ⌘6 in shortcuts.
- Needs an AES-GCM crate: a new dependency, so pin it exactly and say why in the commit (slices.md). Keep the files 0700 and 0600 as Swift does, and add nothing to what's sent anywhere.
- Size L. Spec: new `features/notes.feature`. Scenarios can't use Wayne's real notes; give the mock run a temp data dir.
- Check `docs/on-your-mac.md` still says where notes and their key are once GPUI writes them too.

### 2.5 Connectors additions

Mike's GPUI Connectors panel (`desktop/gpui/src/connectors_panel.rs`, 267 lines) shows services and their sign-in. Wayne's Swift one is about 960 lines longer.

**Connectors setup** (`connectorsSetup` experiment)
- Set up starts a thread that installs plugbear on this Mac and wires the logins; Add a service lists Notion, Slack, Linear, Sentry, Cloudflare, DeepWiki and Other MCP server; × hides a row, + hidden brings it back.
- Swift: `Views/ConnectorsPanel.swift`, `ConnectorsModel.swift`, `ThreadbearCore/Threads/ConnectorsText.swift`, `Views/PanelForm.swift`. Runner: `connectors.ts`, `connectors-catalog.ts`.
- Protocol: `connectors.setup`, `connectors.add`, `connectors.status`, `connectors.url`.
- GPUI: `connectors_panel.rs`, `desktop/core/src/connectors.rs`. Size M.

**Local connectors** (`localConnectors` experiment, needs Connectors setup)
- Blender and Other local server (uvx or npx at one version), with a sheet showing the exact command and which tools ask first, before anything's added.
- Same files. Size S to M.

**Outside agents** (`outsideAgents` experiment)
- Agents that use Threadbear: Connect an agent picks Grok Bot, Notion or Slack, makes a key, shows the backend's `/mcp` address, lists keys with Revoke.
- Swift: `ThreadbearCore/Threads/OutsideAgentApps.swift`, the agents part of `ConnectorsPanel.swift`. Runner: `outside-agents.ts`; backend `convex/mcp.ts`, `mcpRpc.ts`.
- Protocol: `agents.list`, `agents.connect`, `agents.revoke`. The mock runner has them.
- GPUI: `connectors_panel.rs`. Size M. Treat a new key like a secret: shown once, copied, never logged.

**X search** (`xSearch` experiment)
- The runner's `x_search` tool, and an X search row under Built into Threadbear in the Connectors panel saying whether a key is saved.
- Runner: `x-search.ts`, in place. GPUI: one row in `connectors_panel.rs`. Size S.

Spec for all four: `features/connectors.feature` has two scenarios; add setup, add a service, a local server's sheet, hiding rows, Connect an agent and Revoke.

### 2.6 Customize, backgrounds and the public profile page

**Customize the Librarian page** (`librarian.customize`, new id)
- A panel beside the page: keep the bear or use your own picture, a background photo with Fade and Position, which parts show and where (greeting by the clock, quote of the day with Copy Quote, Threadbear activity, GitHub activity), layout switches.
- Not an experiment: Settings > General > Look turns Customize on, off by default.
- Swift: `Looks.swift`, `Views/LibrarianLookEditor.swift`, `Views/LookEditorSections.swift`, `Views/CustomizedPage.swift`, `Views/GitHubActivityCard.swift`, `Views/ThreadbearActivityCard.swift`, `ThreadbearCore/Support/PageLook.swift`, `Quotes.swift`, `ThreadActivity.swift`, changes in `Views/LibrarianHome.swift`. About 2,100 lines.
- GPUI: the look model, quotes and activity counts in `desktop/core/src/` (a new `looks.rs`, beside `librarian.rs`), the panel and cards in `librarian.rs` and a new `look_editor.rs`. The looks are saved in the data folder's `looks/`; read the Swift app's files so a look carries over. GitHub activity calls GitHub's API, which `docs/on-your-mac.md` must already list.
- Size L. Spec: `features/librarian.feature`, new scenarios.

**Customize the Scheduled and Projects pages** (`schedule.customize`, `projects.customize`, new ids)
- The same panel on those pages, with their own backgrounds and layouts.
- Swift: `Views/ScheduledLookEditor.swift`, `Views/ProjectsLookEditor.swift`, `ThreadbearCore/Support/ScheduledLook.swift`, `ProjectsLook.swift`. GPUI: `scheduled.rs`, `projects_page.rs`. Size M once the Librarian one is in.
- Spec: `features/scheduled.feature`, `features/projects-views.feature`.

**Background picture behind a new thread** (`foundation.background`, new id)
- Settings > General > Look: a photo behind the new-thread page, bottom edge on the window's, drag to move, Fade, and optionally behind every transcript.
- Swift: `Looks.swift`, `Views/DetailView.swift`. GPUI: `new_thread.rs`, `thread.rs`. Size M. Depends on the look model above.

**Public profile page** (`librarian.profilepage`, new id)
- The bottom of the Librarian's Customize panel edits your page at `<site>/profile`: name, a line, up to four links, which parts show, the numbers, projects, Publish and front page switches, its addresses with Copy and Open, and a View profile page button on the Librarian page.
- Runner: `public-profile.ts`, `site-links.ts`; backend `convex/profile.ts`; web `web/src/profile.tsx`. All in place.
- Swift: `ProfilePageModel.swift`, `Views/ProfilePageSection.swift`, `ThreadbearCore/Support/ProfilePage.swift`. About 680 lines.
- Protocol: `profilePage.get`, `update`, `remove`. The mock runner has them.
- GPUI: a section in the look editor, saving a moment after each change. Size M. Depends on Customize the Librarian page.

### 2.7 The bear

**What it is.** A bear at the top right of the sidebar that opens a chat for anything. Its chats are ordinary threads in `~/ThreadbearThreads/Bear`, with `CLAUDE.md` there as its instructions and a memory it keeps. Right-click has Talk Out Loud and Show the Bear's Folder. It's always shown; there's no setting to hide it.
- Not an experiment. Talk Out Loud needs Voice (2.8).
- Swift: `Voice/BearChat.swift`, bear parts of `Voice/VoiceBear.swift`, `Views/SidebarView.swift`, `AppModel+ThreadActions.swift`.
- GPUI: a button in `sidebar.rs`, starting or opening its thread through `threads.create` with that folder; the folder setup ported from `BearChat.swift`. Mike passed over drawn bears on 13 Nov, so use the SVG the About window draws. Size S to M.
- Spec: new scenarios in `features/sidebar-actions.feature`.

### 2.8 Voice mode

**What it is.** A microphone button beside Send (and on a new thread's composer and the bear's) to talk with a thread out loud: speech to text on the Mac, the reply read out sentence by sentence by Kyutai's pocket-tts run locally, talking over it to stop it, approvals answered by voice. Also Fn keys to dictate into any box (Shift Fn for hands-free), talking into a note, and Settings > Voice (button icon, voice, keys).
- Experiment: `voiceMode`. Off means no button, no download, no microphone prompt.
- Swift: `Voice/VoiceMode.swift`, `VoiceAudio.swift`, `VoiceServer.swift`, `VoiceButton.swift`, `VoiceIcon.swift`, `VoiceBear.swift`, `KeyDictation.swift`, `VoiceKeyMonitor.swift`, `NoteDictation.swift`, `Views/VoiceSettings.swift`, `Notes/NoteDictationViews.swift`, and `ThreadbearCore/Voice/*` (spoken text, spoken approvals, utterance tracking, voice catalogue, keys). About 4,300 lines.
- GPUI, split three ways:
  - The pure parts (what to read out, answering approvals by voice, utterance tracking, the key map) go to `desktop/core/src/voice/` with their tests ported from `VoiceTests` and `VoiceKeysTests`. Size M.
  - The pocket-tts download, checksum, `serve` on 127.0.0.1 and its pid file are plain process and file work, portable as they are. `desktop/gpui/src/` (a new `voice_server.rs`). Size M.
  - The microphone with echo cancellation, Apple's SpeechAnalyzer, and the Fn key monitor are Mac-only and Swift-first APIs. SpeechAnalyzer has no Objective-C face, so `objc2` can't reach it. Two choices: a small Swift helper the GPUI app starts and talks to over a pipe, or a different recogniser that runs on every OS. That choice is Wayne's to make before any code. Size L, `shim` or `hard`.
- Depends on Notes for note dictation and on the bear for Talk Out Loud.
- Spec: new `features/voice.feature`. Most of it can only run by hand on a real Mac (the microphone), so tag those `@manual` and script what the core decides.
- Keep `docs/on-your-mac.md` true for the GPUI app: the voice folder, Hugging Face cache, the microphone prompt.

### 2.9 Settings and the activity bar

These come with the features above rather than as slices of their own. List them so none gets missed:
- Settings over the main panel, with an X, Esc, and Open Settings in (over the main panel or its own window). Swift: `Views/SettingsPage.swift`. GPUI's Settings is a window today (`settings_window.rs`). Size M. Not needed for any feature, so it can come last.
- Settings > General sections GPUI lacks: Look, The bear, Skills, Notes, Slash commands, Terminal, Files, Prints. Settings > Voice as its own tab.
- Activity bar buttons for Skills and Notes under the Librarian, with ⌘5 and ⌘6, and Back putting Settings, Skills or Notes away first (`rails.rs`, `navigation.rs`, `desktop/core/src/shortcuts.rs`, `features/settings-keyboard.feature`).

### 2.10 Bears, tables and webhooks

- Named teammate threads (bears), group chats (tables) and `/hook/` webhooks, behind the `bears`, `tables`, `webhooks` and `bearNotes` experiments. The runner side shipped on 13 Nov; `prds/bears-gpui.md` has the full GPUI list.
- Port the three pieces that exist in the Swift app from its code and `features/bears.feature` (bear tiles and Bear Colour in the sidebar, the @ picker in the composer, a table's bears on its project page). Design the rest (menus, New bear and New table sheets, the table chat, webhooks on the Scheduled page, bear notes) from `prds/bears-gpui.md`.
- Size L. Every runner call it needs exists. Order: after Prints, as it builds on the sidebar and composer pieces those slices touch.

## Order of the whole backlog

| # | Item | Owner | Size | Flag | Spec |
|---|---|---|---|---|---|
| 1 | Narrow chips, Terminal main panel, switch sizes, stale ticks | Mike | S | none | extend existing |
| 2 | Print card and side pane | Wayne | M | `pages` | prints.feature, mock first |
| 3 | Prints lists, Settings switch | Wayne | M | `pages` | prints.feature |
| 4 | Files, Terminal tabs, slash palette, Git panel check | Wayne | M | `gitPanel` for the panel | extend existing |
| 5 | Skills page and cursor list | Wayne | L | none | write skills.feature |
| 6 | Print menu, Edit Text, PDF | Wayne | M to L | `pages` | prints.feature |
| 7 | Your messages in Markdown, Return allows the oldest card, New Folder | Wayne | S | none | extend existing |
| 8 | Notes | Wayne | L | none | write notes.feature |
| 9 | Connectors setup, local, outside agents, X search row | Wayne | M to L | four flags | extend connectors.feature |
| 10 | Text services, context ring | Mike | M | none | spelling.feature |
| 11 | Customize Librarian, then Scheduled and Projects, backgrounds | Wayne | L | none | extend librarian, scheduled, projects-views |
| 12 | Public profile page | Wayne | M | none | extend librarian.feature |
| 13 | The bear | Wayne | S to M | none | extend sidebar-actions.feature |
| 14 | Voice mode | Wayne | XL | `voiceMode` | write voice.feature |
| 15 | Settings over the main panel | Wayne | M | none | extend settings.feature |
| 16 | Windows and Linux chrome, web runtime there, VoiceOver pass | Mike | L | none | window.feature, accessibility.feature |

Mike's Windows and Linux work sits at the end here only because Wayne works on a Mac. It stays Mike's, and it lands through bring-ins.

## Rules each slice keeps

- A `features.md` row and a scenario before the code. Pass it on the Swift app first so the scenario is right; if the Swift `dump` lacks a field, add it to `AppDump.swift` (debug only, which the freeze script allows for test tooling; check `scripts/swift-freeze.sh`'s list).
- Every control has a tooltip through the shared helper, and every click shows at once or shows a spinner and stops taking clicks.
- Plain words in the UI, no em dashes, no kicker labels.
- Comments name the Swift type a piece of Rust ports.
- Automation commands only under the `automation` feature.
- A change to what someone sees updates `docs/guide.md`; anything new the app reads, writes or reaches updates `docs/on-your-mac.md`; a new feature or move updates `docs/forking.md`.
- Log each slice in `PARITY.md` with its `Verification:` line. A slice with no runtime check doesn't count.

## Open questions

- Which machines take `cloud` and `tinker-desk`'s place for Wayne, or whether GitHub Actions on his fork is enough for now.
- Voice's speech recogniser in GPUI: a Swift helper process, or a cross-platform one.
- Notes: keep the Swift app's exact sealed format (so both apps share notes during the switch), or move them once and leave Swift behind.
- PDF export for Prints: CEF in the app, or the runner's make-print script, which would work the same on every OS.
- Which of these Wayne uses most. If his activity log is on, `tb activity --summary` would say, and could reorder slices 4 to 13.
