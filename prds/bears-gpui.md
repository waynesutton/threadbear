# Bears, tables and webhooks in the GPUI app

The runner side shipped on Wayne's fork on 13 Nov 2026, behind the `bears`, `webhooks`, `tables` and `bearNotes` experiments (`prds/bears-for-mike.md`). The SwiftUI app is frozen, so every piece of UI waits for the GPUI app in `desktop/`. Until then, Wayne's home patch `~/.threadbear/patches/bears.ts` gives a Bears & Tables list panel and three chips. Three pieces now exist in the SwiftUI app as well (Wayne's exception to the freeze, 13 and 14 Nov): the bear tiles and Bear Colour menu in the sidebar, the @ picker in the composer, and a table's bears on its project page. Port those three from the Swift code and `features/bears.feature` (rows `bears.tile`, `bears.colour`, `bears.mention` and `bears.tablepage` in `gpui-port/features.md`) rather than designing them again.

Each item below lists the runner calls it needs. All of them exist today, so none of this needs a runner change unless it says so.

## Bears and tables in the sidebar

A foldable **Bears** section under Starred, shown only while `hello.experiments` has `bears`.

- One row per bear from `bears.list` (the head bear first, then pinned, then Wayne's order): its face, name, and a detail line. The line shows the current thread's title while the bear works, "Away until Mon 9:00" while it hibernates, and its `does` line otherwise.
- The face is one of `bears.list`'s SVG stills (`faces.idle`, `faces.thinking`, `faces.happy`), matched to the bear's newest thread: thinking while it runs, happy once it's done, idle otherwise. Draw them through the same SVG path the app uses for Pele diagrams. No live canvas per row.
- Click opens the bear's newest chat, or a new-thread screen in its folder (`bears.list[].folder`) showing its name and face.
- A **Tables** group under it from `tables.list`, each row naming its bears (`table.bears`) and showing the project's icon (new tables get the `person.3` symbol).
- Drag to sort calls `bears.move`. Pin calls `bears.pin`.
- Runner calls: `bears.list`, `bears.move`, `bears.pin`, `tables.list`. No runner change.

## Bear tiles in the sidebar (in the Swift app, `f69b7232`: port it)

A thread's summary carries `bear` (id, name, icon, color, head) while bears are on. The Swift sidebar draws the bear's SF Symbol in white on its colour in place of the folder or project tile, and a bear thread's right-click menu has Bear Colour (eight swatches, `bears.update { color }`). Port both to the GPUI sidebar. No runner change.

## Menus for every bear action

Right-click on a bear row:

| Item | Call |
|---|---|
| New chat | `threads.create` with `cwd` = the bear's folder, no model (the runner fills in the bear's) |
| Rename… | `bears.update { name }` |
| Edit persona | opens `<folder>/CLAUDE.md` in the side pane |
| Change face… | the face picker below, then `bears.update { avatar, color }` |
| Add routine… | the schedule editor with `cwd` prefilled |
| Add webhook… | the New webhook sheet with the bear picked |
| Make chief of staff | `bears.setHead` (starred when it's the one, per the menu rule) |
| Pin / Unpin | `bears.pin` |
| Away until… | `bears.hibernate { until }`, with Wake when it's away |
| Archive / Unarchive | `bears.archive` |
| Access… | `bears.setAccess` (mode, pool, machine), with a tooltip saying a bear's own thread can't change it |
| Show folder | reveals the folder |
| Delete… | a confirm sheet, then `bears.delete` (its folder goes to the runner trash) |

Every item uses `whileBusy` and gets a tooltip. No runner change.

## New bear and New table sheets, and the face picker

- **New bear**: name, what it does, personality (one line), model and effort menus, then `bears.create { name, does, personality, model, effort, avatar, color }`.
- **Face picker**: a grid of `bears.faces` presets drawn from their SVG stills, a Shuffle button that picks one at random, and a colour well. Each cell gets a tooltip with the preset's name.
- **New table**: title, goal, a multi-pick of bears, and an optional parent project, then `tables.create { title, goal, bears, parent }`.
- No runner change.

## A table's bears on its project page (in the Swift app, `38c291b6`: port it)

`Project.table` is decoded in the Swift app (`ProjectTable`), and the project page has a Bears at this table section: each seated bear's tile, name and what it does, from `bears.list`. Port it to the GPUI project page.

- Seating happens on the runner: a bear's thread put in a table's project (the thread's Projects menu, an agent's `join_project`) seats that bear (`Projects.onJoined`, `Tables.seatFromThread`). Taking the thread out doesn't unseat it. So the GPUI Projects menu needs nothing new; the section just follows `project.table`.
- `tables.create` sets the project icon to `person.3`, so tables look like tables in every project list and menu. Draw the project icon as usual.
- An Add bear… button on the section could call `tables.seat { add }`, and a row's menu Remove from table `tables.seat { remove }`, both behind a busy key. Not in the Swift app yet.
- No runner change.

## The table chat

A table's thread is an ordinary thread in its folder. Bear replies arrive as messages that start with `[Bear: <name> replied in thread:<id>. ...]`, followed by the reply quoted.

- Draw those as a bubble with the bear's face (happy mood) and name, and a link to its thread, rather than as Wayne's message.
- When the runner refuses a mention past `maxTurns`, the host says so in its reply. Nothing extra to draw.
- Runner change: none needed. A later protocol field (`origin: 'bear'` with the bear id on the user.message) would be cleaner than reading the prefix; add it to `docs/protocol.md` first if the GPUI port wants it.

## The @ picker in the composer (in the Swift app, `debffacf` and `57934753`: port it)

Typing `@` opens a picker of bears (from `bears.list`) and recent threads, filtered as you type.

- Picking a bear inserts `@bear:<id>`. The runner hands the message to that bear when it's sent, at most three per message.
- Picking a thread inserts the same thread chip that pasting a thread id makes today.
- Follow the SkillPicker pattern: a child panel that is never key, never `NSTextView.complete`.
- No runner change.
- Keep the Swift app's three rules from its review: list bears only when the message goes to this Mac's runner, leave modified keys (Shift-Return, Shift-Tab) to the composer, and stay shut after Esc or losing focus even when a slow `bears.list` lands (`BearPicker.Gate`).

## Webhooks on the Scheduled page

A **Webhooks** section on the Scheduled page (⌘2), while `webhooks` is on.

- Rows from `webhooks.list`: name, target (a bear, a folder or a thread), last used, and Revoke (`webhooks.revoke`).
- **New webhook…**: name, then target (a bear, a folder, or this thread), then Ask or Auto, then `webhooks.create`. Show the URL once, with Copy, as Connectors does for agent keys, and say to revoke it if it leaks.
- With no public site (`webhooks.list` returns `reason`), the section shows that reason.
- No runner change.

## Bear notes

The librarian's cheers and ideas already show as text in its Highlights line ("- Researcher: Nice, tests pass and it's pushed."). Draw that line with the bear's face.

- An idea in the done card's "Or carry on" list needs a runner change: today the list comes from the turn's own `turn.completed` event, which is written before the librarian looks. Add `needs.next` from the librarian (a `bearNext` field on the thread summary) to `docs/protocol.md` first, then both sides.

## Pictures from Codex bears

A Codex bear (a Designer on `gpt-6-astra`, 13 Nov) makes real PNGs with Codex's built-in `image_gen` and saves them in its folder. Show them well:

- A picture a bear's reply names or pins draws inline in the transcript at a readable size, with Open and Show in Finder, and the same when the reply comes back into the asking thread or a table chat.
- A bear's tile or row can show its newest picture as a thumbnail while it's working on images.
- Say on the card or the row that Codex wrote the picture to `~/.codex/generated_images` without asking, as SECURITY.md says, so Ask isn't taken for a gate it isn't.
- Runner change: none for drawing. A picture's path is already in the reply and in `pin`.

## Checks before landing

- A shot of each piece beside the mockup Wayne picks, through show_options.
- Specs in `features/` for the sidebar section, the menus, the sheets, the picker and the webhooks list, so `docs/gpui-port/PARITY.md` lists them.
- `EveryClickShowsTests`-style coverage for each call that changes something on the runner.
