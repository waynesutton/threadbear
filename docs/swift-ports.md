# Wayne's Swift app changes, carried over to the GPUI app

This copy changed the old Swift Mac app (`app/`, `dock/`) in 125 places
that upstream doesn't have. The new app's Updates panel shows the newest 100
under "Not in the new app yet". This list says what's happening to each one.
Wayne's call (14 Nov 2026): rebuild everything in the GPUI app (`desktop/`)
except Voice and the fuller Connectors panel. Those two keep what the GPUI
app already has from upstream, and stay in the Updates panel.

A port is a commit with a `Ports: <Swift commit>` trailer and a spec in
`features/`. An item that's already in the GPUI app, or has nothing to
rebuild, gets the trailer on a bookkeeping commit so it leaves the panel.

Status: **to do**, **doing**, **done** (landed), **already** (the GPUI
app does it), **nothing** (nothing to rebuild), **skip** (Wayne said no).

## Small changes

| # | Commit | What it does | Status |
|---|---|---|---|
| 20 | 59336820 | Return allows the oldest waiting card | done |
| 22 | 5e07e0c4 | Return fills a slash command in the box; a setting makes Return run it | done |
| 23 | 7cd127cd | A close button on the slash palette | done |
| 25 | aa01270d | Palette search field, Left/Right through categories, palette or list; optional Terminal rail button | done |
| 12 | ae1dd28a | Store skills listed by plain name | done |
| 14 | d3c96861 | Every listed skill turns blue in the box and links to its SKILL.md | done |
| 92 | 0a003c67 | Files open ready to type, autosave switch, unsaved dot, Save prompt | done |
| 93 | 41275129 | Line numbers and ⌘S, opened ready to type | done |
| 98 | d28e38f4 | A saved plan opens in the side pane, with a link under the reply | done |
| 43 | 3708b1c7 | Side pane find and replace, and search in folders | done |
| 79 | 55498864 | Settings > Keyboard list full height | to do |

The approval and slash palette batch passed 33 UI scenarios and 24 core checks.
Item 25 also has its optional Terminal rail button, opening the same drawer as
the keyboard shortcut.

## Terminal

| # | Commit | What it does | Status |
|---|---|---|---|
| 62 | bd5ed3f6 | Several terminals per thread as tabs with +, ⌘-click opens links | done |
| 49 | d47ee37e | Links underline under the pointer; tabs rename | done |
| 48 | e9408139 | The terminal tab menu shows icons | done |

## Bears

| # | Commit | What it does | Status |
|---|---|---|---|
| 9 | f69b7232 | Sidebar rows show a bear's icon on its colour, Bear Colour menu | done |
| 1 | d3e7a3f3 | The Bear Colour menu shows its icons | done |
| 8 | debffacf | Typing @ in the box lists your bears | done |
| 7 | 57934753 | The @ picker keeps to this Mac's runner | done |
| 5 | 38c291b6 | A table's bears shown, seated on join | done |
| 3 | eed7db3c | A project's page lists its table's bears | done |

## Prints

| # | Commit | What it does | Status |
|---|---|---|---|
| 34 | dbbf3fc2 | A Prints section on a project's page | done |
| 33 | 8e2d98be | Print menu, doc editor, versions | done |
| 29 | f5c9a15b | Pages are called Prints | done |
| 32 | 19aa34f1 | Print cards in replies, preview on top | done (the preview picture needs the web runtime; untried here) |
| 31 | efbfb928 | Download a Print as a PDF | done (needs the web runtime; untried here) |
| 21 | f0d66f56 | Publish a Print to phone and web | done (untried: needs sync and the alert) |
| 28 | 321c447c | Prints menus show icons | done |
| 19 | ed22d3ba | Diagram Prints | done |
| 16 | 76d2baba | Prints open in the side pane and follow versions | done |
| 15 | ae966880 | Prints in Highlights and on the rail | done |
| 17 | cf844799 | Make Prints only when asked (Settings switch) | done |
| 11 | 611d2564 | Todo list and kanban board Prints | done (a real click in the page needs the web runtime; untried here) |
| 10 | 5d406b1f | Show in Finder and Delete on every Print menu | done |

## Customizing pages

| # | Commit | What it does | Status |
|---|---|---|---|
| 75 | 52170ffa | Customize the Librarian page: picture, background, layout, GitHub activity | done |
| 63 | 29a2aa09 | A greeting by the clock; title parts hide on their own | done |
| 60 | 03ff9f81 | A quote of the day; Layout's switches line up | done |
| 59 | 59b51fa1 | Picture choice is plain buttons | done |
| 57 | 964f0ddd | Copy Quote menu shows its icon | done |
| 56 | a2d6ecca | Customize panel is plain cards | done |
| 54 | 41964018 | Background photos on the bottom edge, drag to move | done |
| 52 | 79084074 | Background photo doesn't block clicks; GitHub trouble message | done |
| 51 | 970b8308 | Threadbear activity: a year of threads | done |
| 50 | 2b0a8dab | Photo picker as a sheet; drags from Photos | done |
| 47 | 5b2e6d8e | Your own picture sticks | done |
| 6 | 13069201 | Animated GIF backgrounds play | done |
| 39 | 6f5cc154 | Customize the Scheduled page | done |
| 38 | 78be088e | Scheduled page parts move to their place | done |
| 37 | 84fad5ff | Customize the Projects page | done |

## Profile page

| # | Commit | What it does | Status |
|---|---|---|---|
| 67 | cc34b08a | A public page about you at /@handle | to do |
| 58 | 447a0aa9 | One page per site at /profile, saved as you change it | to do |
| 61 | c1d618b4 | Reopening Customize loads the saved page | to do |
| 46 | a4af95c0 | Open goes to the profile, Open Site to the front page | to do |
| 45 | f389447e | Its addresses as two rows | to do |
| 44 | cf99cf58 | A Changelog of public commits; View profile page button | to do |
| 66 | 58dd86ef | Its info tip names no other app | to do |
| 53 | ee9966b7 | No background photo option | already |

## Notes (needs Notes itself first, 0f5c08f0 below)

| # | Commit | What it does | Status |
|---|---|---|---|
| 41 | f6921eaa | Heading sizes, task boxes, Return continues lists | to do |
| 42 | ffaaf7b4 | ⌘1 leaves Notes; the notes list folds away | to do |
| 40 | aba5a97e | Note headings in text colour | to do |
| 13 | dbf82191 | Notes save (no file protection) | to do |
| 27 | 7ebe4e66 | Notes save (no file protection) | to do |
| 90 | f1510297 | One Settings… (already); Notes never asks for the Mac password | to do |
| 88 | 95cdefc8 | A code comment | nothing |
| 64 | 1952b193 | Back and Forward (already); ⌘5 Skills and ⌘6 Notes | to do |

## Voice (skip)

| # | Commit | What it does | Status |
|---|---|---|---|
| 84 | d1855428 | Fn keys to dictate, a quieter microphone button | skip |
| 76 | 8ad536b8 | Shift Fn hands-free; double tap off | skip |
| 81 | a4a6dbc0 | A button to System Settings > Keyboard | skip |
| 78 | 2c27a35d | Open Keyboard Settings through OpenGate | skip |
| 80 | 37f5c9fe | Dictation stops typing phrases twice | skip |
| 87 | a7c523ea | Talk into a note with Voice | skip |

## Connectors (skip: the GPUI app keeps upstream's panel)

| # | Commit | What it does | Status |
|---|---|---|---|
| 74 | d89ee34b | Set up, Add a service, Connect Grok Bot | skip |
| 73 | 31362baf | A gateway on this Mac is "This Mac" | skip |
| 72 | 19a9f651 | A new Grok Bot key, box by box | skip |
| 71 | 21a14ec5 | × hides a row, + hidden brings it back | skip |
| 70 | 08f7d91a | Connect an agent: Grok Bot, Notion or Slack | skip |
| 68 | 84a4f584 | + hidden menu shows icons | skip |
| 65 | 05bae648 | Local connectors like Blender's | skip |
| 18 | 991f0ef4 | X search row in the Connectors panel | skip |

## Already in the GPUI app, or nothing to rebuild

| # | Commit | What it does | Status |
|---|---|---|---|
| 2 | 1042bea6 | Take out Profiles | already |
| 4 | 2ee83b55 | Take out Ask mode | already |
| 30 | 0ad26c0e | Slash palette on the box's display | already |
| 35 | 34aef3be | Slash palette stays on screen | already |
| 36 | 49142e5c | New Folder in every folder picker | already |
| 69 | 8109a502 | Preview / Markdown menu icons | already |
| 77 | a713469b | Settings window drags wider | already |
| 82 | d3ef3ffe | Settings wider, window drags wider | already |
| 89 | 54416877 | One Settings… in the app menu | already |
| 94 | 791bd757 | A ding when a thread finishes | already |
| 97 | fd68ea4e | /skill lists as you type, anywhere | already |
| 24 | 4298c5dc | Voice and Profiles settings start at top | nothing |
| 26 | 3b3d772b | A docs example | nothing |
| 55 | 16fb4bdb | The Swift release build compiles | nothing |
| 83 | 9b358dda | Ask runs only reads (Ask later taken out); skills can't pre-approve (runner) | nothing |
| 85 | 5b5bc1dd | Settings > Profiles (later taken out) | nothing |
| 86 | 1286a735 | Profiles in the app (later taken out) | nothing |
| 91 | d5c9aefa | Fewer approval round trips (runner side) | nothing |
| 95 | 9f16959c | Comment wording | nothing |
| 96 | 54fd8c92 | Shift-Tab with Ask (Ask later taken out) | nothing |
| 99 | 8d9fffd2 | Plan and Ask modes (Ask later taken out) | nothing |
| 100 | 20cf6a3b | Saving a new skill no longer crashes | nothing |

## Older changes, past the panel's 100

These show in the Updates panel once the list above shrinks. Not yet checked
against the GPUI app.

| Commit | What it does | Status |
|---|---|---|
| 4d3524c6 | Closing the Terminal no longer leaves the thread blank | already |
| 99b4038f | Panels stay inside their space in a narrow window | to check |
| e19a3f06 | ⌘/ collapses the Threads panel | to check |
| f248d332 | Quit asks first, with Don't ask again | to check |
| 5e6c8146 | Attach Image takes Markdown files | to check |
| 30fe46cd | Question card descriptions wrap | to check |
| bf73c636 | Markdown files open drawn, Preview / Markdown menu | already |
| ed662aab | Settings > General > Experiments switches | to check |
| b4370100 | New Thread on a project's page starts in the project | to check |
| 1d47f50c | Files panel: clicking opens again | already |
| b79385cb | Web addresses in inline code are links | to check |
| ba5d5cfc | Standing instructions for every session | to check |
| 58151848 | New Skill | to check |
| 52e060a3 | Files and Terminal on the rail | to check |
| 0f5c08f0 | Notes, and Voice/skills/notes switches | to do (Notes part) |
| 3d531d54 | Skills page | to check |
| f9f3c8e4 | Settings over the main panel | to check |
| 1d311b94 | Your messages show basic Markdown | to check |
| c3603b16 | The bear's chat has Voice | skip (Voice) |
| 540cc171 | The bear: a chat at the top of the sidebar | to check |
| 25510a20 | Voice avatar | skip (Voice) |
| bcd71e48 | Voice approvals out loud | skip (Voice) |
| f44b37c4 | Voice echo cancellation | skip (Voice) |
| c2af85a6 | Voice mode | skip (Voice) |
| 57a73caf | Appearance picker for Threadbear alone | to check |
