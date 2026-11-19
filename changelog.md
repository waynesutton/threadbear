# Changelog

## Prints

How Prints work today, what's missing and what's next: [Prints status](https://github.com/waynesutton/threadbear/blob/main/prds/prints-status.md).

## [Unreleased]

### Fixed

- Automatic Still open cards can omit PR reminders for selected GitHub owners while retaining local-work warnings.

- Task list sections leave a gap before the first row so its hover area stays clear of the toggle.

- Composer pictures wrap within the Mac message box and scroll after three rows, keeping later attachments reachable.
- Tasks boards scroll horizontally between sidebars; list completion controls have more room, and board task entry fields are taller. Print web links open in the browser, and delete confirmation accepts Return or Escape.
- Embedded Print and Git pages follow the app appearance, including changes while the panel is open without reloading.

### Removed

- Late answers experiment. It was never turned on and no tool card ever expired, so an expired card is simply closed again.
- GPUI Notes microphone and Use with Notes setting. Thread Voice, keyboard dictation and Pause before sending remain available.

### Changed

- Prints, Bears and Tables sidebars each have search, visible selection controls and bulk Archive/Delete actions. Prints archived on this Mac can be restored without changing their files or shared links.

- Tasks appearance controls use an icon, photo sliders and an explicit List/Board default. Wrap Columns keeps every board column on the page.
- Notes offers Sans and Mono text with size controls and line numbers. Existing Serif choices fall back to Sans.
- Generated Prints uses updated light/dark dashboard and report layouts, metric changes and compact charts. Published Prints offers System, Light and Dark themes.
- Scheduled follow-ups in the GPUI transcript start collapsed, with Show details and Hide details. Expanded instructions use selectable, left-aligned Markdown; the agent still receives the original message.
- Tasks adds keyboard navigation, deletion confirmation, renamed and custom columns, and Send to thread with a link back to the created thread. Boards scroll within narrow windows and phones. Appearance uses Light/Dark with matching text; native photos are read by the picker’s app before upload and downloaded for display. The project-page Tasks tab matches Project typography. Requires backend API 86.

- Desktop CI has 60 minutes on Linux and Mac, and 90 on Windows, to finish tests, the release build and artifact upload.
- Tasks opens on a quick-add list: type a task, press Return, tick it off. Done tasks leave the list with Undo. The board gains a Doing column, title-only cards and drag to reorder columns, list sections and project groups. Each task has a ⋯ menu to add a note, move it to a project or change its status. The web photo picker is a real button now. Focusing a task field returns keyboard control from web panels to the app.
- GPUI Settings opens inside the main window. Back, Esc or Close returns to the previous view with its thread draft intact.

### Added

- Tasks with a due date show in the Mac app's Scheduled view under Tasks due, soonest first, with overdue in red and today in amber. Click one to open it in Tasks while the Scheduled list stays beside it. The Scheduled page lists the five soonest under Upcoming tasks and adds Tasks due, Tasks due today and Tasks overdue tiles.

- Search Prints by title when attaching one to a task. Bears & Tables menus add Open, Edit, Show in Finder, Copy Folder Path and Refresh.

- Task detail pages with priorities, due dates, sub-tasks, links, image attachments and activity/comments, backed by API 87. Project-related threads and Prints appear in the detail page; Highlights can add a task.
- Native Bears & Tables management with rename, right-click actions and multi-selection deletion. Prints supports multi-selection for project assignment and confirmed deletion.
- GPUI desktop: ⌘Z brings back archived threads, newest first, up to the last seven, outside a text box or in an empty message box. The Archived note has an x and goes away with a sideways trackpad swipe.
- Clearer thread completion experiment: task progress across turns on the composer card, separate runner loose-end checks, and neutral action buttons with an amber left edge. Archive remains manual. Mirror support requires backend API 85.
- GPUI Voice waits 1.5 seconds before sending, so a short pause mid-sentence doesn't cut you off; Settings > Voice & Audio > Pause before sending changes it.

- Tasks experiment: account-synced tasks with a quick-add list, four-column board, explicit default view, projects and backgrounds on desktop (⌘7) and the authenticated web page. Deployed and verified in the installed desktop app.

- Customize in the GPUI app: Settings > General > Look turns on Customize for the Librarian and Scheduled pages and a background photo behind the main window, as in the Swift app. The Librarian panel has the greeting, your picture or none, a background photo you drag into place and fade, the quote of the day, the layout, the title parts, the numbers, and the GitHub and Threadbear activity calendars. What you set in the Swift app comes across the first time, with its pictures.
- Notes in the GPUI app: the bar's Notes button and ⌘6 open your notes over the main window, the same encrypted notes the Swift app keeps. New Note, rename, delete, coloured Markdown, task boxes you tick with a click, and lists that carry on when you press Return. Settings > General > Show Notes hides them and keeps them.
- Voice in the GPUI app, behind the voiceMode experiment: a microphone button beside Send to talk with a thread out loud and hear its replies, spoken yes or no for plain command cards, hold Fn (or Shift Fn, Fn Space, Fn Control) to dictate into any text box, and Settings > Voice & Audio for the icon, keys, voice, speed, language and microphone. No bear avatar.
- The GPUI Connectors panel can set up a gateway, add services, connect outside agents with masked key fields, hide and restore rows, and review pinned local-server commands before adding them.
- Bears share one unattended worker across routines, webhooks and mentions, with a separate reply for each mention. Memory rules track prior reports and unfinished tasks; `tb bears refresh-rules` updates existing bears without replacing their personas.
- Built-in Claude memory tidies use Auto after direct creation or an authorized refresh; Codex and agent-created bears keep Ask. Empty runs can request a quiet finish without hiding useful reports or unfinished work.
- Late answers, an experiment: answering a tool card within a day of expiry resumes the work once, without replaying the call or widening access. The deliberate late-answer button remains planned for GPUI.
- A background picture can be an animated GIF, in the main window (Settings > General > Look) and behind the Librarian, Scheduled and Projects pages. It plays, and holds still with Reduce Motion on. Swift Mac app only.
- A project with tables among its Parts lists each table's bears on its own Project tab.
- A table's project page lists the bears at it, a bear's thread put in a table seats that bear, and new tables get a three-people icon.
- `tb bears access --auto` on a bear that runs on Codex says Auto works like Ask there.
- Typing @ in the message box lists your bears as you type; picking one writes its name, so the message goes to that bear too.
- A bear's threads show its icon on its colour in the sidebar, and Bear Colour on a bear thread's right-click menu picks the colour.
- Bears, an experiment: named teammates like Researcher or Writer, each a folder with its own persona, memory, face and routines. Head Bear is made first, as chief of staff for the rest. Manage them with `tb bears`.
- `@Name` in a message, or `tb bears ask`, hands the message to a bear, and its reply comes back to the thread that asked. A bear never gets more access than the thread that asked it.
- Tables, an experiment: a group of bears around one goal, talking in one thread you can read, with a cap of 6 bear replies per message.
- Webhooks, an experiment: a secret URL on your own backend that starts a thread in a folder or bear (Ask or Auto, never Full access) or messages a thread. Bursts become one run, and repeats are dropped.
- Bear notes, an experiment: a bear you opt in can leave one short cheer or idea in Highlights when it's earned.

### Fixed

- Failed Instructions saves remain visible after leaving GPUI Settings, keep the edit for retry while the app is open, and hold automatic restarts until saved.

- A temporary outside-agent list failure in the GPUI app keeps the last list and logs the error without repeatedly showing a banner.
- Shared bear workers now enforce a mention sender's sandbox boundary and preserve routines' explicit login, pool, model and effort choices. Late answers retry a partially saved turn without losing the answer or duplicating its message.
- Network probe checks no longer fail runner updates just because a busy Mac delays test execution.
- Runner updates run at most four unit-test files at once, reducing contention on busy machines.

### Added

- Show in Finder on every Print menu, and the whole Print menu, Delete included, on a right-click in Highlights and the rail's Prints list.
- Todo list and kanban board Prints: click an item to tick it, or a card's arrow to move it to the next column. Each click saves a new version, and Edit Text edits the Markdown underneath.
- Only make Prints when I ask, in Settings > General > Prints. Prints stay on, but a thread asks before making one your message didn't ask for.
- Open on a reply's Print card opens it in the thread's side pane, which moves to each new version as it's saved.
- Print cards in replies on the website: published ones open their page, the rest say they're kept on the Mac.
- The guide covers keeping a Print fresh on a schedule.
- A Prints section in a thread's Highlights, and a Prints button on the right-hand rail under Files that lists every Print with the projects it's in.
- A Convex insights report: ask for one and make-print turns `npx convex insights` into a dashboard of write conflicts and read-limit warnings from the last 72 hours, or an all-clear report when there are none.
- A Convex API map: "map my Convex API" turns `npx convex function-spec` into a dashboard of every function and HTTP route, public or internal, with missing validators first.

### Fixed

- Wide diagram Prints stay readable: past the page's width a diagram uses the whole window and scrolls sideways, never drawn below 70% of its size.
- The built-in make-print's commands ran but did nothing once installed under Application Support. They work from any folder now.

### Changed

- Prints have a new look in dark and light: neutral near-black and white, larger titles, calmer type, dotted links and quieter borders, using the system font so nothing extra loads. The bundle report lost its gradient.
- make-print and bundle-report ship with Threadbear as built-in skills, so every install can make Prints. The saved Print style moved to `~/.config/threadbear/print-style.json`, so a skill update keeps it.
- In Ask before edits, a thread asks once per Print for updates instead of every time (Claude threads).

## 2026-11-11

Landed on main at `b221756c` and pushed to GitHub. Prints are still the `pages` experiment, off by default.

### Added

- Create diagram Prints from Mermaid source, edit their text, keep versions and download the source and SVG.
- Let outside-agent keys with read-all access list and read published Prints in bounded pages, without access to local unpublished Prints or any Print write operations.
- Behavior specs for diagram editing, invalid edits and project assignment in `features/prints.feature`, run by hand until the GPUI port has Prints.

### Changed

- Print cards use a solid native surface without a preview fade; diagram previews follow Threadbear's light and dark palettes.
- Prints were rebased onto Mike's 2026-11-11 release, keeping his new menu icons.

### Fixed

- Publishing, the Return key on the oldest waiting card and the Print wording in tool replies came back on main (`e8dd6683`) after they were lost when main was replaced with GitHub's merge of Mike's release.

## 2026-11-10

### Added

- Prints (called Pages until this day): stored, named, versioned pages a thread makes, in a project or none. Kinds: chart, dashboard, report, doc and bundle.
- A Print card in replies that follows the newest version.
- Prints on the project page, with Refresh, Edit with Agent, Edit Text for docs, Download, Versions, Projects and Delete with a 7-day restore.
- Download as PDF for the newest and each kept version.
- Publish a Print to your own Convex backend (API 77), opening it on the phone, the website's `/p/<slug>` page and for anyone with the link. Unpublish takes it down.
- The make-print and bundle-report skills.

### Changed

- Pages renamed to Prints everywhere people and agents see it. The old tool names and the page fence still work.
