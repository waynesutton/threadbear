# Bears, tables and webhooks in the GPUI app

The runner side shipped on Wayne's fork on 13 Nov 2026, behind the `bears`, `webhooks`, `tables` and `bearNotes` experiments (`prds/bears-for-mike.md`). The SwiftUI app is frozen, so every piece of UI waits for the GPUI app in `desktop/`. Until then, Wayne's home patch `~/.threadbear/patches/bears.ts` gives a Bears & Tables list panel and three chips.

Each item below lists the runner calls it needs. All of them exist today, so none of this needs a runner change unless it says so.

## Bears and tables in the sidebar

A foldable **Bears** section under Starred, shown only while `hello.experiments` has `bears`.

- One row per bear from `bears.list` (the head bear first, then pinned, then Wayne's order): its face, name, and a detail line. The line shows the current thread's title while the bear works, "Away until Mon 9:00" while it hibernates, and its `does` line otherwise.
- The face is one of `bears.list`'s SVG stills (`faces.idle`, `faces.thinking`, `faces.happy`), matched to the bear's newest thread: thinking while it runs, happy once it's done, idle otherwise. Draw them through the same SVG path the app uses for Pele diagrams. No live canvas per row.
- Click opens the bear's newest chat, or a new-thread screen in its folder (`bears.list[].folder`) showing its name and face.
- A **Tables** group under it from `tables.list`, each row naming its bears.
- Drag to sort calls `bears.move`. Pin calls `bears.pin`.
- Runner calls: `bears.list`, `bears.move`, `bears.pin`, `tables.list`. No runner change.

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

## The table chat

A table's thread is an ordinary thread in its folder. Bear replies arrive as messages that start with `[Bear: <name> replied in thread:<id>. ...]`, followed by the reply quoted.

- Draw those as a bubble with the bear's face (happy mood) and name, and a link to its thread, rather than as Wayne's message.
- When the runner refuses a mention past `maxTurns`, the host says so in its reply. Nothing extra to draw.
- Runner change: none needed. A later protocol field (`origin: 'bear'` with the bear id on the user.message) would be cleaner than reading the prefix; add it to `docs/protocol.md` first if the GPUI port wants it.

## The @ picker in the composer

Typing `@` opens a picker of bears (from `bears.list`) and recent threads, filtered as you type.

- Picking a bear inserts `@bear:<id>`. The runner hands the message to that bear when it's sent, at most three per message.
- Picking a thread inserts the same thread chip that pasting a thread id makes today.
- Follow the SkillPicker pattern: a child panel that is never key, never `NSTextView.complete`.
- No runner change.

## Webhooks on the Scheduled page

A **Webhooks** section on the Scheduled page (⌘2), while `webhooks` is on.

- Rows from `webhooks.list`: name, target (a bear, a folder or a thread), last used, and Revoke (`webhooks.revoke`).
- **New webhook…**: name, then target (a bear, a folder, or this thread), then Ask or Auto, then `webhooks.create`. Show the URL once, with Copy, as Connectors does for agent keys, and say to revoke it if it leaks.
- With no public site (`webhooks.list` returns `reason`), the section shows that reason.
- No runner change.

## Bear notes

The librarian's cheers and ideas already show as text in its Highlights line ("- Researcher: Nice, tests pass and it's pushed."). Draw that line with the bear's face.

- An idea in the done card's "Or carry on" list needs a runner change: today the list comes from the turn's own `turn.completed` event, which is written before the librarian looks. Add `needs.next` from the librarian (a `bearNext` field on the thread summary) to `docs/protocol.md` first, then both sides.

## Checks before landing

- A shot of each piece beside the mockup Wayne picks, through show_options.
- Specs in `features/` for the sidebar section, the menus, the sheets, the picker and the webhooks list, so `docs/gpui-port/PARITY.md` lists them.
- `EveryClickShowsTests`-style coverage for each call that changes something on the runner.
