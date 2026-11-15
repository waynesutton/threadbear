# Notes in the GPUI app

Created 2026-11-15 UTC. Wayne asked for the Swift app's Notes, its Settings
switch and its ⌘6 shortcut in the GPUI desktop app.

## Why

Notes is the one page in the bar that the Swift app has and the GPUI app
doesn't. Wayne uses the GPUI app every day, so his notes are out of reach there.

## Behaviour

- The bar on the left gets a Notes button below the Librarian, with ⌘6
  (`showNotes`, the key the Swift app and shortcuts.json already use). It
  shows only while Settings > General > Show Notes is on.
- Notes opens over the main panel, the way Settings does: the thread, its
  draft and the sidebar stay put underneath.
- The page has a header (a fold button for the list, "Notes", a lock that
  says the notes are encrypted, New Note and an X), the list of notes newest
  first, and the open note: a title you can click to rename, when it was
  last changed, a bin, and the text.
- The text box is a code editor with line numbers in a fixed-width font.
  Basic Markdown is coloured as you type: headings, bold, italics, code,
  code blocks, list marks, links and quotes, with the marks dimmed. A
  `- [ ] ` box ticks with a click, and ⌘Z undoes it. Return on a list line
  starts the next item, and Return on an empty item ends the list.
- Notes save half a second after typing stops, and on leaving the page,
  switching notes and quitting.
- Esc, the X, Back, ⌘1 to ⌘4, a bar view, a thread, a project or Settings
  leave Notes. Pressing ⌘6 or the button again also closes it.
- Delete asks first ("It can't be brought back.").
- Turning Show Notes off hides the button, closes the page and keeps the notes.

## Storage

The same files as the Swift app, so both apps open the same notes:
`<data dir>/notes/<id>.tbnote`, each a version byte (1) followed by AES-256-GCM's
nonce, ciphertext and tag over the note's JSON (`id`, `title`, `body`,
`createdAt`, `updatedAt`, dates as seconds since 2001-01-01). The key is
`<data dir>/secrets/notes-key`, 32 bytes, 0600, in a 0700 folder. Files are
0600 and the notes folder 0700. If there's no key file, the GPUI app makes
one only when the notes folder has no notes yet. The Keychain key that the
Swift app moved out on 7 Nov stays the Swift app's job: with notes but no key
file, the page says the notes couldn't be unlocked, and nothing is replaced.

A mock or dev runner's app uses that runner's data folder, never the real one.

## Not in this port

- Voice dictation into a note: the GPUI app has no Voice yet.
- Headings drawn bigger by level: the GPUI editor colours ranges but can't
  size them, so headings are bold and coloured at the body size.
- Bringing the Swift app's `notesShown` and `notesListHidden` across: the
  settings import already ran once, and both start as the Swift app's
  defaults do (shown, list showing).

## Scope

New: `desktop/core/src/notes.rs` (note, sealing, names, highlighting, list
typing, the store's files) and `desktop/gpui/src/notes_page.rs`. Changed: the
shortcut list, the bar, the command table, navigation, Settings > General,
the dump and scripted commands, docs. `ring` 0.17.14 is added to
`desktop/core` at the version already in the build. No runner, protocol,
Convex, Swift or iPhone changes.

## Verification

Core unit tests port the Swift `NotesTests` and add a check that a file
written by the Rust code opens with the Swift layout, and the reverse
(a fixed sealed note made by CryptoKit). Specs in `features/notes.feature`.
Drive a mock dev app: open with ⌘6, make, rename, type, tick a box, Return
on a list, delete, leave by Esc, X and ⌘1, and the Settings switch. Look at
dark-mode shots. Then `tb land`, and check the installed app.
