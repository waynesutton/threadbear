# Using Threadbear

<!-- Each `## ` section is also a page on the website, at
     threadbear.mikecann.app/docs/<slug>, named by the slug comment on the
     line under its heading (site/scripts/docs.mjs builds them). The Mac
     app links to some of those pages (site/app-doc-slugs.json lists them),
     so keep a section's slug when you rename its heading. -->

Threadbear is a Mac app for running AI agent threads. It drives the official
Claude Code and Codex command line tools on your own logins. A small program
called the runner keeps every agent going in the background, so quitting or
updating the app never stops a thread. Your threads are also copied to a
Convex deployment of your own, which is how the iPhone app and the website
can follow along.

This guide is for using it. `INSTALL.md` covers setting it up, and `docs/`
has how it all works inside.

## Threads
<!-- slug: threads -->

<!-- picture: /clips/inbox.mp4 alt: Your threads down the left, the one you picked in the middle, and its Highlights on the right -->

A thread is one conversation with an agent, working in one folder on your
Mac. You say what you want done, the agent does it (reading files, running
commands, making changes) and replies. You can have as many threads going as
you like, and each one remembers its whole conversation, so you can come
back to it days later and carry on.

### The list

The threads view has four sections. Inbox holds the threads for you, with
the ones that need you at the top. The number on the Threads icon in the
left-hand rail is how many are in it. The arrows at the end of its heading
sort it newest first or oldest first instead, which is handy for working
through a long inbox from the bottom up. Whatever the order, a thread that's
stopped mid-turn on a card for you (a password, a question, a permission)
stays at the top, as the card closes if nobody answers it. ⌥⌘↓ (Go > Next Inbox Thread)
opens the thread below the open one in the Inbox, or the Inbox's first if
the open one is somewhere else; past the last one, a note says you're
caught up. Once you've clicked the list, the up and down arrows move through
the threads, and Esc in the Scheduled, Projects or Librarian view goes back
to the Inbox. The list scrolls to keep the open thread's row in sight,
whether you got there by the arrows, ⌥⌘↓ or a thread named in a reply. When your threads pass 100, and each thousand after (archived ones
count), a note says so once, the next time you're at the app, with a burst of confetti. Running holds threads whose agent is
working now, and one you've just sent a message to: while your message
still has its spinner (a busy Mac can take a while to pick it up), the
thread has already left the Inbox. If the message can't go, it comes back. A thread that
asks you something keeps its place in the inbox, wake-up or not, and so does
one that has left a job running while it waits for you (a preview server
you need, a job watching for you to do the step): a small grey
arc turning by its time says the job is still going, and hovering it says what it is. Sub-agents are
different: while they're working the thread stays in Running, even if it has
asked you something, as what they find can change what it asks. It comes to
the inbox when they're done, and you can still open it and answer early.
The same goes for a job the thread is itself waiting on, like a landing or a
build: the agent is asked which kind its job is when it has a question for
you with one still running, and if the job has to finish first, the thread
stays in Running until it has.
Hibernating and Archived, at the foot, hold the threads that are put away
(see [Hibernate and archive](#hibernate-and-archive)). They start folded,
showing only how many; click a heading to open or fold it. A thread whose agent
is waiting on its own work stays under Running too, with a grey spinner and
what it's waiting for: on other threads to finish (a fix it needs, a review of
its change), for something it's watching for while its commits aren't on
main yet ("Waiting to land"), or for your app to restart onto what it landed.
Its card says
that rather than Done, as the job isn't over until it is. Hibernating also
has the threads that have booked a message to themselves for later (an
agent saying "I'll check again in 20 minutes") and have nothing for you
until then. Each row says when it wakes, soonest first. When the time comes
one of these runs again, and lands in the inbox when it's done.

Your runner decides which of these a thread is in, so the Mac, the iPhone,
Android and the website all sort it the same way, and so does another Mac
looking at this one's threads. A thread still asking you something counts in
the badge until you've done it, whether or not you've read it. One whose login
has run out of usage stays in the inbox and counts too, as you may want to move
it to another login; it carries on by itself when the limit resets.

To keep a thread at the very top, star it: hover its row and press the star,
or use its right-click or ... menu. Starred threads sit in a Starred section
above the Inbox, whatever they're doing, hibernating ones too, in the order you
starred them, so they don't jump around. Archiving one moves it to Archived;
bring it back and it's starred again.

The search box at the top, beside the New thread button, looks through
every thread, the put-away ones too. Type part of a title, or of a thread's
last reply, and each section shows only the threads that match, with folded
ones opened. It also matches a few search words the librarian gives each
thread, so "dentist" finds a thread called "Wisdom tooth meeting location".
The x clears it.

### What a row tells you

Each row has the thread's title, with its project under it (or its folder,
when it's in none). Hover that line to see the last thing you said to it.
The tile before the title shows the project too. Your phone's list shows the
same line. A title in bold needs you or has a reply you haven't read.

A thread you didn't start has a small mark before its project: a ladybug
or other tool symbol for a tool such as Sentry, a clock for a schedule, a
pawprint for another thread's agent, books for the librarian. Hover the mark
to see who started it and why.

Before the title, in the gap at the left:

- **A blue dot**: the thread has finished and is waiting on you, with a
  reply you haven't read yet. It stays near the top of the inbox.
- **A tick**: the thread has finished and said there's nothing needed
  from you. It shows until you open the thread, so you can see at a
  glance what got done while you were away.

At the right of the row:

- **Working**, with a spinner and a timer: the agent is busy. In the
  Running section the spinner and timer are enough, so the word is left
  off.
- A grey spinner with something like "2 sub-agents": the turn has ended
  but helpers or jobs it started are still going.
- The blue dot, with an approval card at the end of the thread and no
  grey line: a sub-agent is waiting on your answer. It waits as long as it
  takes, whatever the agent does in the meantime, and carries on when you
  answer. Stop closes the card and stops that sub-agent.
- A time: the thread is idle, or the blue dot on the left says it's
  waiting on you. After a while Threadbear stops an idle
  thread's agent to save memory, and your next message starts it again
  where it left off.
- **Failed**: the last turn ended with an error. Read the thread to see why,
  and send a message to try again.
- **Waiting for connection** (on the Mac and the iPhone): its turn failed
  because your internet went.
  It carries on by itself once the connection is back, so there's no need
  to tell it to continue. A thread cut off mid-turn because the Mac or
  Threadbear restarted carries on by itself too, once it's back up.
- **Stopped**: the runner restarted while the thread was busy.
- **Waiting for room**, with a time: every login it could use has hit its
  limit (see [Accounts, pools and Usage](#accounts)). It carries on by
  itself when one has room again. A box at the top of the message field
  says so, with a button to switch to the login with the most room, and
  Carry on by itself (Yes or No) to have it wait for you instead.

Your messages from the phone app have a small phone beside the words or
pictures. Hover or click it to see where the message came from.

### Reading a reply

When an agent finishes a piece of work, its reply is the message it
writes at the end: what it did and what it found. While it's being
written, each new piece fades in as it arrives. Everything it did to
get there, its notes, thinking, commands and edits, is one line above the
reply, "Worked for 2m 13s": click it to open it. Pictures it made stay
out, as they're often the answer. When the agent carries on by itself
after that (to check its change once the app restarts, say), the
follow-up joins the same line, so there's still one reply to read. When
it picks up again because a sub-agent or a background job finished, a
thin line between its messages says how long it waited and what finished
("6 min later · Mine the repo finished"), or what Threadbear told it
when its own message started the turn ("5 min later · Codex answered").
The iPhone and Android apps and the website draw the same line. When
it needs something from you, a card under the reply asks, and each answer
says what happens if you pick it. If you'd rather have a few short
bullets in place of the agent's message, ask a thread to turn off One
reply (`"oneReply": false` in runner.json). A link with ↑ after it goes
to the step it's about: click
it and the work opens there, with that step lit up. A link to another
thread opens that thread.

To find something said earlier, press ⌘F (Ctrl-F on Windows and Linux) in
the new desktop app: a find bar comes up over the thread and counts the
matches as you type. Return or ⌘G goes to the next one and Shift-Return or
⇧⌘G to the one before, going round at either end; Escape or Done closes it.
It finds words in the work too, so a match in a command's output or a
thought opens that line, and the "Worked for" line it's folded into, and
folds them again when you move on. Matches in a reply are marked in its
words, and a bar beside the row says which one you're on. It searches the
part of the thread that's loaded: going back past the first match loads
the earlier part. With a file showing in the pane and clicked into, ⌘F
finds in the file instead.

An agent can draw a diagram or a small page in its reply: a ```` ```mermaid ````
block shows as the diagram (drawn with [Pele](https://pele.run/), in
Threadbear's colours, light or dark), and an ```` ```html ```` block as the
page, each in a box with the language on the left. Its Diagram / Source
switch (Preview / Source for a page) flips it to the code, the copy button
copies the code, and the arrows open it full size in a window of its own. A
diagram that can't be drawn says why above its code. A page is only shown,
never run: its scripts are off, it can't reach the internet or your files,
and it can't open anything, so only its own styles, SVG and `data:`
pictures appear. Your phone draws them the same way, with the arrows
opening it full screen. The new desktop app draws
them with Chromium, which it downloads the first time one needs it: the box
asks "Show this diagram? It needs the web runtime, a 135 MB download, once."
with Install web runtime and Not now, and shows the code until then (or
after Not now, until the app restarts). Settings > General > Web runtime
says where it's got to and installs it too. If Chromium doesn't start, a
web panel and that row say why, with Try again, or Restart Threadbear when
only a new run of the app can try again (Chromium starts once per run);
the window and what you've typed stay. Its own log is `web-runtime.log` in
the `web-runtime` folder of Threadbear's data folder.

The installed Mac runner checks the web runtime's CEF pin daily while a
runtime is installed. Updates and
`tb status` warn if it is two weeks behind stable, or the check has failed
for three days, when an agent can update it. While it's only waiting on
cef-rs, the Rust bindings Threadbear needs for the newer CEF, that's a quiet
line in the Updates panel, with nothing flagged. Bringing in a Threadbear update carries a newer pin when
one is available. Chromium's sandbox stays off so the runtime can be
downloaded on first use; keep these updates current.

Text an agent writes for you to send somewhere else (a Slack message, an
email) comes in a ```` ```slack ````, ```` ```email ```` or
```` ```draft ```` block. It shows in a box the way it'll go out: every
line where the agent put it (the rest of a reply joins single lines up,
as Markdown does), and a Slack draft's `*bold*`, `_italic_`, `~strike~`
and `<url|label>` links drawn as Slack draws them. Copy, at the box's
top right, puts it on the clipboard with its links and bold, so it
pastes into Slack or Mail as it looks. The phone and the website show it
the same way.

To change a draft before the agent sends it for you, press Edit beside
Copy, change the words and press Done (or ⌘Return). The box says
"edited" and shows your version, and Undo puts the agent's back. Your
version goes on the end of your next message to that thread (an answer
on its card counts, from the thread or a project page), with a line
asking the agent to use your words, so picking "Post and close" posts
what you wrote. Until then a chip over the message box says so ("Your
edited email · 2 lines changed"): View shows the draft in its reply,
and × puts the agent's back, as Undo does. Once sent, your message
shows the same chip under your words in place of that line, and Show
opens the draft you sent; the website and the phone show it that way
too. The change is kept on your Mac, in the desktop app.

A reply that needs nothing from you ends with a Done card at the top of
the message box, with an Archive button to put the thread away. When something it talked about is
still worth doing (a piece it didn't get to, a check that's due), the
card lists them under "Or carry on". Click any of them to tick them
(click again to untick), add anything you like in the message box, and
send: the ticked ones go to the agent together, with your words. Not
sure what one means? Click the ? at its right to ask; it toggles the
same way, so you can ask about several, tick others, and send them all
together.

Before you archive, the Done card says what's still open, so you don't
have to ask the agent first. Threadbear checks it itself: commits on the
thread's worktree branch that aren't on main, files in its worktree that
aren't committed (or, for a thread with no worktree of its own, files it
changed that aren't committed and commits it made that aren't pushed),
work it pushed that isn't on main yet (naming the PR when one is open on
GitHub), threads it started that are still working or waiting for you,
and a background job it's still running (archiving would stop it). The
agent adds the things only it knows, like a reply it's waiting on from
someone. They're listed under "Still open": a thread opens from its line,
and Land is the chip above the message box. A check-in it booked shows
under "Coming back", as the thread comes back by itself when it runs. If
nothing is left, the card just says Done, so Archive is safe. It checks
again when you open the thread, after an action like Land, and when a
thread it started finishes. The phone's and the website's Done cards
show the same list, though not for a thread on another Mac.

When
the reply does need you, a Needs you card takes its place in the message
box, outlined, listing what to do; the same list is near the top of the
thread's [Highlights](#highlights), with a box to tick off beside each
one. Its heading says what's wanted ("Decisions needed" when every ask
has answers to pick). When there are several asks and they'd take a lot
of room, the card shows one at a time, with ‹ 1 of 3 › by the heading;
picking an answer moves on to the next ask still waiting for one. A long
ask scrolls inside the box, so the reply stays in sight. The chevron at
the card's top right collapses it to its heading, to read the reply behind
it, and brings it back; the next card opens again. Once you reply, the
card goes.

With the **Clearer thread completion** experiment on (Settings > General >
Experiments), the agent can report progress on every task in your request.
The card shows how many are completed and which are pending or blocked,
including work carried across turns. "Reported by the agent" labels that
list. The card goes once you reply, but the list stays in the thread's
[Highlights](#highlights) under **Task list**, so you can look at it again
without asking. Threadbear checks loose ends separately; only a successful empty
check says "No loose ends found". An unavailable check makes no such claim.
To omit automatic PR reminders for particular GitHub repository owners,
set `looseEndsIgnoredPrOwners` in runner.json to their owner names. This
keeps unpushed commits and other local-work warnings. The preference applies
when the runner next starts; agents should also respect it in their replies.
An agent that worked on something and left the list out is reminded once
before its turn ends, whichever model it is.
Action buttons such as It works and Archive keep their neutral fill with
a thin amber left edge. Archive stays manual. A thread without reported
tasks keeps its usual card; turning the experiment off restores that view.
In the desktop app, a done card with tasks, loose ends or follow-ups has the
same chevron beside Archive: it folds the card to its heading line until the
next card comes.

To answer a Needs you card, click its buttons: your picks show on the
card and go with your next message. A button you pick fills and hops, so
you can see the click land. To answer an ask in your own
words, click Something else under it and write in the box that opens
there. You can paste or drop screenshots in that box too, or use its
picture button, and each goes in where the cursor is as a chip, as in
the composer. Each ask can have a box of its own. Return in a box takes
you on to the next ask that hasn't got an answer yet (a box left empty
doesn't count as one; Shift-Return adds a line), and once they all have one it sends the lot as one message, with
anything you wrote in the message box at the end. Return in an empty
message box does the same with your picks. To send with some left
unanswered, press ⌘Return.

When an ask is about pictures the agent put up to compare, its button can
open that comparison window straight from the card.

A reply can have buttons partway through it, beside the line they're
about, so you can set jobs off as you read without typing: one starts a
new thread with a brief the agent wrote ("Fill in the form"), another
sends a message to a thread that's already going, and a third opens a
panel from another folder's patches beside the thread (the Daily brief's
"Go through comments" opens the YouTube panel), as often as you like. Hover over a button to
see the brief and where it runs. Nothing happens until you press it, and
it only runs once: after that it turns green and opens the thread it
started. The agent hears which you pressed with your next
message, so a go ahead in your reply doesn't set the same job off twice. It works the same on your phone and the website, except a panel button, which only works on the Mac. A thread
without Full access can't make a button that does more than it could
itself, and that's checked again when you press it.

On the iPhone, tap Reviews in the thread's top bar, or a review link in a
reply. The sheet shows the options, their notes and a still of each
picture. Pick any options, None of these, or write a reply, then Send
answer. Reference options are there to compare and can't be picked. The
thread's Mac has to be on to load the review and take the answer, and it
sends the stills only once you open the review, so they can take a moment
to turn up. Pages, videos and sounds show their still; open the real thing on the
Mac. Pictures pasted into an answer on the Mac stay with it, though the
phone doesn't show them. This needs your backend on API 86 or newer. If
the review changes on the Mac while you're answering, your answer stays
in the sheet while it reloads, ready for you to check and send again.

In the thread, a review's card shows the agent's lead and each option's
small picture and name. Click one to open the comparison window on that
option, or the card's button to start on the first. The picture has the
whole width of the window, with back and next beside it. Under it is a tab
for each option, with its tick, and under the tabs the name of the option on
show and the agent's reasons for it, at the size you read replies in the
thread, so you read about each one while you're looking at it. An option
can have several pictures (its overview, how it looks mid-drag, a detail),
shown in a strip under the picture. Next and back (or the left and right
arrow keys) step through every picture of every option, up and down jump to
the option before or after, and after the last one they're all side by
side, with the agent's lead under them. Click a tab or one of the strip's
small pictures to go straight to it, the first tab to see them all, and
Open live at the end of the tabs to try one the agent has running (a web
page only). A review from a thread on another Mac opens on that Mac. An
agent can show a video, a sound or an HTML page in place of a picture: videos play
quietly on a loop, side by side when they're all showing, with controls
for sound when one is big; a sound (MP3, M4A, AAC or WAV, say music beds to
compare) shows its waveform, with a play bar under it when it's big; a page
shows as it does in the side pane, with its scripts off. When
the agent includes today's version to compare against, hold Space (or click
its button at the end of the tabs) to flick to it and back. To look closer at a
picture, scroll or pinch over it to zoom in where the pointer is (a long page
starts small enough to see whole). Then scroll or drag to move around, or use
the small picture in the corner to jump to a part. Pinching out, ⌘0, Esc or a
double-click goes back to the whole thing, and ⌘+ and ⌘- zoom too. Tick as
many as you like, or None of them if none will do, write what you think in
the box at the bottom right, and Send takes it all back to the thread.
Pasting with the keyboard outside the reply box puts the words in it and moves
the cursor there, so you can dictate just after ticking an option.

Short tables in replies stay as wide as their words need. Wide tables fit or
scroll inside the reply, and list items have space between their lines.

Inside a thread, rest the pointer on a link for a moment, in a reply or on a
Needs you card's buttons, to see where it goes: a picture of the file, or
the page's address. On a Mac a local PDF shows its first page and a video
its first frame. A PDF over 16 MB or a file that can't be previewed keeps
its file symbol. Click it to open it (the middle mouse button works
too). A file the side pane can draw opens there: Markdown, text, a page, a
picture, a PDF, a video or sound file on a Mac, data and logs (JSON, YAML, TOML, CSV, `.log`) and code
(TypeScript, Python, Rust, shell scripts and the rest), data and code
coloured. Anything else opens in its own app, and ⌘-click opens any file
there. A path with lines after it, in a link or in a reply's code
(`server.ts:12` or `server.ts:121-125`), opens with those lines tinted,
marked beside their numbers and scrolled to. If an agent already has that page
open in Chrome through [BrowserBear](https://github.com/mikecann/browserbear)
(say, a sign-in page it wants you to use), the click brings the agent's tab
into your Chrome window rather than opening a second one. A link to a file
without its whole path (`shorts-plan/candidates.md`) is looked for from the
thread's folder, as a path in a reply's code is. Web addresses written in
inline code are links too, still drawn as code: `github.com/owner/repo`
opens as HTTPS. Common web endings count without a scheme; file names
such as `readme.md` stay files. On the phone and the web,
links to files on the Mac only say so. A link that isn't a web page (one
that would open Slack, System Settings or another app) asks before it opens,
naming the app, on the Mac and the phone, and so does a file that isn't a
plain document (a `.command`, an app, a kind Threadbear can't place).
For a kind it can't place, the question names the app it opens in and
offers to always open that kind there ("Always open .tandem in Tandem"),
so your own apps' files stop asking. To take that back, remove the entry
from `trustedOpeners` in `desktop-links.json` in the data folder and restart the app.
Another Mac's program never opens here, and its replies never show
pictures from this Mac.

On a Mac, selected words offer Look Up, Translate, Services and, where the
Mac supports it, Writing Tools. The app menu's Services uses the selection
in the focused text box, or the selected words in a reply. Passwords and
one-time codes stay out of these menus. A Service can replace an unchanged
selection in a text box, and Undo restores it. Replies, picture selections,
multiple cursors and an active input-method composition are copy-only.
Control-Command-D looks up selected words. Writing Tools opens the Mac's
native commands, including Show Writing Tools, Proofread and Rewrite.

On Windows, file links and their hover cards only read local drives. A UNC
share, mapped network drive or device path is refused, including through a
local symbolic link or junction. A local link to another local file works.
WSL's `\\wsl$` and `\\wsl.localhost` paths count as shares too.
Files on ordinary disk and CD/DVD volumes work; third-party virtual drives
such as VeraCrypt and RAM disks are refused until the app can verify them.
An open Terminal shell holds off an automatic update restart on Windows:
the app cannot yet tell whether PowerShell is waiting or running a command.
On Windows, for now, a project's patches only run what you click: their
buttons work, but their hooks, worktree setup and teardown, and the commands
their panels and badges run don't run, even once you've trusted the patch.
A folder's Claude Code hooks and the settings that run commands stay off
too. Patches says so beside each patch it affects, and a new worktree's
thread says when its setup didn't run. A schedule's check doesn't run in a
repo's folder either.

The Browser panel, the globe on a thread's right-hand rail, lists the pages a
thread's agents have open through [BrowserBear](https://github.com/mikecann/browserbear):
in the agents' own browser ([Helium](https://helium.computer), in real windows
that open behind yours without taking the focus) or in one of your Chrome
profiles. Each page has a still of how it looks, what the agent is doing in it,
Show window (which brings its real window to the front), and Select on page
(Add from page). The panel doesn't draw the live page or take clicks for it:
you use the page in its own window. An address typed at the top opens a new
page in the agents' browser.

You don't need Helium. Without it the agents' browser is Google Chrome with a
folder and sign-ins of its own, apart from your profiles, and it needs no
extension. If BrowserBear isn't running, the open pane starts it. If that fails
the pane says so, with Try again. The pane can only start a BrowserBear whose
`scripts/install.sh` has written `browserbear-service`, so an older install
needs it run once more.

Settings > General > Browser has Open pages beside Threadbear, off to begin
with. With it on, Show window and Select on page put the page's window next to
Threadbear's, on whichever side of the screen has more room, and after a pick
(or Escape in the page) Threadbear comes back to the front on the thread you
picked for. Move or resize the page's window and it opens that way from then
on. If there's no room beside Threadbear's window, or it's full screen, the
page's window stays where it is. Only windows in the agents' own browser are
moved, never your own Chrome's.

Select on page pins the current thread, brings that page's real window forward,
and turns on a temporary picker overlay in the page. Hover outlines an element,
click picks it without navigating or submitting forms, scrolling keeps working
including inside nested containers, and Escape cancels cleanly. A small bar at
the top of the page also switches to Region (drag a box to crop what is visible,
including over cross-origin frames or closed controls) or Text (highlight text
and press Return to attach a quote). While you are picking, agents cannot click,
snap, evaluate or navigate that tab, and unclaimed tabs in your own Chrome stay
read-only. The capture lands as a preview in the pinned thread's message box
with the cropped screenshot, title, sanitized address (token parameters
removed), optional quote, collapsed element details, a reminder to check the
image for visible secrets, and Discard. Nothing is sent until you press Send.

When the agent needs you to do something in a page (sign in, approve, pay),
handoff brings that page's window to the front. For a page in the agents'
browser it also opens this thread's Browser panel, with the page marked Your
turn and a Done button, even if the panel was closed; your selected thread
stays yours. The agent waits for Done or for the page to settle after your step.
Normal browsing by the agent doesn't open the panel.

An agent can also use a separate Chrome with `--profile agent`. Its pages show in
the same panel, with its own sign-ins. When that Chrome asks for a passkey or a
permission, its window comes to the middle of your screen and the panel says so,
with Show it to bring it to the front. Existing-profile pages use captured frames,
so movement is less smooth than the separate Chrome's stream.

A link to a folder opens it in the side pane, where it works like a Finder
window. Double-click a folder to go in, or a file to see it there, and the
back arrow takes you back to where you were. Press the space bar for Quick
Look, or use the path along the bottom to go up. ⌘↑ goes up too, and ⌘⇧.
shows hidden files. The button in the bar, or Show as in the right-click menu, swaps the list for
icons, with thumbnails of pictures and videos, and the list sorts by whichever column
you click. It works for folders on your other Macs too, though only this
Mac's files have thumbnails and Quick Look.

On a Mac, Office and iWork documents in the pane show a read-only Quick Look
thumbnail. Videos and sound files (WAV, MP3, M4A, AIFF, CAF, FLAC, AAC) have
playback controls, and PDFs can be read and scrolled.
The thumbnail cannot follow links or run document actions. Open the document
in its own app to work on it. Files from another Mac stay on that machine.
Documents over 256 MB can be opened in their own app instead.

Files, the folder on a thread's right-hand rail, opens the thread's own
folder that way, without a link to it. Click it again to close it.

A folder on this Mac has a search box at the top that looks inside its
files: type, and the list gives way to every line with those words, file
by file, with line numbers. Folders like `.git`, `node_modules` and build
output are left out, and so are big files and anything that isn't text.
Click a line to open the file there with the words picked out, and the
back arrow brings you back to the same results. Clear the box for the
folder again.

A file you open in the side pane stays open until you close it with the
back arrow. Going to Highlights or another panel and back, or to another
thread and back, finds it where you left it, scrolled to the same place.

Code and plain text in the side pane open ready to type in, with line
numbers down the side; the check mark in its bar puts the file away to
read, and the pencil brings it back. Nothing is written until you type.
Lines a link named lose their tint as soon as you do. What you type is
saved a moment after you stop, and ⌘S saves it straight away. The
magnifying glass in the bar, or ⌘F in the file, finds words in it,
lighting up every match and counting them, with arrows for the next and
the one before. While you're editing, the same bar has Replace and All,
to change one match or every one. If the file changes on disk while you have unsaved edits (an agent
rewrote it), nothing is written over: a bar asks whether to take the
file's version or keep yours.

A Markdown file opens drawn, as a page to read: headings, lists, tables,
code and pictures, with the lines kept to a comfortable width. In place
of the pencil, a menu in its bar switches between Preview (drawn) and
Markdown (the raw text, ready to type in, saved as above), and the next
Markdown file you open shows the way you last picked.

An HTML file opens in the side pane as a static page, with the styles,
pictures, fonts and media beside it that its markup names. Script files,
SVGs and other pages beside it don't load, so a mockup made of several
HTML or JS files shows only its main page. It's kept off the internet
too, so a font or picture from the web is left out, and its links don't
open. A page with scripts of its own opens with them off and a bar over
it: Run them turns them on for that page, still kept off the internet,
until the file changes, when it asks again. Open the file in your
browser to see it with everything. The GPUI app shows these pages with its
web runtime: until that's installed, the pane asks for it where the page
would be.

To share a file somewhere else, drag it out of the app: a file link in a
reply, a picture in the transcript, a file in Highlights, a row in a folder
in the side pane, or the picture or video showing there. It drops as the file itself, so you can
drag a clip straight into a post on X, or onto the Finder. A web link drags
out as its address.
To paste a picture instead, right-click it in the side pane and choose
Copy Image. The same menu has Show in Finder and Copy Path, as a file
link's right-click menu in a reply does.

## Full access or Ask before edits
<!-- slug: full-access -->

<!-- picture: /docs-img/full-access.webp alt: Ask before edits: the agent waits for Allow, Always allow or Deny before it runs a command -->

Read this before your first thread. Every thread runs in one of four modes,
shown on the composer (the box you type in) and changed by clicking it.

- **Full access**: the agent edits files and runs commands
  without asking. It can do anything your Mac user account can do, in any
  folder, including deleting files. This is how Threadbear is meant to be
  used day to day, and it's fine for a folder you trust it with.
- **Ask before edits**: the agent stops for your OK before it edits a file
  or runs a command. A card shows in the thread with Allow, Always allow and
  Deny. Always allow only lasts for that thread. Return allows the card
  showing ⏎, from an empty message box or after a click in the thread;
  with several cards waiting that's the oldest, and Return again takes the
  next. A Codex thread in this mode
  can read files but not change them unless you agree. It also asks before
  it does anything to Threadbear itself: messaging or starting another
  thread, changing this one, schedules, projects, pins, panels and the
  Pocket. Reading threads, asking you for a secret and ending its turn
  never ask. When a Claude agent sends several things that ask at once
  (say four edits), they come on one card, each with a tick and a cross:
  Allow lets through the ticked ones and refuses the rest, and Deny all
  refuses them all. The iPhone shows the first and asks about the rest one
  at a time.
- **Auto** (Claude threads only): a separate safety check approves or blocks
  each action, and a card shows only when it can't decide. The card says
  why it couldn't, in the check's own words, and an action it stops
  outright leaves a note in the thread saying why. It's for work on
  text you didn't write, like an error report or a web page. It catches a lot
  but it's no guarantee, and on a Codex thread it behaves like Ask before edits.
  Before a card reaches you in an Auto thread (the safety check unsure, an edit
  to a file Threadbear runs as code, a message to a thread with more access,
  a project's brief), a reviewer looks at it first: Opus, with no tools,
  shown the card in full and what the thread was asked to do. If it fits the
  task and can't do real harm, it goes ahead and the thread says "Allowed by
  the reviewer" with its reason. If the reviewer has doubts, or can't run (no
  pool, a private folder, a card or brief too long to read whole), the card
  comes to you after a note with its reason. It treats everything it reads
  as data, and text trying to tell it what to answer is a reason to hand you
  the card. Cards that hand out access lasting past the one action
  (switching a thread to more access, starting a thread with more, a
  schedule, moving a thread or changing its logins) always come to you, and
  so do Codex threads' own tool cards.
- **Plan first**: the agent looks around and writes a plan, but changes
  nothing yet. The plan shows as a card with Go ahead and Send back. Go
  ahead (or Return in an empty message box) lets it start, back in the mode
  the thread had before (Ask before edits for a thread that started in Plan
  first). Send back, or just writing what you'd change, has
  it plan again. Once it's working, the same card ticks off its steps.
  On the iPhone app the plan shows as a card in the thread: tap it to read
  the whole plan, copy it, or share it as a Markdown file (Save to Files,
  Notes, AirDrop). Typing what should change sends it back. Go ahead is
  on the Mac.

In Ask before edits and Auto, an edit to a file Threadbear runs by itself
(a patch, a file a patch uses, or Claude Code's settings) always asks, with
no Always allow, and the card says Threadbear runs the file as code. If a
patch runs something Threadbear can't follow, every edit asks, and the card
says it can't tell whether the file runs as code. The thread then says which
patch it is, the command and why, and so does that patch in Patches. Usually
it's a command built from something only known as it runs, like
`node ${script}`: write it as plain text, or from constants in the patch
(`node ${join(import.meta.dirname, 'x.mjs')}`), and edits go back to normal.
URLs, patterns and prompt text in a patch don't count, only what it runs. Even
once you allow it, the patch waits for you to trust it in Patches.

The mode belongs to one thread. On the Mac a new thread starts on the
mode you last picked for one, as it does the model (Plan first aside, which
is for one task), so if you're unsure about a task, switch it to Ask before
edits before you send. Until you've picked one it's your default mode. On a
new install the default is Auto; an install from before 9 Nov 2026 keeps
Full access. Threads you start from the phone, the website, `tb new` or a
schedule with no mode of its own start on the default. It's `"defaults": { "permissionMode": "auto" }` in `runner.json`
in the data folder (`"full"`, `"ask"` or `"auto"`; none is Full access), so
ask any thread to change it. It applies once the runner next restarts. The
threads Threadbear starts by itself, to bring in Mike's releases or fix a
failed update, start on it too, so on Ask before edits (or Auto on Codex,
which asks before edits) they wait for you to allow what they need. Deleting
a thread is one thing an agent always has to ask about, whatever the mode.

An agent in Ask before edits or Auto can't give itself more room, with its tools or with `tb`: `tb` in a thread signs in with that thread's own key and gets what the thread can do, and anything more is refused with a pointer to the tool that can ask you on a card. Switching
any thread or schedule to more access, starting a thread with more access
than its own (the threads it starts get its mode), messaging or moving a
thread that has more, changing the brief of a project such a thread is in,
lowering a thread that has more access, adding a schedule check, or deleting,
lowering, moving to another login, model or time zone, or taking the check off a
schedule that has one or whose runs have more access all show a card for that one call, with
no Always allow. The card shows the whole message or brief, and the
folder where it really is. On a Codex thread, Always allow on a command allows that
exact command again in that folder, not anything that starts the same way,
and it's only offered for simple commands like `ls` or `grep`: one that
runs the repo's own code (`npm test`, `make`, `cargo run`) asks every
time, as the repo can change what it runs.

What such a thread wrote for another thread to read later stays at the
access it was allowed at. If you raise the other thread before it arrives
(a report from a thread it started, a reply from a thread it started on
another Mac, a schedule it set up for that thread), its agent doesn't get
it: the thread comes to Needs you with a note saying what wasn't passed on
and where to read it, and a repeating schedule pauses until you resume it
(resuming it, or Run now, is your OK).

A card shows the whole command. On the phone, one that's too long to read
there in full, one with anything but plain ASCII in it (an accent, an
emoji, an invisible or text-reversing character), one asking for more
access, or any file edit (the phone shows only its path) has only Deny:
allow it in the Threadbear app on the machine the thread runs on (the
card names it), which shows every line an edit writes and writes an
invisible character out by name (like `⟨U+2028⟩`). On a machine with no
Threadbear app of its own (none has connected in two weeks), such a card
couldn't be allowed anywhere, so it isn't put up: the agent is told to
do it another way, and a note in the thread says so.

A thread on another machine shows its real access in the composer
(Auto, Ask before edits and so on), or "Access unknown" while that
machine runs a Threadbear from before 18 Nov 2026. It can only be
changed in the app on that machine.

### When an agent needs a password or a key

An agent that needs a secret (your Mac's administrator password for a
`sudo` command, an API key, a token) asks for it in a card in the thread
instead of in its reply. Type it into the card and click Save securely, or
Decline. You can answer it on your phone too, where it's sealed so only your
Mac can read it on the way through. The phone only sends secrets to Macs
it learned about from a pairing code, so if it says it doesn't know this
Mac yet, answer on the Mac, or pair the phone again from the code in that
Mac's Settings > Phone & browser.

It never goes into the chat: the agent gets a private file holding it,
which its commands can read for 24 hours, and then it's deleted. That's
not the same as hiding it from the agent. A command that prints the file
would put it in the transcript, so only give a secret to a thread you'd
trust with it.

If a key does end up in a thread's words, Threadbear takes out what it can
recognise before the text reaches your phone or the website, which show
`[REDACTED]` there; the Mac keeps every word. It goes by the shapes keys
usually have and the values in your `.env` files, so it won't catch
everything, and it can't read pictures. Treat a key that showed up in a
thread as leaked and replace it, since the AI provider has seen it too.
`tb status` says what this runner does.

## Your first thread
<!-- slug: first-thread -->

<!-- picture: /clips/decisions.mp4 alt: A thread ends with what it needs from you, and one tap answers each -->

Press ⌘N, or click New thread (the pencil beside the search at the top of
the list), type a message and press Return.

New threads start in the `ThreadbearThreads` folder in your home folder
(`~/ThreadbearThreads`), which setup makes for you. You can pick another folder for
one thread from the composer's + menu, or change the default in Settings
(⌘,).

The agent starts while you write that first message, so its first reply
comes about half a second sooner. If you change the folder, login, model,
effort or mode before sending, it starts again to match, and if you don't send
within ten minutes it stops. This is for Claude threads in a folder as it is;
one that gets a worktree, or a thread started in a project, starts when you send.

A safe first message is one that only reads, such as "What's in this
folder?" or "Explain what you can do for me here." When you're ready for it
to make something, try "Make a file called hello.txt that says hi" and watch
it appear in `~/ThreadbearThreads`.

Return sends and Shift-Return (or Option-Return) adds a new line. In a list, Shift-Return
starts the next item ("2. " after "1. milk", "- " after "- milk"), and
Shift-Return on an empty item ends the list. You can paste or drop images
into the composer, or add them with + (⇧⌘A in the new desktop app). Each one goes in where the cursor
is as a small chip, "Picture 1", "Picture 2", with
its thumbnail, which drops into a strip over your words as the box opens
for it, and the agent gets each picture right there in your words.
In the new app, + > Attach images or Markdown… also takes `.md` files: each
goes in as its path, as a dropped file does, and the agent reads it from
there.
Let go of a file over the box's words and it goes in right there. You can
also let go of it anywhere else over the thread: the thread's outlined while
it'll take it, and the file goes in where the cursor is. A picture or file
dragged out of the app itself (a picture in the transcript, a Highlights
file) only goes in when you let go over the box, so a drag out that changes
its mind doesn't land in your message.
So "this one [Picture 1] should look like this one [Picture 2]" says which
is which. In the new app, thumbnails wrap to the box's width; after three
rows, their strip scrolls so the words stay in view.
The thumbnails show over the text too; the cross on one takes it
and its chip out. Hover a chip to see its picture, in the composer or once
it's sent. Click a chip or a thumbnail, in the composer or in the thread,
to open the picture, with the arrows to step through the others.
On the Mac, screenshots dragged from the floating thumbnail, pictures dragged
from a browser, HEIC pastes and photos copied from Photos work here too.
Threadbear keeps image data and temporary screenshots before the source can
take them back. It opens and copies ordinary files in the background, showing
a loading chip while it waits. Keep those files in place until their
pictures finish loading.
A picture being copied from another app or fetched from a web URL shows a dim
chip while it arrives. Send spins until the picture is ready. If a running
thread's draft contains only a loading picture, Stop stays available instead.
The Stop keyboard shortcut still works. A web URL
is fetched by Threadbear without the browser's cookies or sign-in, using public
internet addresses only. For a picture on a local or private-network server,
copy the picture itself.
Any other file you paste or drop in (a video, a PDF, a folder) goes in as
its full path where you drop it, for the agent to open where it is.
Nothing's copied, so a big one is fine. On the Mac app it shows as a chip
with the file's name and a small dot, which means it isn't uploaded: only
an agent on that Mac can open it. Click the chip to open the file.

Shift-Tab in the message box turns **Plan first** on and off. Plan first
is the access menu's Plan first: the agent reads and plans, changes
nothing, and puts its plan to you on a card with Go ahead and Send back.
Threadbear also saves each plan in the thread's folder as
`plans/<date>-<project>.md` (the folder's name if the thread isn't in a
project), opens it in the panel beside the thread, and pins it in
Highlights; a revised plan updates the same file until you say Go ahead.
Turning Plan first off puts back the access the thread had, or Ask before
edits if Threadbear can't tell what that was (after a restart). Settings
(⌘,) > General can give Shift-Tab back to the message box.

Your messages show basic Markdown as replies do: start lines with "- " or
"1. " for a list, and use **bold**, *italics*, `code`, [links](https://example.com),
"# " headings and "> " quotes. Your line breaks are kept as you typed them,
and a message with none of that in it shows exactly as you wrote it.

Paste a thread's id (Copy thread id in its ... menu or its right-click
menu in the sidebar), or drag its row from the sidebar onto the open
thread, and it goes in as a chip with that thread's title. Dragging a row
left still archives it; any other way drags the thread. The agent gets a `thread:<id>` link it can
read the thread from, and clicking the chip once it's sent opens that
thread. An id that isn't one of your threads pastes as plain text.

Or type `#` at the start of a word and a list of your other threads opens
above the message box, the latest first. What you type after the `#`
narrows it by title or folder, or by the start of a thread's id, and
archived threads show once you've typed something, after the rest. Up and
Down move, Return, Tab or a click puts the thread in as its chip, and Esc
closes the list. A `#` that matches no thread (`issue #12`, a Markdown
heading) leaves it shut.

When an agent's reply links another of your threads (one it started, say, or
one it's waiting on), a small copy of that thread's tile sits just before the
link and shows how it's getting on, changing as it does: a turning ring while
it works, an orange dot when it needs you, a red one if it failed, and a tick
once it's done. Rest the pointer on it for the words. The Mac app shows these;
the phone and the website show the link alone.
Stop (⌘.) ends a turn that's running.
If you stop before the agent starts reading your message, Threadbear saves
it and its pictures and leaves a note under that turn's Worked for line.
They go with your next message, including after you switch models or restart
Threadbear. Stop on its own doesn't retry them. Clearing the conversation
discards them. Moving the thread to another runner keeps the original messages
in its history, but you need to send the stopped request again there.

The caret stays visible after ten seconds without editing or moving it,
and starts blinking again when you continue typing or refocus the box.

On a Mac the composer checks your spelling as the Mac's own text boxes do. A
misspelled word gets a red wavy line once you've finished typing it. Words
aren't changed for you: Correct Spelling Automatically is off to start with,
whatever your Mac's Keyboard settings say (right-click the box, then Spelling
and Grammar, to turn it on). With it on, the Mac's correction goes in as you
start the next word, as your text replacements do either way (those start
as your Mac's Keyboard settings have them); ⌘Z puts back what you typed. A
correction you
take back (⌘Z straight after it, or deleting into the word and typing it
your way again) isn't made again while the app runs, and the word stops
being marked. "Threadbear" is never marked or corrected. Right-click a marked
word for the Mac's guesses, Ignore Spelling and Learn Spelling. The same
menu has Spelling and Grammar (a green line under grammar, off until you
turn it on), Substitutions (smart quotes and dashes, off to start with, and
text replacement), Transformations (upper case, lower case or capitalized,
for the selection or the word the cursor's in) and Speech, which reads the
selection, or the whole message, aloud. Edit > Paste and Match Style (⌥⇧⌘V)
pastes only the words, leaving out any picture or file that was copied with
them. Windows and Linux have no spell checker here.

A message shows in the thread as soon as you send it, faded, with a
spinner beside it until the runner has it (a busy Mac can hold the runner
up for a few seconds). A clock instead means it's waiting for the turn
that's running to finish, and it starts its own turn then. If it can't be
sent, it goes and its text is back in the composer. Starting a new thread
keeps your message in the composer, greyed, with a spinner in place of
Send, until the thread exists.

### Going back to an earlier message

Hover over one of your own messages and click the pencil to change it. Its
text goes into the composer; send it and the thread goes back to that
point. The agent forgets that message and everything after it, the
changes it made to files since then are put back, and your new version is
sent as if the rest had never happened. A turn that's running is stopped
first. What was cut is gone from the thread, and a line where it was says
how many files were put back.

Going back stops anything the agent left running in the background (an
upload, a long build), even if it started it before the message you're
changing. When something is, the app asks first and says what: Cancel
keeps it running, and you can send your change as a new message instead.

Files changed by commands the agent ran (a script, `git`, `npm install`)
aren't put back, only ones it edited or wrote itself. If you've changed one
of its files yourself since, that file may be left as it is, and the line
says so. A message you sent while the agent was working went into that
turn, so edit the message that started it instead.

### Slash commands

A message that starts with a command, like `/compact`, runs it rather
than going to the agent as words. In a Claude thread Claude Code runs its
own commands and your skills (`/compact`, `/context`, `/cost`, `/clear`,
`/code-review`). A Codex thread runs `/compact`, `/review` (the
uncommitted changes, or what you write after it), `/clear` or `/new`,
`/status`, and `/` with the name of one of your Codex skills.

Two are Threadbear's own, on both: `/model` and `/effort` change the
thread's model and effort level, the same as the menus under the message
box, so the change sticks. On their own they say what the thread is on and
what you can pick. `/clear` starts the agent on a fresh session; the
transcript above stays for you to read.

Type `/` in a thread's message box, a new thread's included, and a
palette opens above it with every command the thread can run: Threadbear's, the CLI's and your
skills, with Recent for the ones you picked lately. What you type narrows
it, and the one picked is described on the right: how to use it, its
other names, what `/model` and `/effort` are set to now, and where it
comes from (yours, this folder's, a plugin's) with its file and an Open
button that shows the file beside the thread.
Up and Down move, Left and Right step through the categories (All,
Threadbear, the CLI's, Skills, Recent), Return or Tab puts it in the box
(Return again sends it, so a new thread never starts by surprise), and
Esc closes the palette. The Run button sends it at once; to have Return
do that too, turn on Settings > General > Slash commands > Return sends
a command right away. While the command list is still loading, Return leaves
your words in the box. Clicking a category shows only those;
double-clicking a command does what Return does.
Click the search field along the palette's top to find a skill or
command by any word in its name or description. The arrows, Return and
Esc work there too, the X at its right closes the palette, and clicking back in the message box carries on
typing there.
Drag the palette's top edge or sides to make it bigger or smaller (the
border lights up where it will drag), as wide as the window, and the
lines between its columns to size them. It keeps the sizes for next
time. Right-click it for its default size or column widths, or
double-click a line to put that column back.

Type `/` partway through a message ("then use /td") and the palette
lists the skills the agent can run when it reads one named there. Return
or Tab puts the skill in your words, and nothing is sent until you send
the message. Claude Code's own commands, and skills set to run only when
you type them (`disable-model-invocation: true`), work only at the start
of a message, so they aren't listed there. If you'd rather have the
small list above the message box for these, turn off Use full palette for
skills inside a message in Settings > General > Slash commands. Only one
of the two shows.

The iPhone app has it too: type `/` in a thread's reply box and a list of
its commands opens above it, with the categories as buttons along the top.
Tap one to put it in the box, then send it (add details first if it takes
any). It comes from the thread's Mac, so with that Mac asleep it says so,
and a command typed in full still runs.

A command waits for a running turn to finish rather than going into it.
Commands that only make sense in a CLI's terminal (`/resume`, `/rewind`,
`/login`) get a line saying how to do that here. Anything else starting
with a slash, such as a file path, goes to the agent as words.

## Hibernate and archive
<!-- slug: hibernate-and-archive -->

<!-- picture: /docs-img/archive.webp alt: An archived thread, still there to read, search and pick up again -->

To put a thread away for a while, hibernate it: hover its row and press
the moon, or use the Thread menu or the ... menu. Pick when it should
come back: in an hour, this evening, tomorrow, next Monday, or a day on a calendar and a
time of your own. It leaves the list, its agent stops, and it comes back marked unread
when the time comes. In the GPUI app a bear's thread also has Until needed, at the top: no
time at all, so it sleeps until something sends it a message (you, a
schedule or another thread), and its row says Until needed. Its row says Woke up until you open it, and the
thread says how long it was away, beside a bear that's only just got up. Hibernating threads wait in the Hibernating section at
the foot of the list, soonest to wake first. Hibernate the thread you have
open and its page sinks into the moon on that heading as the next thread
comes in.

Or pick Wake when… and say it in your own words: "at the weekend", "in a
few weeks", "next time I talk about woodworking". Opus works out what you
meant (the row says Thinking… for a few seconds) and the thread says what
it settled on. A topic waits for you to bring it up: when the librarian
sees you talking about it in another thread, it wakes this one and says
where. Give it a limit as well ("or Saturday at the latest") and, if you
haven't brought it up by then, it comes back saying so.

Whenever a thread is hibernated, woken, archived or brought back, a grey
line in it says who did it (you, an agent or Threadbear), when, and why
if there's more to say: "Hibernated by you on Tue 24 Nov, 3:32 pm, until
Wed 25 Nov, 9:00 am." A thread that's done with a check-in booked moves
under Hibernating by itself, with no snooze, and its line says so and
names the check-in; it goes back to the Inbox, saying so, if the check-in
is deleted or paused. These lines are on the Mac only.

When a thread is done, archive it: press ⌘E, hover its row and press the
box, or swipe the row left with two fingers on the trackpad (right with
natural scrolling off), or drag it left with the mouse, and let go once the
space behind it turns blue (a quick flick works too). However it's archived (⌘E, a button, or an agent
or your phone doing it), its row slides off the same way. Archive the
thread you have open and its page drops into the Archived heading as the
next thread comes in. If it was the last one in the Inbox (archived or
hibernated), confetti pops from the empty Inbox. When you archive one, a note saying so floats at
the top of the thread panel for a few seconds (longer while the pointer's
on it): Undo puts the thread back, and a click anywhere else on it opens
the thread. Its x, or a sideways two-finger swipe on it, puts the note
away early. In the desktop app on a Mac, ⌘Z brings back the last thread you
archived, then the one before, up to seven, even after the note has
gone. It works outside a text box or in an empty message box; while
there's typing in the box to undo (words, or ones you've just cleared),
⌘Z undoes that first. The Archived section at the foot of the list has
them, most recently archived first, with the older ones behind Show more
(the search at the top finds any of them). Unarchive, or the same swipe
there, puts one back.
To get rid of one for good, hover it there and click the trash, or
right-click it and choose Delete…; after you confirm, it goes to the
runner's trash folder, with no Undo in the app. Another Mac's thread can
only be deleted on that Mac, so there the trash is greyed out and its tip
names the Mac.

A thread another thread's agent started archives itself when it finishes
with nothing left for you: nothing asked, no follow-ups suggested, nothing
uncommitted, unlanded or unmerged, no background job running and no
check-in booked. If threads it started itself are still going, it waits for
them first. A note in the thread says why it went, and it shows as archived
under "Threads it started" in the Highlights of the thread that started it.
It stays in the inbox if you've written in it, starred it or brought it back
out of the archive, and so does one that failed or was stopped. Threads you
started, and ones a tool or a schedule started, don't archive this way,
unless you've turned on the experiment Quiet inbox for your threads.

With that on (and Inbox guess, which it builds on), one of your own threads
that finishes with nothing for you is archived too when a model judges you
didn't need to read the reply, with a note in the thread giving its reason.
Not one with follow-ups, anything left open or a design review waiting for
you, one you've starred, one you
had open as it finished, or one you've brought back out of the archive
before. To check what it hid, a daily thread lists the ones it archived
the day before, each with the model's reason: tell it which you should have
seen, and it brings them back and writes the misses down. Or right-click an
archived thread and pick Should Have Shown Me, which brings it straight
back to your inbox and tells the model it got that one wrong.

Back and Forward (⌘[ and ⌘], in the Go menu, or the ‹ and › beside the share
icon at the top right of a thread) step through the threads and
pages you've had open, as a browser does. Hover either arrow to see which
thread it goes to. With Settings, Skills or Notes open over the thread, Back
puts that page away first. A mouse's side buttons do the same,
and so does a trackpad swipe, as in Safari: with natural scrolling right for
Back and left for Forward, and the other way round without it. A card in the
middle of the window says which thread or page you'll land on, and fills the
way your fingers go, turning blue once letting go will go. With nowhere to go
that way, it says so. A swipe that starts mostly up or down is a scroll, so a long thread still
scrolls. It follows Swipe between pages in System Settings > Trackpad (two or
three fingers). Over a thread's row a swipe left still archives it. A thread
you start takes the place of the new-thread screen, so Back from it goes to
wherever you were before you pressed New thread.

Hibernating or archiving a thread that's partway through a turn stops
that turn. It also stops any background jobs that thread's agent left running,
including shell jobs waiting their turn in a queue. From the row's hover buttons,
Threadbear asks first. A scheduled message can bring the agent back later,
but it does not keep those jobs alive. To let a job finish, leave its thread
awake; the agent can end its reply while the job carries on. An agent that
only needs to wait for something (a merge, CI, another thread) is stopped
from doing it with a background job, and uses a scheduled check instead,
so its thread can still hibernate.

You can also ask a thread to do it to itself ("hibernate this until
Thursday", "archive this when you're done"). It finishes its reply first,
then goes, with a note in the thread saying when it'll be back. A time
works, or words like "until the phone arrives", read the same way as Wake
when…. When the words are something to happen ("until the GPUI app
replaces the Swift app"), the librarian wakes it once another thread shows
it has, not just when it's talked about, and the thread gets a turn to
check for itself: if it hasn't really happened, it goes back to sleep.
While it checks it isn't marked unread and nothing notifies you or your
phone, so a wrong guess passes unnoticed. It comes to your inbox, unread,
only if it doesn't go back to sleep: the thing has happened and it has
carried on, or its agent needs you.
Such a wait often has a latest time too ("or Sunday 9:30 am at the
latest"). When that comes with no sign of the thing, the thread doesn't
land in your inbox: it gets the same turn to check, says in a line what's
still missing and sleeps again, for at least twice as long as last time (a
week at most). After three of those checks in a row without a word from
you, the next latest time brings it to your inbox with a note saying what
it's waiting for, how long it has waited and that it's yours to decide:
chase it, give up, or tell it to keep waiting. Its agent is told why it
woke, so you can ask it. It won't put itself away while it's waiting on you. A thread
whose next step waits on other threads hibernates by itself until they
finish, then wakes with what they said and carries on; its row says
Waiting on and which. If it was given more access while it slept, and
one of them has less, it wakes for you instead, with a note, and what
they said isn't passed on until you send a message. It can also say what it's waiting for a thread to
do ("until it reports the I/O saving"), and then the librarian reads
that thread after each of its turns and wakes it once it has. On the
iPhone and the website a hibernating row starts "Until …" when it waits
for more than a time. A thread can also hibernate until the runner
updates to a new release (a restart on the same one leaves it asleep).
One that has landed a change to the app and is waiting to check it
hibernates by itself when it's done for now, and wakes once the app has
restarted onto a build with the change. One waiting on a job that will message it does the
same until the message comes. Either way it has a time it comes back
anyway, a day unless it says otherwise.

## Panels
<!-- slug: panels -->

<!-- picture: /docs-img/panels.webp alt: A panel beside the thread: this one lists a project's previews -->

The window is made of panels you can move around: the list of threads,
the thread you're reading, Highlights, a project's panels, Patches, and
the ones you open from the left-hand rail, such as Usage and Updates. Each
panel has a bar along its top with its title, and panels that share a
space show as tabs in it. To begin with there are three side by side:
the list, the thread and Highlights. Highlights, Files, Patches and a
project's panels each show their own content wherever they are, so you can
keep Files open beside Highlights; a file opened from one opens over that
one.

To move a panel, press and hold on it for a moment, then drag. That works
on its title, one of its tabs or the empty part of its bar, and on its
background too if you turn on Settings > General > Move panels by their
background (a panel with its title hidden always moves from its
background). On a title or tab a plain drag works too. A quick drag on the empty part of a
bar moves the whole window instead, as there's no title bar, and a
double-click there zooms it. While you drag, the panel shrinks under the
pointer and shows where it will go when you let go:

- Over another panel's bar, a tab with its name shows there: let go to
  add it as a tab.
- Near one side of another panel, that side lights up: let go to split
  that panel's space in half.
- In the middle of another panel, a compass shows. Drop on one of its
  arrows to dock above, below, left or right of it, in the middle to add
  it as a tab, on Swap to change places, or on Window to give it a
  window of its own.
- Along the window's own edge, it docks across that whole side.
- Past the edge of the window, it opens in a window of its own. Right-click
  a panel's title for Open in New Window too, and Put Back in Main Window
  to bring it home.

Escape, or letting go anywhere else, puts it back where it was. Drag the
line between two panels to resize them, and click × on a tab to close it.
Right-click a title for Hide Title, which leaves the panel without a bar
while it's on its own; a thin strip along its top still lets you drag it.

The icons down the right edge of the window open and close the thread's
own panels: Highlights at the top, one for each panel its patches add,
and Patches at the foot. To open a thread in a panel of its own, beside
the one you're reading, right-click it in the list and pick Open in New
Panel. The × at the end of its title bar closes it, and Move to Main
Panel, on the same right-click menu, brings it back to the main panel.

A patch's panel follows the thread you pick, and steps aside for a thread
that doesn't have it. To keep one in view while you work in another
thread (one project's Convex dashboard while you're in another project's
thread, say), right-click its tab and choose Pin. It stays where it is,
with a pin on its tab, showing the thread it was pinned from whichever
thread you pick; hover over the tab to see which. That thread's button on
the rail brings it to the front. Unpin, on the same menu, puts the
ordinary panel back. Pinned panels are in the new desktop app only.

The View menu has the rest: Show or Hide Threads (⌃⌘S, or ⌘B in the new desktop app) for the list,
Collapse or Expand Threads (⌘/), which folds the list away while the bar
of icons on the left stays, gives its room to the thread, and brings it
back at the same width (clicking Threads, Scheduled, Projects or
Librarian in that bar brings it back too), Collapse or Expand Sidebar
(⌥⇧⌘B, the new desktop app only), which folds the whole left sidebar away,
the list and any of Runners, Connectors, Updates, Usage or Setup under it,
and brings it all back the same way (any icon in the bar brings it back too),
Show or Hide Sidebar Panels (⇧⌘B, the new desktop app only), which closes
whichever of Runners, Connectors, Updates, Usage or Setup are open and keeps
the list, then brings the same ones back,
Show or Hide Panels (⌥⌘B, ⌥⌘P in the Swift app) for the panels beside the thread, Show or Hide
Terminal (⌘J), and Reset Panel Layout, which puts everything back how it
started. Threadbear remembers your layout between launches.

### Terminal
<!-- slug: terminal -->

⌘J (or ⌘`) opens a terminal under the thread, and the same key again hides it. It's a real
one, with your login shell, its colours and full-screen programs like vim
or htop, and it takes the keyboard as it opens. Each thread has a shell
of its own, started in the thread's folder (its worktree, when it has
one), and it keeps running while you look at other threads, so going back
finds it as you left it. A thread on another Mac or the cloud gets a shell
on this Mac in your home folder, with a line saying so, and so do an
archived thread (its worktree may be another thread's by now) and the
new-thread page. Click + above it for another shell in the same folder,
so a dev server can keep running in one while you work in the next; each
shows as a tab, and its ✕ closes it and ends its shell. `exit` closes
that tab, and in the last one New Shell starts another. Double-click a
tab, or right-click it and choose Rename…, to give it a name of your own
(like Dev server); an empty name goes back to the shell's own. A shell
only starts when you ask for one, with ⌘J, a click on the empty panel or
a command's play icon (below), so looking through threads with it open starts nothing. A thread's shell
stops when the thread is archived or deleted, and they all stop when you
quit Threadbear. The Terminal is a panel like the others: drag it
beside the thread, into a tab, or out into a window of its own.

It has Terminal.app's keys, except where Threadbear already uses one:

| Key | What it does |
|---|---|
| ⌘⌫ | Deletes back to the start of the line |
| ⌘← / ⌘→ | Start and end of the line (Home and End in vim and other full-screen programs) |
| ⌥-click | Moves the cursor there on the line you're typing |
| ⌥-drag | Selects a rectangle |
| ⌘L | Clears everything above the prompt, scrollback too (⌘K is Open Anything) |
| ⌥⌘K | Clears only the scrollback |
| ⌥⌘R | Resets the terminal after a program leaves it in a mess |
| ⌘F, ⌘G, ⇧⌘G | Find, then the match above or below. Return and Shift-Return do the same in the Find field, Escape closes it |
| ⌘↑ / ⌘↓ | Jumps to the command before or after, each line you pressed Return on |
| ⌘Home / ⌘End | Top or bottom of the scrollback |
| ⌘T | Another shell in a new tab |
| ⌃Tab / ⌃⇧Tab | The next or previous tab |
| ⌃⌘V | Pastes the clipboard escaped for the shell, as one word |

Right-click (or Control-click) for a menu with the same: Open Link, Copy,
Paste, Paste Escaped Text, Find, the clears and New Shell. Drop files on
the Terminal to type their paths, escaped. When a program rings the bell,
the Terminal flashes, and a tab you aren't looking at gets a dot until you
go to it. Closing a tab with a program still running in it (a dev server,
vim) asks first. When you quit, each tab's last 1,000 lines are kept, and
the next time you open that thread's Terminal its tabs come back with that
text above a new prompt, once, within 14 days.

⌘-click a web address, a file path or a labelled link printed in the
Terminal to open it (Ctrl-click on Windows and Linux). Web addresses underline under the pointer, with a tip showing how to open them. Programs and links
that start another app ask first, as they do elsewhere in Threadbear.

A shell command an agent gives you (a ```` ```bash ````, `sh`, `zsh` or
`shell` block) has a play icon and a copy icon at its top right. Play
opens the Terminal with the command typed at the prompt and the keyboard
there, and you press Return, so you see it before it runs. Pressing play
on another block while one is still waiting copies it instead, so run or
clear the first. If something is already running in that shell (or the
shell has become another program, as `exec ssh` does), or you've typed on
its prompt, nothing gets added: the command goes on the clipboard and a
bubble says so (a shell without bracketed paste, like macOS's old bash,
always copies). Pasting lines (or a Tab) into a shell like that asks
first, as each line would run as soon as it arrived. A paste leaves out control
characters (anything but Tab and line breaks), so copied text with a
hidden one in it can't run itself. Play isn't there on a thread
on another Mac or the cloud, or an archived one, as this Mac's shell
wouldn't be in the right place; Copy still is. Nor is it on a thread open
in a panel of its own, as the Terminal shows the main panel's thread's
shell: open the thread in the main panel to use it. It's on the Mac app only.
A block with control or invisible characters in it has no play icon.

Pointing at a web address in the Terminal, like a dev server's
`http://localhost:5173/`, underlines it. A link in the Terminal (one a
program prints, or a path you ⌘-click) opens the way a link in a reply does: a web page in your browser, a document in
its app, and anything that would run something, or start another app,
only once you've said yes.

## Files and Terminal

**Files** on the rail down the right, below the folder's own panels
(such as Worktrees), lists the thread's folder, to go into folders and
open files in the panel (a thread on another Mac lists that Mac's
folder). Turn on Settings > General > Terminal for a **Terminal** button
beside it, which opens the Terminal under the thread as ⌘J does (see
Terminal above), for threads on this Mac. It's off unless you turn it on.

Clicking a file in a reply, Highlights or Files opens it here in the
panel: code with syntax colours, Markdown drawn, and Edit for text. To
have files open in the app your Mac uses for them instead (Cursor, Xcode),
set Open files in to Its own app in Settings (⌘,) > General. ⌘-click
always opens a file in its app, and its right-click menu has both.

Text files open ready to type in, as in an editor: double-click one in
Files (or click it in a reply or Highlights) and it's there with line
numbers down the side and a bar underneath showing the line and column.
What you type is saved by itself about a second after you stop, and when
you leave the file, the panel or the app; while it's waiting, the file's
tab has a dot and the bar says Edited, then a quiet Saved when it lands.
⌘S saves at once. If an agent changes the file while you have changes
that aren't saved, Threadbear never writes over it: a bar asks whether to
take the file's version or keep yours. The eye at the top shows the file
as it reads and the pencil goes back to editing.

Markdown files open drawn, as a page to read: headings, lists, tables,
code and pictures, with the lines kept to a comfortable width. The menu
at the top switches between Preview (drawn) and Markdown (the raw text,
ready to type in, saved as above), and the next Markdown file you open
shows the way you last picked.

Two switches in Settings (⌘,) > General > Files change this, both on
until you turn them off. Turn off Save files automatically and nothing is
written until you press ⌘S or Save (handy when a dev server reloads on
every change); then going Back, opening another file or thread, or
closing the panel with changes asks Save, Don't Save or Cancel. Turn off
Edit files like an editor and files open to read, and the pencil switches
one to editing, saved as you go.

## Git

A thread working in a git repo gets a Git panel on the rail down the
right, like VS Code's Source Control. It's an experiment for now: turn on
Git panel in Settings > General > Experiments. The panel is for the repo
the thread's folder is in, or, for a thread in a folder that isn't a
repo, the first repo among its projects' folders.

Along the top are the repo, the branch and how far it is from its remote,
with Fetch, Sync and Push. Below them, write a message and press the one
button, which changes with what's next: Commit when something's staged,
Sync with the arrows counting what's to push and pull, or Publish branch
for a branch the remote doesn't have yet. Its chevron has Commit,
Commit & Push and Commit & Sync. ⌘↩ commits, ⇧⌘↩ commits and pushes. ✨ in
the message box has Claude Haiku write the message from what's staged, in
the style of the repo's recent commits (not in a private folder).

The lists fold: Staged, Changes and Commits. Point at a file for + to
stage it or − to unstage it, or at a list's heading to do them all. A dot
marks a commit that isn't pushed yet, and a pushed one opens on the web.
The panel's button counts what's waiting on you: staged files plus
commits to push.

Two chips above the message box do the same through the agent: Commit
staged & push and Sync. They turn up only when there's something to do,
and the agent writes the message, deals with a rejected push and says
what it shipped. Saying "commit and push" in your own words does the
same.

Nothing here force-pushes or skips the repo's hooks, and Push and Sync
only push to a remote you own: one under your GitHub login (the one `gh`
is signed in as), or an owner you list in `runner.json`. `"*"` allows any.

```json
"gitPanel": { "pushOwners": ["you", "your-org"] }
```

`"chips"` in the same place says when the chips show: `"when-needed"`
(the default), `"always"` or `"off"`. A repo whose own git settings would
run something when it's read (a filter or a diff driver) is left alone,
and the panel says so: ask the agent instead.

## Highlights
<!-- slug: highlights -->

<!-- picture: /docs-img/highlights.webp alt: Highlights: what the thread pinned, the files it wrote, its links and its changes -->

Highlights is the panel beside the thread that collects what the thread
has made and shared, so you can find it without scrolling back through
the conversation. It fills in by itself as the agent works. Its icon is
the pin at the top of the right-hand edge, which gets a dot while the
last reply has things for you to do.

From the top, and only when there's something to show:

- A line or two from the librarian on where the thread is up to.
- **Needs you**: what the agent's last reply asked you to do, each with a
  box to tick off as you go. Ticking one here ticks it in the thread too.
- **Task list**: with the Clearer thread completion experiment on, the
  tasks the agent reported for your request, with how many are completed.
  What's still pending or blocked comes first; the completed ones fold
  under a row like "21 completed" that opens them. It stays after you
  reply and after the thread is archived, and grows as you add requests.
  The agent keeps it, so you can't tick these yourself. It changes when a
  turn ends, so during a long turn it shows the list from the turn before.
  A list over 30 tasks drops its oldest completed ones to make room.
- The thread's projects, each under its icon and name with its links,
  such as a live site or an admin page, and its task board if it has one.
  When an agent deploys a Convex-hosted site (`https://<name>.convex.app`)
  for a thread in a project, Threadbear adds it to the project as Live site
  by itself
  (see [Projects](#projects)). The ⓘ beside a project's name
  says who put the thread there and why ("Shortlists photos for the
  calendar entry"). If the librarian guessed wrong, press Take out there
  and it won't put it back; Keep stops it ever taking the thread out.
  The librarian also renames a project whose name has gone out of date
  (a tool it's about was renamed, say), but never one you named or
  renamed yourself.
- **Scheduled**: messages booked to come to this thread. Click one to
  change it.
- **Sub-agents**: helpers the agent started, with how each is getting on.
  Click one to read what it did.
- **Threads it started**: other threads its agent started, and any it's
  waiting on, each with where it's up to: the ones that need you first,
  then failed, working, hibernating, done and archived. Click one to open
  it. Their questions still come to your inbox like any thread's.
- **Pinned**: what the agent pinned for you (below).
- **Files**: documents, pages, pictures and videos the thread wrote, opened
  or mentioned, each with a small preview: the picture itself, a frame of
  a video or a PDF's first page, or a document's first lines. Dotfiles,
  hidden folders in your home folder (`~/.convex`, `~/.ssh`) and files
  named like they hold a key show their icon instead, as the previews are
  on screen whenever Highlights is. Another Mac's files show their icon.
- **Links**: web pages and pull requests it opened or mentioned.
- **Changes**: its commits, and a line such as "4 files edited" that
  opens to the code it changed.

Click a heading to fold that section away, and again to bring it back. A
folded heading shows how many rows it's hiding. Threadbear remembers what
you've folded, between launches and for every thread: fold Sub-agents once
and it stays folded everywhere. A project's heading folds for that project.

Click a file and it opens in the panel, with Back to return to the list
(code and text open there ready to type in). A link opens in your browser,
unless you change Default in its right-click menu to open pages in the
panel. A Tandem project opens in Tandem, here and in a reply. Right-click any of them for more,
such as Reveal in Finder, or Remove to take it off the list. You can drag
files out of Highlights into other apps.

### Pull requests

A GitHub pull request link clicked in a reply, or in Highlights' Links,
opens as a tab in the panel instead of your browser (⌘-click still goes to
the browser). In the desktop app only, and it needs the `gh` command line
tool installed and signed in (`gh auth login`); it acts on GitHub as
whoever that is. A PR link in a reply has GitHub's pull request mark before
it: green while it's open, grey as a draft, purple once merged and red if
closed.

The tab has the PR's title, who opened it and when, its branch with lines
added and removed, its status (approved, changes asked for, conflicts,
merged, closed) and its checks, naming any that failed. Below that are the
description, comments and reviews, and a box to comment on GitHub. Diff
shows every changed file, each folding open or shut; Checks lists each
check with its time, and clicking one opens its log.

Review approves it or asks for changes, with what you typed in the comment
box. Merge merges it the way its menu's Default says (rebase, squash or a
merge commit, whichever the repo allows). Close without merging is at the
bottom. Every button spins until GitHub has answered, then the PR is read
again. Start a thread puts the PR in a new thread's message box in the same
folder; Ask Claude to review starts a thread there that reads the PR and
says whether to merge it, without touching it on GitHub.

PRs load ahead so they're usually ready when you click: when you rest the
pointer on a link, when you open a thread whose last few messages link to
PRs, and when a turn ends with one. A PR you've opened before shows
straight away as it was last time while it's read again, with its buttons
waiting until GitHub has answered. The app keeps the last 200 PRs it read
in its cache folder for this. Each PR you open stays as a tab at the top of
the panel until you close it, and Back goes to Highlights.

### Pins

An agent can pin a file or link to Highlights, under Pinned, with its
`pin` tool and a short note on why you should look at it: a mockup it
made for you to check, a preview address, a pull request. It can also
open the pin in the panel beside the thread: straight away if you're
looking at that thread, or else when you next open it. Files it
writes and links it mentions show up by themselves, so pins are for the
few things that matter most. To have something pinned, ask the thread
("pin the mockup for me").

## Projects
<!-- slug: projects -->

When a project update needs your approval, its details show the current
project name and ID alongside the proposed changes.

<!-- picture: /docs-img/projects.webp alt: The Projects view: each project with its threads, what it's for and where it's up to -->

A project gathers the threads about one thing, whatever folder they ran
in: an app you're building, a trip, a house move. Each has goals, a few
lines on where it's up to, notes for later threads (decisions and
gotchas), links (its live site, an admin page) and folders, the repos its
work lives in. The Projects view lists them, with New project at the top;
click one for its page, with its threads, state and notes. The search at
the top of the Projects page looks through their names, state, goals, notes
and folders: Return opens the top result and Escape clears it. Add folder…
under Folders on a project's page picks one, and its New Folder button
makes one first if the repo you want isn't there yet. Every folder picker
in the app has that button.

With Customize the Projects page on (Settings › General › Look, off by
default), the page gets a Customize button at its top right, like the
Librarian and Scheduled pages'. The page has no bear, so its picture is
one of your own or none, which it starts as. Its panel also puts a photo
behind the page, with Position and Fade; moves the picture, the title and
search, and the cards to the top, middle or bottom, or hides them; and
picks what each card shows: the icon tile, the open thread count, the
status and where it's up to. With the tile off, the count sits by the
project's name. Hiding the title and search shows every project, so a
search left typed in can't hide any. Turning the switch off puts the page
back as it ships and keeps your changes; Reset to the standard page
clears them.

To start a thread in a project, press New Thread at the top of its page
(or New thread… by its Threads). The message box opens with the project's
folder picked and an "In" chip naming the project (its x starts the thread
outside it instead). The thread is in the project from its first message,
so its agent starts with the project's goals, state and notes; it shows
in your Inbox like any other, and under the project's Threads.

A thread also gets into a project when the librarian files it (below),
when its agent joins one, or when you put it there, from Projects in its
right-click menu. A thread can be in more than one. Its agent is told a
little about each project it's in, and gets the panels and Patches from
the projects' folders. If one is wrong, Take out in its Highlights keeps
it out for good.

A project can be a part of a bigger one, for one body of work with
separate strands: an app, its desktop port and its Android version, each
with its own goals, state and notes. Right-click a project in Projects (or
use the ... menu on its page) and pick Make a Part of, then the project it
belongs to; Make Its Own Project undoes it. A part sits under its parent
in the Projects view and inside its card on the Projects page, and the
parent's page lists its parts. A thread in a part is told the parent's
goals as well as the part's own; a thread in the parent is told which
parts there are. One level only: a part can't have parts. Agents can do
the same with update_project. The phone and the website list parts as
projects of their own for now.

A project can show a task board from Notion. Give it a Notion page with a
Tasks database on it (a board grouped by Status): Add Notion page… under
Tasks on its page, Notion Page… when you right-click it in Projects, or ask
an agent to. The page shows the board, read every few minutes, with a
column per status and who and when on each card. Cards are moved in Notion,
by you, anyone you share the page with, or the agents, who see the open
cards and are told to keep them moving. A card whose Thread column names a
thread links to it. A thread's Highlights shows each of its projects' boards too, under the
project: what's under way first, then what's to do, with Done folded to a
strip you can click open. If the columns don't fit, drag the bar under them,
use its arrows, or scroll sideways, or drag the panel wider. Click a card to
mention it in your message: it shows as a chip with the card's title, and the
agent gets a link it can read the card from. Mentioning the same card again
keeps one copy in the draft. Each column starts with four cards; More and
Show fewer change how many are shown. Click the chip in a sent message to
open the card; the arrow on a card, or its right-click menu, opens it in
Notion. The menu also offers Copy Link and, when another thread is working
on the card, Open the Thread Working on It. Boards are read through
a Connectors gateway (see [Connectors](#connectors)); without one, the
project's page says so and nothing is read.

## Prints

A Print is something a thread makes for you to keep. It can be a chart, a dashboard, a report or a doc. It stays after the reply that made it scrolls away, and asking again updates it rather than making another one. It's an experiment: turn it on in Settings > General > Experiments > Prints. (Prints were called Pages until 10 Nov 2026; an older reply's page still shows as a card.)

Ask in plain words, in any thread:

| Say | You get |
|---|---|
| "Make a pie chart of this spreadsheet by region" | A **chart** print. Attach the file or name it. CSV, TSV, JSON and Excel (.xlsx) work |
| "Make a dashboard from this data" | A **dashboard**: big numbers, a few charts and a table |
| "Give me a report on where this project stands" | A **report**: sections of text with charts |
| "Write up the launch plan as a print" | A **doc**. You can edit it yourself too |
| "Draw this flow as a diagram print" | A **diagram** from Mermaid source, with text you can edit and an SVG you can download |
| "Make a todo list for the launch" | A **todo list** you tick by clicking, with how many are done |
| "Put these tasks on a kanban board" | A **board**: columns of cards you move with each card's arrows |
| "Run bundle-analyzer summary" | A **bundle size** report for a Convex app |
| "Make a Convex insights report" | A **dashboard** of the app's insights from the last 72 hours: write conflicts and functions near their read limits, by function, with each one's table and when it was last seen. Add "for prod" for production |
| "Map my Convex API" | A **dashboard** of every function and HTTP route the app's deployment runs: public or internal, the arguments each takes, and any missing validators, listed first |
| "Add a figure that explains it" or "make a share clip" | A larger extra: an illustrated figure, or a short video. Only when you ask |

**Where a print goes.** A thread pinned to exactly one project puts its new prints there automatically, under **Prints** on the project's page. Say "keep it out of projects" for none, or "put it in Poddle" for another. A print can be in several projects without duplicating its files. All Prints are stored in `~/Library/Application Support/Threadbear/pages/<print-id>/v<version>/`, separate from your code folders. With no project, reopen its card in the thread or ask "show my prints with no project". Download saves a separate copy wherever you choose. Nothing about a print goes to your Convex backend unless you publish it (below).

**Only when you ask.** Threads also make Prints on their own, like a progress Print on a long task. To keep Prints but have them made only when you ask, turn on **Only make Prints when I ask** in Settings > General > Prints. A thread that goes to make one your message didn't ask for then asks you first, on a card, and No means none is made. A message that says print, chart, graph, dashboard or diagram counts as asking, so it goes ahead without a card. Changing a Print that already exists never asks.

**Progress prints for long tasks.** On by default: a thread on a task with more than five steps, or one that should take over half an hour, keeps a progress Print of where it's up to. Turn it off in Settings > General > Prints and threads skip it, without asking you about it, and start the work straight away. Other Prints, like a chart you ask for, are still made. With the Clearer thread completion experiment on, the thread's [Task list](#highlights) in Highlights keeps track of the work instead.

**In a reply**, a thread shows a print as a card: a picture of the top of it, then its title, kind, version and age, with Open, Refresh and a ⋯ menu. Click the picture or Open to open it in the side pane beside the thread. While it's open there, a new version replaces it in the pane as soon as it's saved. If you opened an older version on purpose, it stays put. A thread on another Mac opens it in a window of its own instead. On the website, the same reply shows a card too. A published print's card has its title and an Open button that goes to its page. One that isn't published says it's kept on your Mac, since its files never leave it. The iPhone app shows the block as text for now. The card always shows the print as it is now, so when a print gets a new version, the card in an earlier reply moves to it too, marked "New version", rather than a second card appearing. While a refresh is under way, the card says so.

Print previews and Git side panels follow the app’s light or dark appearance. Changing appearance updates an open panel without reloading it.

**Light and dark.** Every print holds both looks in one page, so a thread never makes it twice: near-black with quiet lines in dark, warm paper with white cards in light, big numbers first and one colour per chart. On your Mac it follows the app's appearance. On the web, a published print follows your device's light or dark setting, and the switch at the top right of its page (**System**, **Light**, **Dark**) picks one instead; the browser remembers it, as it does the website's own appearance in its settings menu. That's why a print could look light on the web while dark in the app: the app has its own appearance, and the browser goes by the Mac's or the phone's.

**In Highlights and on the rail.** A thread's Highlights list the Prints it made, under Prints, each with its kind, version and the projects it's in, or "No project". The Prints button on the right-hand rail, under Files, lists every Print you have, newest first, with the same details. It shows once there's at least one. Click a Print in either to open it beside the thread. Right-click one for everything a project's Print menu has: Show in Finder, Edit Text, Download, Versions, Publish, Delete and the rest. A deleted Print waits 7 days under **Recently deleted** on its project's page, where **Restore** brings it back.

**Several at once.** In the Prints sidebar, search by title, press **Select**, then click the Prints you want. Selection circles and a count show what will change. **All** (or ⌘A while the list has focus) selects the visible results; typing in Search clears the previous selection. ⌘-click and Shift-click also work. **Done** or Esc ends selection. **Delete** asks once, naming the selected Prints; Return deletes and Esc cancels. Any deletion the runner refuses comes back with an error.

**Archive Prints.** **Archive** hides the selected Prints from this Mac's active sidebar. Their files, project memberships and published links stay intact. Choose **Archived**, select items and press **Restore** to bring them back. Search works in both views. Archive is local to this Mac; Delete is the existing seven-day trash operation.

In Highlights or a project's Prints list, ⌘-click and Shift-click keep their existing selection behavior. Their selection bar and right-click menu offer **Projects** and **Delete…**.

**Links in a Print** open in your browser when you click them, beside the thread and in a Print's own window. The Print stays where it was. Only web addresses (http and https) go out, one for each click.

**Every row under Prints on the project's page** opens its print in a window of its own when you click it. Its ⋯ menu, or a right-click, has:

- **Show in Finder** shows the print's newest `index.html` in its folder, to copy, attach or look through its files.
- **Refresh** sends the print to the thread that made it, to be made again the same way, and opens that thread so you can watch.
- **Edit with Agent…** asks for what should change, for example "make the region chart a pie". The thread does it and saves a new version.
- **Edit Text…** opens a doc's Markdown or a diagram's Mermaid source to edit yourself. Save redraws it as a new version marked "edited by you". Invalid diagram source shows an error and keeps the previous version.
- **Download…** saves the print's files in a new folder. Its `index.html` opens in any browser.
- **Download as PDF…** saves it as a PDF on US Letter pages, in the light look, so it prints and sends cleanly. A line that lands right on a page break can be cut across two pages. For print-perfect breaks, ask the thread to "download it as a PDF", which prints it through Chrome.
- **Versions** lists the last 10. Open one, download it, or make it the newest again; nothing in between is lost.
- **Projects** puts it in a project or takes it out.
- **Publish…** puts the print on your own Convex backend, after asking, and copies its link. It then opens on your phone, on the website, and for anyone you give the link to, and each new version goes up by itself. **Copy Link**, **Open on the Web** and **Unpublish** replace it once it's published. Unpublish, or Delete, takes it down and the link stops working. Only the print's one page goes up, and only prints with no scripts can be published.
- **Delete** takes it away. It waits 7 days under **Recently deleted** at the bottom of the list, where **Restore** brings it back.

You can also ask a thread for any of these: "refresh the sales dashboard", "go back to version 3", "delete the old chart", "bring it back", "download it", "what prints do I have".

**Keeping one up to date.** Say "refresh it every weekday morning" (or any time you like) and the thread schedules itself to make the print again from its recipe and save a new version. It tells you when it will run. It shows on the Scheduled page, where you can change or stop it, or ask the thread to.

**Asking once.** In Ask before edits, the first change to a print asks you. Once you allow it, later versions of that same print from that thread go ahead without asking again, until the thread is next restarted. A different print, or deleting one, still asks.

**Todo lists and boards** are Prints you change with a click. In a todo list, click an item to tick it or untick it. On a board, each card has arrows to move it to the column before or after. Each click saves a new version, so Versions takes back one you didn't mean. Edit Text opens the list as Markdown (`- [ ] item` for a todo, a `##` heading per column for a board) to add, rename or reorder things. Clicks work in the side pane and a Print's window on your Mac; published or downloaded, it's a plain list.

**Diagrams** are drawn on your Mac with Pele, the same library used for diagrams in replies, and kept as static SVG with their Mermaid source. No package download or network access is needed to render one. A wide diagram uses the whole window and scrolls sideways rather than shrinking its labels too small to read; printed or saved as a PDF, it fits the page. Download includes `source.mmd` and `diagram.svg`. Published diagram HTML includes the diagram and its folded source, so both become public when you publish. Editable CSV sheets are deferred.

**Outside agents** can list and read published Prints through the existing Threadbear MCP connector when their key allows **Can read all my threads and published Prints**. A key limited to its own threads sees no Prints. Only published copies are shared; nothing is read from your Mac's local Prints store. `list_prints` returns a page of metadata; `read_print` takes its slug and reads the HTML in bounded pieces. They cannot create, edit, publish or delete Prints. Unpublish removes access on the next read. Reconnect the outside app if it caches the old tool list.

Prints are static: pictures, charts and text, with no scripts, so they open straight away and can't reach the internet. A thread can make a "live" print with scripts when you ask for one. Its scripts run only after you choose Run them, and again for each new version.

### Tables

An experiment (Tables, which needs Bears). A table is a group of bears
around one goal, with you at the head. It's a project marked as a table:
its goals are the table's goal and its notes are its decisions.
`tb tables new "Launch crew" --bears "Researcher,Writer" --parent Launch`
makes one inside the Launch project, with a folder of its own in
`~/ThreadbearThreads/Tables`. A thread there is the table chat: its host
passes work to the bears, and their replies come back into it, so you
read every exchange. Each message you send allows at most 6 bear replies
(`tb tables seat <table> --max 4` changes it). The librarian keeps a
table's state but never files a thread into one.

To seat another bear, put one of its threads in the table's project (the
thread's Projects menu) or run `tb tables seat <table> --add Designer`.
The table's project page lists the bears at it, with their icons, under
Bears at this table, and new tables get a three-people icon. A project
with tables among its Parts lists each table's bears on its own Project
tab too, under Bears at and the table's name.

## The librarian
<!-- slug: librarian -->

<!-- picture: /clips/librarian.mp4 alt: The librarian files each thread into its project and keeps the notes -->

The librarian reads your threads as they go and keeps your projects tidy.
It files each thread in a project, making one when none fits, keeps each
project's state up to date, adds a repo to a project's folders once its
threads work there, and renames threads that have wandered off from their
title (never one you named). It also looks after the Pocket cards threads
filed. Its picture, and the one on the Scheduled page, say something if you
poke them.

It looks after a thread's first message and when a turn ends, then catches
up at most every 20 minutes while the thread keeps going. A catch-up after
only a quick reply is skipped without asking anything.

Each look is one short call to Haiku, Claude's small, cheap model, rather
than the model your threads run on. When it wants to file a thread in a
project the thread hasn't touched the folders of, one more short call, to
Opus, checks the thread is really working on that project and doesn't just
share a subject with it. They run on one of your own Claude
logins: the one the pool would pick next in the pool chosen under Runs on (the
same pool that writes thread titles, also in Settings › Models), and they
count towards that login's usage like any other call. When every login in
the pool is out of room it waits, and with only Codex logins it doesn't
run at all. Threads in a private folder (Settings › Models) are never sent
to it.

The Librarian view lists what it did and what it will look at next. Click a
project name in its log to open that project's page. Hover
a row to see why it filed a thread where it did. Turn
off Look at threads there to pause it; what's queued waits until you turn
it back on. If it files a thread wrongly, take the thread out of the
project and it won't put it back.

The GPUI app keeps page looks in its desktop settings on this Mac.
With Customize the Librarian page on (Settings › General › Look, off by
default), the page gets a Customize button at its top right. It opens a
panel beside the page that changes it as you go: keep the bear, use a
picture of your own or have none, and pick its size; put a photo behind the
page, drag it in Position to choose which part shows (it starts with its
bottom edge on the window's), and fade it so the text stays readable in
light and dark; move each
part (a greeting, the picture, the title and status, a quote, the numbers,
Threadbear activity, GitHub activity and Settings) to the top, middle or
bottom, or hide it; hide the
title, the line under it or the status line on their own; and pick which
numbers show. Greet me puts "Good morning, Wayne" at the top (afternoon
from 12, evening from 5 pm, night from 10 pm, by your Mac's clock), with
today's date and the time under it if you like. It suggests your Mac
account's first name; change it, or clear it for just "Good morning".
Quote of the day adds a quote from a writer, scientist, engineer, artist
or athlete, with who said it under it and where it's from when you hover
it. It's the same all day and changes at 4 am by your Mac's clock; the
refresh button beside it shows another for the rest of the day, and
right-click copies it. The quotes ship with Threadbear, so nothing is
fetched, and none are from politicians, religious figures or tech
founders. Put it at the top, middle or bottom under Layout like any other
part. Drop a picture from Finder on its spot in the panel, or choose one.
Thread activity (off until you turn it on under Layout) adds the
Threadbear activity card your profile page can show: a year of days shaded
by how many threads you started each day, worked out on your Mac from your
threads, with threads in a private folder left out. Hover a day for its
count and step back through the years with the arrows. It sits with the
numbers until you move it, and where you put it is where it goes on your
profile page too.
GitHub activity shows a year of your public contributions, as on your
GitHub profile, once you type your user name: step through the years with
the arrows, hover a day for its count, and click one to open that day on
GitHub. It's fetched from the GitHub Contributions API
(github-contributions-api.jogruber.de), sending only the user name. Turning
the switch off puts the page back as it ships and keeps your changes for
next time; Reset to the standard page clears them.

The GPUI desktop app has the same Look switches, the same Customize panels
for the Librarian, Scheduled and Projects pages, and the same background
photo. It
picks up what you set in the Swift app the first time it opens, and the two
share the pictures. You can also drop a photo from Finder on the
background's Picture row in Settings. Two things differ there for now: a
photo has no frosted fill behind the cards on it (they get a near-solid one
instead), and a picture has to be a PNG, JPEG, GIF or WebP.

The bottom of the Customize panel makes you a public profile page on your
own Threadbear site, at `https://<your deployment>.convex.site/profile`.
Your site is yours alone, so there's no user name to pick, and your Convex
backend serves the page, so it needs one (Your phone and a browser). It
saves as you change it, like the rest of Customize. Give it a name, a line
about you and up to four links, then turn on each part you want on it: the Librarian page's picture, Threadbear
activity (a year of days shaded by the threads you started), GitHub
activity, the numbers you pick (threads this year, active days, longest
streak, projects, models used), the projects you tick (title and thread
count only), the month you started, and a Changelog: your public commits
to your Threadbear repo on GitHub, newest first and grouped by day, in a
section at the bottom of the page that opens when clicked. It lists the
commits of your GitHub user name (under GitHub activity) on
`<that name>/threadbear` unless you type another repository, and the
visitor's browser reads them from GitHub, so only public commits ever
show. Every part starts off. The cards go
in the order the Librarian page has its numbers, Threadbear activity and GitHub activity. Until
Publish my profile page is on, only you can see it, in a browser signed in
to your Threadbear site, with a line saying so. Published, Show it on my
site's front page makes it what visitors see at your site's address; when
you're signed in you still get your threads there, and Sign in at the
bottom of the page says how. Customize shows the page's addresses with Copy and Open
for each: Profile page (your site's /profile) always opens the profile,
signed in or not, and Front page, there while it's on the front page,
opens your site's address, where visitors see the profile and you, signed
in, see your threads. The page looks like Threadbear's own site. A link
to a published page shows your name, line and picture when pasted into
Slack, X or iMessage. Published, the Librarian page gets a View profile
page button beside Customize; the View profile page button switch under
Profile page takes it away. Its numbers and activity refresh by themselves within
the hour. Thread titles, prompts, replies, folders and project notes are
never sent, and threads in a private folder aren't counted. Take the page
down deletes it and its pictures from your backend.

## Schedules
<!-- slug: schedules -->

<!-- picture: /docs-img/schedules.webp alt: The Scheduled view: what runs next, what's paused, and what's done -->

A schedule sends a message at a set time, once or over and over: to a
thread, or to a new thread each time (a morning summary, say). Make one in
the Scheduled view (⌘2), or ask an agent ("look at this again every weekday
at 9"). To send what's in the message box later, click the arrow beside
Send, or right-click Send: **Send tonight at 9 pm…** (until 9 pm), **Send
tomorrow at 9 am…** and **Send when tokens reset…** each open the schedule
editor on Once with that time filled in, ready to check and save, and
**Schedule…** opens it for any other time or a repeat. When tokens reset is
a minute after the thread's login has room again (or, on a pool, the first
of its logins of that kind), or with room left, after the next reset of a
limit it has used. With old readings, or none saying when, it's greyed out.
A schedule sends only words, so pictures in the box stay there.

The Scheduled view lists them under Active, Paused and Done, with
when each runs next. One that starts a new thread each time keeps the
pool the composer was on when you made it, so each run starts on
whichever of that pool's logins it would pick then; ask an agent to put
it on another pool or one login. Its model and effort start as the
composer's too, and you can change them in the schedule's editor (click it
in the list). In a pool with logins of several kinds, each run starts on
the kind you picked its model under. Click a heading to fold it away (Done starts folded); a
folded one shows how many are under it. Its page
says what's coming up next with a few numbers: how many are on, due in the
next day, repeating, set by threads, paused, and how often they've run.

With Tasks on (an experiment), any task with a due date that isn't done
also shows in the Scheduled view, under **Tasks due**, between Active and
Paused. Soonest first: each row has the task's title and its day (red once
it has passed, amber on the day), then its project, its status and an
Urgent or High priority. A task with only a date sends nothing; one with a
due time reminds you (see Tasks, Notifications). It's there so what you've
promised sits beside what's booked. Click a row to open the task in
Tasks, with the Scheduled list still beside it, so you can click through
them one after another. Folded, the heading counts the ones due today or
overdue, and the page's status line says how many there are ("1 task
overdue, 2 due today"). The page itself lists them too, under **Upcoming tasks** below the status
line: the five soonest, each opening its task, then "N more in Tasks" when
there are more. The page's numbers add three tiles too: Tasks due,
Tasks due today and Tasks overdue. With Customize the Scheduled page on,
each can be hidden like the others. Mark a task done or clear its date and it leaves
the list.

In the Mac app’s conversation, a scheduled follow-up starts with its title,
time and **Show details**. Its instructions stay collapsed until you open
them; **Hide details** closes them again. Expanded details use left-aligned
Markdown and can be selected and copied. This only changes the display: the
agent still receives the complete scheduled message.

With Customize the Scheduled page on (Settings › General › Look, off by
default), the page gets a Customize button at its top right, like the
Librarian page's. Its panel keeps the bear, uses a picture of your own or
has none, and sets its size; puts a photo behind the page, with Position
and Fade as on the Librarian page; moves the picture, the title and status,
and the numbers to the top, middle or bottom, or hides them; and picks
which numbers show. On this page the places are where a part sits in the
window: middle is the middle of the space left, and bottom is the
window's bottom edge. A page too tall for the window stacks them in order
and scrolls. Turning the switch off puts the page back as it ships
and keeps your changes; Reset to the standard page clears them.
With Convex sync on, schedules need a connection to your backend. If the
first connection fails after a restart, the runner tries again after a
second, then waits longer between attempts, up to a minute. Try saving
again once the connection is back.

A schedule's message brings an archived thread out of the archive while it
runs. If the run ends with nothing for you, the thread goes back into the
archive, so a watcher you've put away only lands in your inbox when it has
something you need to act on.

A reminder due while an agent is working waits until that turn ends. A
newer run from the same schedule replaces its older queued reminder, so
repeated check-ins don't pile up. Your messages and other threads' messages
can still reach the agent while the reminder waits.
Repeated Run now requests also replace a pending reminder from that schedule.
Runner notifications also wait behind your messages and other threads' messages.
If the agent answers a later message by archiving or hibernating the thread,
older queued reminders wait with it. They don't start another turn immediately.
New inputs arriving after the agent receives that later message still need handling.

Pausing, deleting or editing a schedule doesn't take back a reminder already
queued. Ordinary messages go ahead of reminders. Reminders carrying check
output are kept separately so new items found by a check aren't lost.

If an update leaves a thread on the old runner while a command finishes,
its scheduled messages can still start turns there. An old runner from before
this feature was added may have to finish moving the thread before its next
scheduled message arrives.

A schedule can have a check: a shell command that runs first, in its
folder, each time it's due. If the check prints nothing, that run is
skipped and no agent starts, so a schedule that looks for new Sentry
issues every five minutes only uses a login when there's something to look
at. What it prints is added to the message. If the check itself fails, the
run goes ahead with the error, so you hear about it. A check has a minute
to finish, or up to five if its schedule says so, for one that reads
something slow like Slack. What a check prints is kept until a run
carrying it has been sent, so a runner restart in between doesn't lose it.
A runner update keeps the check in its thread's folder; during handover,
it waits until that folder is known.
Ask a thread to "only run when there's something new" and it writes the
check. Run now skips it. It's bound to what it runs: if a script it
reaches changes, or you move the schedule to another folder, it doesn't
run until the same check is passed to the schedule again (ask a thread to
pass it), and the thread is told once. Changing a schedule's words, time or
folder in the app never approves its check.

If you run Threadbear on more than one Mac (or on a Linux server as well,
INSTALL.md section 6), a schedule that starts a new
thread can run on any of them: pick the Mac under Runs on. A new thread
can too: Runs on under the box lists the Macs, and its Folder lists that
Mac's folders, with Other… to type a full path there. It runs there
even while this one is asleep, in that Mac's folder (leave the folder
empty for its default). Its threads show on your phone and the website, and
threads made on another Mac show in your sidebar too, with that Mac's
icon in the row. Open one and this Mac reads it from your Convex backend
and sends what you do there (messages, approvals, Stop, archive) for that
Mac to pick up, the way your phone does, so there's no network link between
the two Macs to set up. Pictures you paste into a message, or into a new
thread there, go with it (up to 6, 10 MB each), held in Convex only until
that Mac has fetched them. You see its messages, with the pictures pasted
into them, and a line for each tool it used; the tool output, diffs and
tools' pictures stay on that Mac for now, and a tool row says so. A pasted
picture shows a moment after its message, once that Mac has sent it up
(not at all with `"sync": { "pastedPictures": false }` there, or for one
over 10 MB). Its login and model change from here. Editing a
message, its highlights and patches, and changing its access or effort
are done on that Mac: the access chip names the Mac instead of opening a
menu, and the model picker leaves out the effort levels. Agents see them the
same way: `list_threads` lists and searches them (each row says which Mac),
`read_thread` reads them, `send_message` messages them, and they can stop,
snooze or archive them. A Mac that's off shows
its threads but can't take a message until it's back. Its archived
threads are in your Archived list with the rest, to reopen or unarchive.
To add a Mac, give it a key of its own from this one: `tb runner
add <name> --ssh <that Mac>` writes it there, and its runner uses it from
its next restart. Set that Mac up with `./threadbear setup --existing`, which
leaves out the Convex login and deploy key (either could do anything to your
backend) and ends by telling you to run that command, with that Mac's
`<user>@<name>.local`. It needs Remote Login on there (System Settings >
General > Sharing; setup says if it's off), and ssh from this Mac that asks
nothing: trust its host key once by hand (setup says how to check its
fingerprint) and `ssh-copy-id` it if it wants a password. If ssh fails,
`tb runner add` says which of those it was. `tb runner check` there says if
an older setup or an earlier `npx convex login` left a deploy key or Convex
login behind. Revoke and remove them, then rotate the main key unless that
Mac has only just joined, never had the main key and has run no threads yet.
That key only reaches its own Mac's threads, schedules and Pocket cards: it
can't pair a phone or put a schedule on another Mac, and that Mac's app
doesn't list the other Macs' threads. Only the first Mac's key can. A new
thread from your phone goes to your first Mac, and waits for it while it's
asleep, unless you pick another Mac for it. Notifications from another
Mac's threads start with that Mac's name. After moving a Mac onto its own
key, `tb runner rotate-owner` on your first Mac changes the main key (it
won't while another runner it has seen lately is still on the main key:
it names each one with the `tb runner add` that gives it its own, or
`--lock-out` cuts it off on purpose), and
`tb runner audit` lists the phone tokens, schedules, cards and messages the
old one could have made, to take out any you don't recognise.
An agent on one of those other Macs can still ask for a thread on this
one (to try something only your Mac can, say): a card in its thread shows
who's asking, the folder, its access and the whole brief, and Start it
starts it here, from this Mac. Answer it in the Mac app; the phone shows it
but can't start it. The agent can't see this Mac's folders, so a guess
like `/home/mike/x` or `/Users/mike/x` starts in `x` under your home here
if that's a folder; if no such folder is here, the card says so, and
Start it sends the request back for the agent to ask again with one that
is. The new thread is told which thread asked, and can
message it back. Agents on this Mac start threads on the other Macs
without asking, and each reply such a thread ends a turn with comes back
to the thread that started it as a message (the other Macs can't message
this one's threads themselves).
To start a thread on another Mac, pick it with the machine chip on the composer: the
folder, account and model are then that Mac's. New threads start on the
starred one: change it under Default in that chip's menu, or in
Settings > Runners, which also lists your runners and whether they're
online, in your order: drag one to change it. The
phone has the same order, and a new thread from it starts on the first
runner that's online in it. Adding or renaming a runner is done on its own machine,
so ask an agent to do it for you. Schedules need your
Convex backend: with no internet, they wait.

To set one up, open Settings > Runners and press Add a cloud runner… under
Cloud runners (it shows with the `virtualRunners` experiment on). Pick a box
size, then paste a Fly org token and press Add, or, if flyctl is signed in on
this Mac, press Set up with flyctl's sign-in, which starts it right away.
Without flyctl yet, run `brew install flyctl` and `fly auth login` in Terminal
first; the window lists both under Before you start. Threadbear makes the Fly app, saves the
token and keys to your backend, builds the box image and tries a test box,
showing each step as it goes. Check again and Rebuild are there afterwards.
`tb cloud setup` in a terminal does the same.

With the `virtualRunners` experiment on and a virtual runner set up in your
backend (docs/virtual-runners.md), Runs on also lists it after the Macs,
for example "Cloud (Fly)": the thread gets a machine of its own in the
cloud, made from the folder's git repo on this Mac and your Claude login.
Its row says "Starting a machine" while it boots, "Asleep" once it's been
idle a while, and "Waking" when a message brings it back. The first time a
login runs there, the thread asks for a cloud token in a card on its
message box: Make one runs `claude setup-token` for that login and opens
claude.ai to approve it (not on Windows, where you run it in a terminal
and paste what it prints), Paste a token takes one you made, and Use this
Mac's token appears when that login already has a year-long token here.
The message waits and goes once it's set. If Claude later refuses the
token, the same card comes back.

With more than one runner, Runners (the rack icon near the bottom of the
left-hand rail) opens a panel with each one: whether it's online and for
how long, or when it was last heard from, the commit it runs, and the
threads running there, which open with a click. A grey dot on the icon
means one is offline or updating. Click a runner's name to see its CPU,
memory and disk, what machine it is and how long that machine has been
up. A bar turns orange when memory is 90% used or disk is under 10 GB free.
The figures come every 10 seconds while the details are open and the
window can be seen; a runner too old to send them says so. The runners are
in your order, the same as in Settings > Runners and on the phone: drag one
to change it.

A thread can move to another of your machines whole: the same thread, with
its whole conversation, its pictures and Highlights, and the git work in
its own worktree (commits and uncommitted changes alike). Once you have more
than one machine, the machine a thread is on shows under its message box, by
name; click it and pick **Move to** a machine. You can also right-click the
thread (or use its **...** menu) and pick **Move to**, then the machine. Offline
ones are greyed. While it moves, the machine chip spins and its menu offers
**Cancel Move**. It waits for the turn it's on to end, and its message box
and any open card are locked until it lands or you cancel; a message sent
from your phone meanwhile goes with it. It goes straight away, to the
folder and login that machine guesses. If it can't go, something won't
come (background jobs, changes in a checkout other threads share), or that
machine has no copy of the repo and would clone it (it says from where and
to where), a small window shows the folder and login with why; change
either and press Return to check again (pressing **Move** after a change
checks it too, and shows what it found), then press **Move**. The machine it
goes to picks the folder: its own copy of the repo, a new clone beside one
of its repos, or the same folder in its home, never anywhere else, and
never its home itself, a hidden folder or one tools load from, such as
`~/Library` or a `node_modules`, unless you type it in. A machine whose
Threadbear is from before moves were checked this way says so and takes no
move until it updates. If the
thread or where it would go changes between that look and the move, it
stays where it is and says so: move it again. The row says "Moving to" with a spinner, and the
message box stays locked, until the other machine has it (a moment after it
leaves the first), then the thread is listed under the other machine, in the same
place and still open, and its next message carries on in the same Claude
session there. If anything goes wrong before it lands, it stays where it
was: the bar at the top of the window says where it was going and why, and
the row says "Couldn't move" for a minute. Its schedules come
too, but not a schedule's shell check or the thread's "Always allow"
answers: set those again on the new machine. A thread moved to your main
Mac arrives asking before tools run, even if it had Full access or Auto
there. Agents can do the same with `move_thread` and a `runner`, and
`tb move <thread> --to <machine>` (`--preview` to see where it would go).
Moves are started from your first Mac and need Threadbear's backend.

### Webhooks

An experiment (Webhooks). `tb webhooks new "GitHub" --bear Researcher`
makes a secret URL on your backend, shown once. A POST to it starts a
thread in that bear's folder (or `--cwd` a folder, or `--thread` messages
one thread on Ask or Auto), marked with a bolt and the webhook's name.
Its threads run in Ask, or Auto with `--auto`, never Full access, and the
payload arrives quoted as data, not your words. Deliveries 10 seconds
apart become one run, a repeat `Idempotency-Key` or `X-GitHub-Delivery`
is dropped, and each hook takes 30 calls a minute and 50 a day. Keep the
URL like a password: `tb webhooks revoke <id>` ends it, and turning the
experiment off ends them all. It needs your backend's public
`.convex.site` address, so a local backend can't take webhooks.

## Accounts, pools and Usage
<!-- slug: accounts -->

<!-- picture: /docs-img/usage.webp alt: The Usage panel: how much of each login's limits your threads have used -->

In the GPUI desktop app, the gear or ⌘, opens Settings inside the main
window. Its section list stays beside the form. Back, Esc or ⌘W returns to
the previous view; selecting a thread or another page also leaves Settings.
Your thread and its draft stay where they were, and pending instruction
edits save when you leave. While recording a keyboard shortcut, Esc cancels
the recording first.

Threadbear uses the Claude Code and Codex logins already on your Mac: the
Claude Code login in `~/.claude` and the Codex login in `~/.codex`. It never
reads their passwords or tokens. The account and model a thread runs on are
small menus on the composer. Whatever you picked last is what the next
thread starts on.

Once a thread has run a turn, a small ring beside the send button shows
how full its context is: how much of the model's window the conversation
takes up. It turns orange when it's nearly full, as Claude and Codex
compact the conversation by themselves soon after. Click it for the
tokens used out of the window, the model and login the thread really ran
on, and what it has cost so far. On a Claude thread it also shows what
fills the context, as Claude's `/context` counts it: the conversation,
memory files, skills, tools, the system prompt, MCP servers, the room
Claude keeps for compacting and what's free, with the tool definitions it
loads only when needed listed below. Claude is asked when you open it and
when you click its refresh button. Once a thread has sat idle for half an
hour its Claude has stopped, so it shows the last breakdown and how old
it is until your next message.

A Claude thread keeps its conversation in the provider's prompt cache for
about an hour after its last model call. Wake a long thread after that
(your message the next morning, a schedule, a report from a thread it
started) and the whole conversation has to be read in again, which on a
thread past 200k tokens costs about three times a warm turn. So a Claude
thread with 200k tokens or more of context that has been idle for over an
hour and five minutes compacts first: Claude Code summarises the
conversation, asked to keep what was decided, what you asked for that
isn't done, what's still running or waited on, the files, branches and
threads involved, and how you said you want the work done. Then your
message runs on the summary, in the same turn. It takes a minute or so
before the turn starts, and a quiet line in the thread says it happened
and why. Messages you send meanwhile show a clock until it's done, then go
into the same turn after the first, as they would have without it. A thread
that's still warm, a shorter one, a slash command and Codex threads are
left alone.

To add another login, such as a second Claude subscription, give it a
folder of its own under `accounts` in Threadbear's data folder, named
`claude-1` (then `claude-2` and so on, or `codex-1` for Codex), and sign in
there in Terminal:

```
CLAUDE_CONFIG_DIR=~/Library/Application\ Support/Threadbear/accounts/claude-1 claude
```

then type `/login` (for Codex, `CODEX_HOME=<the folder> codex login`). It
shows up in the account menu within a few seconds. A Codex login's threads
get the same instructions, hooks, skills and settings as `~/.codex`: the
runner links them in before its first thread starts.

To drop a login you no longer use, such as one whose plan has lapsed, add
its id to `hiddenLogins` in `runner.json` (`"hiddenLogins": ["claude-main"]`)
and restart the runner. It goes from the Usage panel and the account menu,
and its sign-in stops putting a dot on the Usage button. Take it out of
any pool first.

If a login stops working (you signed out, or its sign-in ended), the
thread's turn fails and a line above its messages says so, naming the login
and its email ("claude-1 (you@example.com) is signed out."). Sign in… opens Terminal on that login's own sign-in, and Send
again sends your last message again once you're done. A thread in a pool
doesn't wait for you: it moves to another login in the pool and carries on,
and the pool leaves the signed-out login alone until you sign it back in.
A schedule pinned to a signed-out login runs on another login in a pool
that login is in, and its thread says which. If no pool has another login
for it (a login kept apart for work, say), the run fails once with the
sign-in line, and later runs wait, rather than adding a failed thread each
day, until you sign it in or archive that thread.

From Claude Code 2.1.325 a Claude sign-in only lasts 28 days and can't be
renewed by itself. Three days before one ends, its row in the Usage panel
says "Sign-in ends" with the day, the Usage button gets a dot, and its
tooltip names the login; a signed-out login shows "Signed out" there.
Either way the row has Sign in…, so a scheduled job on that login doesn't
fail overnight. Once it's signed back in, confetti pops from its row
(or from the thread's sign-in line) if you're looking at it. Threadbear learns this from each login's own
`claude auth status`. For a thread on
another Mac the line says which Mac to sign in on, as Terminal here would
only sign in this Mac.

To stop signing in every month, a Claude login's row in Usage has
Year-long token…. It opens Terminal on `claude setup-token` for that login:
sign in once, and from then on the login runs on a token that lasts a year.
Threadbear keeps it in a file in that login's own folder
(`threadbear-token.json`, which only you can read) and gives it to that
login's Claude Code and nothing else. The row then says "Year-long token
until" the day, with a burst of confetti if Usage is open, and three days before that it warns as a sign-in does, with
New token…. A token can only run models, so claude.ai's own connectors and
Remote Control don't work on that login; MCP servers you set up yourself
still do, and so do the usage numbers. To go back to signing in, delete
that file.

A login on another of your machines (a second Mac, a Linux box, a Windows PC) gets
one from the Runners panel: open that machine's card, and its Claude logins
are listed under its figures, each with Year-long token…. Your browser opens
on claude.ai's page for it; click Authorize, then Copy code on the page it
ends on, and Paste code back in the panel. That machine makes the token and
keeps it, and it never leaves there. It needs `python3` on that machine,
and your main Mac's app (it goes through your Convex backend).

The sign-in command comes from the runner's provider list, so Cursor opens
`agent login`, for example. Sign in… is offered only when the login's own
details can be read.

A Claude login whose paid plan has run out (it's back on the free plan,
which has no Claude Code) shows faded with "Plan ended" in the Usage panel,
with no numbers, reset or Sign in…, and pools pass it over. Threadbear
learns this from the plan the CLI last saw for that login, which it checks
about once a day while it runs there.
If a runner sends an unreadable thread field or
row, readable threads still appear. An empty or partly unreadable local list
keeps the previous rows and their Terminal shells until a complete list arrives.

If Claude stops with a safeguards refusal, a small box above the message
field offers **Continue with Codex Sol**, when Sol is available on a Codex
login with room. Clicking it moves this thread to GPT-6.1-Sol and asks it
to continue, with the conversation carried across. It pins the thread to
that Codex login. Anything you've typed in the message field stays there.
The switch happens only when you click.

OpenCode, Cursor and Grok, and other agent CLIs that speak the Agent Client
Protocol, get threads of their own too. If one is installed and logged in
it turns up as a login and a pool, with its models in the picker. A pool
with logins of several kinds lists each one's models under its name. OpenCode's
free models work with no login; Cursor needs `agent login` first, and Grok
`grok login`.
Gemini on a Google AI plan (Pro or Ultra) runs through Google's own
Antigravity agent, the way T3 Code does: download `agy_acp_server.par` for
your Mac from the [ACP Registry](https://agentclientprotocol.com) (version
1.4.5 works; 1.5.0 didn't start sessions on a Mac on 20 Nov 2026), set its
path as `antigravityPath` in `runner.json`, and sign it in once with your
Google account. Threadbear can't sign it in for you yet: the login shows up
once `~/.gemini/antigravity-acp/acp_token.json` exists, which signing in
from an ACP client that uses that folder makes. It then shows as a Gemini
login with your plan's models.
Google's FAQ says using an Antigravity login from other software can get the
account suspended, so that risk is yours. Every tool it uses asks Threadbear
first, and reads inside the thread's folder go ahead.
Everything the agent wants to do comes through Threadbear first: on Ask and
Plan first you get the usual cards, Auto behaves like Ask, and Full access
goes ahead without them. A folder's own OpenCode config and plugins aren't
read in these threads. Cursor can't be told to always ask, so its Ask, Auto
and Plan first threads run in its read-only modes and can't change
anything; to let Cursor change files, use Full access. Grok's own settings
(including your `~/.claude/settings.json`, which it reads) can tell it to
go ahead by itself, so Threadbear starts it with a hook of its own that
sends every write, command, web search and MCP tool call to you first,
whatever those say; reading files inside the thread's folder goes ahead,
and reading outside it asks. If Grok didn't load that hook (a folder's
`.grok/config.toml` can turn plugins off), the thread stops before Grok can
do anything. A folder you've trusted in Grok runs its own Grok hooks, which
could act before anything asks you, so there (and in your home folder, which
Grok always trusts) Grok threads need Full access;
take the folder out of Grok's trusted folders (`/hooks-untrust`) to use Ask
or Auto. Denying a card ends Grok's turn, as it does in Grok's own app. Grok's models each list their effort levels (low to extra
high), and the effort you pick is the one it runs at. Small jobs (titles
and summaries) stay on Claude unless you pick a pool of only OpenCode or
Grok logins for them in Settings. Grok logins show their plan's room in
Usage, read from Grok itself (its weekly share and when it resets, a minute
after the runner starts and after Grok turns). When a Grok login runs out
of balance, the thread says so in plain words ("grok-main is out of Grok
usage...") and is treated like a Claude login at its limit: the login
counts as out until its week resets or a recheck finds room again (every
10 minutes, so a top-up is seen), and a pool thread moves to another login
with room in its own pool, or waits for room. OpenCode and Cursor have no
usage readings. None of these have a sandbox or
going back to an earlier message yet, and Threadbear only updates
their CLIs when you press Update in the Updates panel.

Titles, the librarian and Wake when… run on a Claude login, so with only
Codex they don't happen: threads keep the first line of their first message
as their title.

Logins are grouped into pools, one for Claude and one for Codex to begin
with (Settings > Pools). A new thread in a pool starts on the login with room whose
weekly limit resets soonest, so a week's leftover gets spent before it's
lost rather than draining the login with the most room. A login is used
until it's all but out (under 1% left), so none of a week goes to waste. When
the one it's on runs out, before a turn or partway through one, it moves to
another login in the pool with room, and carries on from where it was. A
quiet line in the conversation says so each time: the login it moved to,
the one that ran out, and when that one is back (for a thread on another
of your machines too, and on the phone and the website). When
none has room, the thread waits and carries on by itself once a limit
resets, and the line says when that is. With only one login, that means it waits for that login's limit to
reset. A thread pinned to one login does the same on that login. Either
way, the box at the top of the message field says what happened and offers
Switch to another login now; set its Carry on by itself to No and the
thread waits for your next message instead.
When a Codex login it would carry on with has a usage reset left (Codex
gives out free ones, listed under Usage limit resets in its settings), the
box also offers Use a reset: it restores that login's 5-hour and weekly
limits at once, spending the reset that expires soonest, and the thread
carries on. Threadbear never uses one by itself.
A Codex login that reads as used up is asked for its limits again every
10 minutes (free, no turn), so a reset you use in Codex itself shows up
here within minutes and that login is used again.
Picking a model on another login from the model menu after a limit stop
(a Codex one when Claude is out, say) carries on straight away too, on
the model you picked. When the thread is idle otherwise, it says the new
model takes over from your next message, as nothing runs until then.
The first time a pool has a second login, a card by the composer says
what pools do.

With more than one machine, they all share the same pools, so you set them
up once from Settings > Pools on any of them. A pool knows its logins by
email, and each machine uses the ones it's signed in to: a pool of three
Claude logins is just one on a server signed in to one of them, and a pool
none of its logins are in doesn't show there. Two logins on one machine
signed in to the same account (two config folders, say) are told apart
there by which login you picked, and Settings > Pools shows the login's
note or id beside the email; other machines still match them by email.
`tb accounts` says when two logins share an email, and `tb accounts --json`
gives every login with the room it has left (each limit's percent and reset,
merged from all your machines' readings), for a script to check. A login signed in on several
machines is one subscription, so each machine shows the newest reading of
its room from your first Mac, and one that hit its limit there waits on
the others too. A Mac with a key of its own gets your first Mac's readings of only the
logins you list for it there: `tb runner logins` shows each one's list
beside the logins it says it has, and `tb runner logins <runnerId>
claude:you@example.com` sets it. Your first Mac goes by its own readings,
never that Mac's.

Usage, the honey pot near the bottom of the left-hand rail (it's filled to the
room left on the login your next new thread would use), shows how much room each login
has left before it hits its limits: the weekly one, and the 5-hour one for
logins that still have it (Claude Team does, Max plans don't any more).
Threadbear learns these numbers from the threads it runs, so a login's bars
fill in once it has run something.

At the top it shows what you've spent: tokens today (and what a usual day
had spent by this time), in the last hour and over the last 7 days, with a
chart by the hour or by the day, each login in its own colour. Hover a bar
to see who spent what. The tokens are read from the Claude and Codex
transcripts on your Mac, so work done outside Threadbear counts too; the
first read takes a minute or so after the runner updates.

Below that, each pool has a card with a row for each login in it: a ring
with how much of its week is left, the provider's logo and the login, and
one line saying how it stands: in red, that it's used up and when it's
back; in orange, when it runs out and how long before the reset; in green,
how much of the week will go unused, so it's worth leaning on that login;
otherwise that it's on track, or when it resets. At the right is what it's
spent today.

Click a row to open it. Under it is a small chart of the last seven days,
rolling, with the days along the bottom: last week's readings stay on it
after a reset, and your current pace is dashed on to the next reset. Then
the 5-hour window as a small chip, where the login has one, and the
login's buttons. Hover the ring, the chart or the chip for its rate. In a
week's first half day the pace is the week before's, as a busy first hour
says little; with no week before to go by, the line just says when it
resets. A sign-in that's ending or has ended shows under its row even when it's
shut.

A Gemini login on a Google AI plan shows the plan's 5-hour and weekly
windows for the Gemini models, which Threadbear reads from Gemini's own
`/usage` every 10 minutes and after each Gemini turn (it costs nothing), with
the plan's Claude and GPT models' week as a meter of its own that never makes
the login look full.

A login with no number yet says why on its row, with a dashed ring: a
Claude or Codex login that hasn't run a turn here yet; a Grok login
Threadbear hasn't read yet (it reads one a minute after it starts); Gemini,
before its first reading; or OpenCode,
whose own models spend a balance rather than a weekly limit.

Bought more usage after a login ran out? Press the refresh arrow beside
the login, in Usage or in Settings > Pools. Threadbear asks the login how
much room it has now (for Claude that's one short Haiku reply, a few
hundred tokens; Codex costs nothing), and says what it found under the
login's name: "Has room again", or "Still out, back 3:40 pm". With room,
the threads waiting on it carry on straight away, without waiting for the
reset. A login running on extra usage or credits you paid for says "On
extra usage". Pools use it only when no login has room left on its plan,
because extra usage costs money.

A Claude login with a limit of its own on Fable shows that week too,
"Weekly Fable", with its own chart beside the others, as Claude Code's
`/usage` does. Running out of it only stops Fable: the login can still run
other models, so it never makes a login look full. If you don't use Fable,
turn off "Show models' own weeks" in Settings › General › Usage.

When a login's week will go partly unused (the green line), its open row
has **Find work for it**. It starts a thread on that login that looks through
your projects and threads, suggests 2 to 4 jobs worth spending the rest of
the week on (a deep review, the tests a report said were missing, rough
options for something you're exploring), and asks before starting any.
Each one you say yes to starts on that login, so it spends the week that
would otherwise go to waste. Pressing it again while that thread is still
open takes you back to it.

Threadbear also does this by itself, with "Tell me when a week's about to
go unused" in Settings › General › Usage (on to begin with). A day before
a login's week resets, if at the rate it's going 30% or more of it would be
left, the same thread turns up in your inbox, and once more 8 hours before
the reset as a last call if 20% or more would still be left. It never
starts the jobs without asking.

### Review by another lab

When a thread changes code (five lines or more, not counting docs) and
finishes with nothing for you to do, Threadbear has another lab try to
break the change: Codex reviews a Claude thread's change, and Claude a
Codex one's, to begin with. Settings › General › Code review has two
lists. Every change lists who reviews, in order: drag them into the order you want, add any other
provider you have a login for (Grok, say), and pick each one's model and
effort. The first that isn't the thread's own provider and has a login
with room does the review, so when one runs out for the week, the next
takes over. You can list a provider more than once on different models.
Gemini reviews through the Gemini login on a Google plan (above), or
through OpenCode: give OpenCode a Google AI Studio key
(`opencode auth login google`) and its Gemini models show up under
OpenCode once the runner restarts.

For a big change (150 lines of code or more) or one to code about
sign-ins, passwords, secrets or payments, a review panel can look at it
instead: the reviewers under Big or risky changes, each on its own model
and effort, review at once, each in a thread of its own. A provider can be
in both lists, and one only on the panel (Gemini, say, which is paid for
every token it reads) never reviews a small change. Your thread waits for all of
them and gets their findings in one numbered list, each saying who found
it, and the round only passes if none of them found anything. Smaller
changes still get the one reviewer, so the panel's cost goes where it
pays. With fewer than two of the panel able to review, the one
reviewer does. `"labPanelLines"` in `runner.json` changes the 150. The review runs in a thread of its own in the same folder, with
the same permissions as the thread it reviews, and is told not to edit
anything. Your thread says who is reviewing and waits. The reviewer gives a
verdict: a list of concrete findings, or nothing that holds up. Then your
thread wakes with them numbered, and it can't finish until it has answered
each one: fixed, disputed (saying why), or left for you to decide, which
comes to you as a question. Anything it fixed gets reviewed again, just the
new change, until a round finds nothing. A quiet line in the thread, over
the reply, shows who reviewed and how it came out. It folds like the
agent's work: click it for each finding and what was done about it, and
the review thread's link under them. Findings still open (left for you,
or not answered) show under it even while it's folded. It
stays off the done card, as it happens in the background. The review thread is archived when it's
done, so it stays out of your inbox. That holds however the review ends:
if it gives no verdict, asks you something or stops with an error, your
thread still wakes with whatever it said, and carries on. A thread gets at most three reviews
between your messages, and if the last one's fixes weren't looked at, the
card says so. For now the findings only show on the Mac whose runner ran
the thread; the phone, the website and your other Macs see the agent's
reply, which says the same in words.

With no login of another provider, a fresh session of the same one
reviews, and the note says it isn't another lab. A folder a pool is kept
for is only ever reviewed on that pool's logins. Reviews cost tokens on the
reviewing login, so if you'd rather not have them, turn off Settings >
General > Code review (`"labReview": false` in `runner.json`).

Claude threads run through Claude Code's Agent SDK, and Anthropic's 5-hour
meter seems to count that traffic as more per token than the same work typed
into `claude` in a terminal. People have measured anywhere from about 1.6 to
2.9 times as much ([anthropics/claude-code#97074](https://github.com/anthropics/claude-code/issues/97074)),
while weekly limits come out about the same in most of their tests. Some apps
change the label the SDK sends to get around it. Threadbear leaves the label
as it is for now.

## Standing instructions
<!-- slug: instructions -->

Settings > General > Instructions is for what every agent should always
keep in mind: who you are, how you like things written, what you build
with. Every new session gets it, on every login, Claude and Codex alike,
and a chat that's already going hears a change before your next message,
with a line in the thread saying so. It saves as you type; empty is none.
If a save fails, the GPUI app shows the error and keeps your edit while the
app stays open. Reopen Settings to restore it and retry. Automatic updates
wait until the edit is saved.
A login's own `CLAUDE.md` or `AGENTS.md` still applies, and is the place
for what's particular to that login or provider: this box is for what goes
for all of them, without keeping several files in step.

Only what you save in Settings counts. If anything else changes the file
(an agent, say), no agent is given it, Threadbear warns you, and the box
shows what the file holds so you can keep it or clear it. Agents can't
change it for you, so ask one to draft some and paste them in yourself. On
Full access an agent that goes after the runner's own key could still get
past this, as it could with anything on your Mac.

## Open Anything
<!-- slug: open-anything -->

In the new desktop app (the `gpuiApp` experiment), ⌘K or ⌘P (or Go > Open
Anything…) from anywhere in the main window puts a box over it, with the rest
dimmed. Type part of a name and
it finds threads, panels, projects and the app's own pages, in that order of
groups: Panels, Threads, Projects and App. Letters in order are enough
("ytcm" finds yt-channel-manager).

- Threads are found the way the sidebar's search finds them (the title, the
  last reply, the librarian's search words), archived and hibernating ones
  too, and it says so on the right.
- A panel shows its count and the folder or project it comes from. If the
  open thread hasn't got it, it opens on the project's page when its folder
  is in a project, otherwise on the newest thread in that folder.
- App has Inbox, Scheduled, Projects, Librarian, Usage, Updates, Connectors,
  Runners and each section of Settings ("keyboard" finds Settings ›
  Keyboard).
- What Return opens is the first one named for what you typed, even below
  others: "usage" opens Usage, not a thread that only mentions usage.

With nothing typed it shows Recent (what you've opened lately, through the
box or not) and Go to (the pages, with their keys). ↑ and ↓ move, Return
opens, and ⌘Return opens beside: a thread in a panel of its own, or a panel
next to the main one. Esc or a click outside puts it away, and ⌘K again does
too. The Mac app hasn't got it.

## Keyboard shortcuts
<!-- slug: keyboard -->

<!-- picture: /docs-img/tall-keyboard.webp alt: Settings > Keyboard, where any command can have a key of your own -->

Hold ⌘ to see the keys beside the buttons that have them. Settings (⌘,) >
Keyboard lists every command in the menus that can have a key, and you can
change any of them there. Click a command's key and press the new one. Esc
cancels, and Delete removes the key. A command with no key has an Add button,
and one you've changed has Reset beside it.

A key has to include ⌘ or ⌃, or be a function key, otherwise it would type
instead. If another command already has the key, it asks whether to give it to
this one, which leaves the other with no key. Keys that something else needs,
like ⌘C for Copy or ⌘← for moving the cursor, can't be used.

The keys you've changed are in `shortcuts.json` in Threadbear's data folder, so
you can ask an agent to change them too, and a change to the file shows straight
away. It holds only what you've changed, by the names below, and `null` means no
key:

```
{
  "nextInboxThread": "ctrl+j",
  "archive": null
}
```

- New Thread: `newThread`, ⌘N
- Archive: `archive`, ⌘E
- Hibernate (opens Pick a time): `hibernate`, no key
- Stop: `stop`, ⌘.
- Copy Transcript as Markdown: `copyTranscript`, ⇧⌘C
- Choose Model (the new desktop app only): `chooseModel`, ⌥⌘/, opens the message box's model picker with its search ready, and closes it again
- Attach Images or Markdown (the new desktop app only): `attachFiles`, ⇧⌘A, opens the message box's + > Attach images or Markdown… picker
- Find in Thread (the new desktop app only): `findInThread`, ⌘F, in the Edit menu; ⌘G and ⇧⌘G then go to the next and previous match
- Back: `back`, ⌘[
- Forward: `forward`, ⌘]
- Next Inbox Thread: `nextInboxThread`, ⌥⌘↓
- Open Anything (the new desktop app only): `openAnything`, ⌘K, `openAnythingAlso`, ⌘P, and `openAnythingPalette`, ⇧⌘P
- Go to Message Box (the new desktop app only): `focusMessageBox`, ⌘L, puts the cursor in the open thread's message box
- Show Inbox: `showInbox`, ⌘1
- Show Scheduled: `showScheduled`, ⌘2
- Show Projects: `showProjects`, ⌘3
- Show Librarian: `showLibrarian`, ⌘4
- Show Skills: `showSkills`, ⌘5
- Show Notes: `showNotes`, ⌘6
- Show Hibernating: `showHibernating`, no key
- Show Archived: `showArchived`, no key
- Settings: `settings`, ⌘,
- Quit Threadbear: `quit`, ⌘Q
- Zoom In: `zoomIn`, ⌘=
- Zoom Out: `zoomOut`, ⌘-
- Actual Size: `actualSize`, ⌘0
- Show Threads: `toggleThreads`, ⌃⌘S, and `toggleThreadsAlso`, ⌘B (the new desktop app only)
- Collapse Threads: `collapseThreads`, ⌘/
- Collapse Sidebar (the new desktop app only): `collapseSidebar`, ⌥⇧⌘B
- Show Sidebar Panels (the new desktop app only): `toggleSidebarPanels`, ⇧⌘B, closes Runners, Connectors, Updates, Usage and Setup and keeps the Threads list
- Show Panels: `togglePanels`, ⌥⌘B (⌥⌘P in the Swift app)
- Show Terminal: `toggleTerminal`, ⌘J, and `toggleTerminalAlso`, ⌘` (the new desktop app only; it takes ⌘` from macOS's switching between Threadbear's own windows)
- Reset Panel Layout: `resetPanelLayout`, no key

In the file, a key is its modifiers and then the key, joined with `+`: `cmd`,
`ctrl`, `opt` and `shift`, then a character, or `down`, `up`, `left`, `right`,
`return`, `tab`, `space`, `delete`, `home`, `end`, `pageup`, `pagedown` or `f1`
to `f20`.

The rest are every Mac app's own and can't be changed: the Edit menu's (⌘Z,
⇧⌘Z, ⌘X, ⌘C, ⌘V, ⌥⇧⌘V, ⌘A, with Start Dictation and Emoji & Symbols under them),
⌘W to close the window in front, ⌘M to minimize it, ⌃⌘F for full screen, and
⌘H and ⌥⌘H to hide. In Settings, the up and down arrows move through its
sections.

## Experiments
<!-- slug: experiments -->

<!-- picture: /img/bear-dizzy.webp alt: Experiments are off until you turn them on -->

Some new features start as experiments: off unless you turn them on, while
they're tried out. Turn one on or off in Settings > General > Experiments
(the ⓘ beside each says what it does). Nothing restarts, except Voice mode,
which lives in the app and takes effect when you quit and reopen
Threadbear. The switches are kept under `experiments` in `runner.json` in
Threadbear's data folder, read at every turn, so an agent can set them
there too:

```json
"experiments": { "keepAwake": true }
```

Once an experiment has proved itself it becomes a standard part of
Threadbear, and its switch goes.

<!-- experiments:start -->
The ones there are now, and what each does, are in
[docs/experiments.md](experiments.md), which is made from the experiments
themselves so it stays true.
<!-- experiments:end -->

## Connectors
<!-- slug: connectors -->

<!-- picture: /docs-img/connectors.webp alt: The Connectors panel, bottom left: each service and whether it's signed in -->

Agents get services like Notion, Sentry and Cloudflare from your Connectors
gateway, and Gmail, Drive and the like from claude.ai's connectors on each
login. Now and then one needs signing in again.

The gateway is optional: it's a small server of your own
([plugbear](https://github.com/mikecann/plugbear)) that signs in to each
service once for all your logins and Macs. Without one, agents use
claude.ai's connectors alone, and Notion boards on projects aren't read.

Claude and Codex threads find the gateway in their own config, which
plugbear's `install-clients.sh` writes. Cursor and OpenCode threads get it
from Threadbear instead, added to each session, as plugbear can't write
their config and Threadbear keeps OpenCode's from loading. That needs the
agent in the gateway with a token in your Keychain: add `cursor` or
`opencode` to plugbear's `agents.json`, run `connectors agent add cursor`
and save the token under the Keychain service `connectors` with the agent
id as the account. Without one, the thread starts without Connectors.

Once there's a gateway, the Connectors icon (the plug) near the bottom of
the left-hand rail opens a panel listing the gateway's services and whether each is signed in. A blue
dot on the icon means one is signed out: click Connect beside it, and its
sign-in page opens in your browser. A grey dot means something's off that
usually sorts itself out, such as the gateway not answering for a moment.
The arrow in the panel checks again, and the ⓘ says more. If the gateway
can't be reached at all, the panel says so.

When the gateway holds a call for your OK (a Slack post, an email), the
card shows up in the thread that made it, with everything the call would
send: Allow sends it, Deny doesn't. It needs the gateway on this Mac with
its approvals API (plugbear's local mode); otherwise, or from the phone,
approve it on the gateway's own page as before. The page still works, and
a call decided there closes the card.

When an agent runs into a service that needs signing in, it asks you in its
reply, with a button that opens the page to do it on. That's only when the
gateway itself says the service is signed out (or claude.ai's own sign-in
error comes back), never because an email or a page the agent read says
so: those are anyone's words, so a "sign in again" in them asks nothing. claude.ai's own
connectors are signed in on claude.ai (Settings > Connectors), on the login
the thread runs on.


### Setting up the gateway, and adding services

The GPUI desktop app has the same setup, service suggestions, outside-agent
forms and local-server review as the Swift app. Hidden rows stay hidden on
this Mac, including their sign-in alerts.

With the Connectors setup experiment on (Settings > General >
Experiments), the plug icon shows before there's a gateway, and the panel
has **Set up**.

1. Click **Set up**. A thread called "Set up Connectors" opens and works by
   itself. It installs plugbear on this Mac (it listens only on this Mac, at
   127.0.0.1:8795), wires each of your logins to it, and points Threadbear at
   it. It asks you before anything that needs a password or changes your
   logins' settings, and says what it changed. If a step fails it says
   which, and pressing Set up again carries on from there. Threadbear picks
   the gateway up the next time its runner restarts, which it does by itself
   once nothing is busy.
2. Once it's running, the panel lists its services and, under **Add a
   service**, ones it doesn't have yet: Notion, Slack, Linear, Sentry,
   Cloudflare and DeepWiki. Click **Add**. A short thread adds it and
   restarts the gateway, and in a few seconds the service shows with
   **Connect**.
3. Click **Connect** and sign in on the service's own page. The row turns
   green and says how many tools it has. Ask any thread "list my Notion
   pages" to check. If it says "Sign-in expired" or similar, click
   **Reconnect**.

**Slack** needs a Slack app of your own, and the panel makes it about a
minute. Its row (or its entry under Add a service) has **Create Slack app**:
it opens Slack with the app already filled in (the redirect address, the
user scopes, and Slack's MCP switch, which Slack's MCP server needs). Pick
your workspace and press Create. Then copy the Client ID and Client Secret
from Basic Information, App Credentials, into the two fields under the row
and press **Save**. Threadbear saves them where the gateway reads them
(never in Threadbear, never in a chat), turns Slack on, adds an "ask" rule
for scheduled messages, and restarts the gateway, and the row shows
**Connect**. Already made the app? **Paste keys** opens the fields without
going to Slack. This works for a gateway on this Mac; for one on another
Mac, Add starts a thread that does the same with you. Your workspace's
admin may have to approve the app. Posts then wait for your OK.
"does not support dynamic client registration" means the Client ID and
Secret weren't saved, and "redirect_uri mismatch" means the redirect address
in the Slack app differs from the gateway's.

**Any other MCP server**: click **Other MCP server...**, paste its `https`
address and give it a name. Only public `https` servers are taken. To take a
service out, its row's arrow opens its page on the gateway, where Disconnect
is, or ask a thread to remove it from the gateway.

### Local servers (Blender and others)

With the Local connectors experiment on as well (Settings > General >
Experiments), the gateway can run MCP servers that are a command on this
Mac, not a site on the web. **Blender** shows under Add a service with an
**On this Mac** tag, and **Other local server...** takes any `uvx` (PyPI)
or `npx` (npm) package.

1. Click **Add...**. A sheet shows the exact command that will run (for
   Blender, `uvx mcp-for-blender@2.2.7`), that it runs on this Mac whenever
   the gateway is running, and which of its tools wait for your OK first.
   Blender's `execute_blender_code`, which runs any Python in your Blender,
   always does. Click **Add** to go ahead.
2. A short thread adds it to the gateway, pinned to that one version, so
   what runs can't change under you. If `uv` isn't installed it says so
   and installs it with Homebrew. For Blender it also copies the MCP for
   Blender add-on into Blender's add-ons folder.
3. Open Blender. The add-on connects by itself; if it doesn't, turn on
   "MCP for Blender" in Preferences > Add-ons, press N in the 3D view and
   click **Connect to MCP server** in its tab. The gateway's Blender row
   can be green while Blender is closed: its tools then answer that they
   can't reach Blender.
4. Ask any thread, on Claude or Codex, "what's in my Blender scene?".

A local row has no Connect button, as there's nothing to sign in to. When
it won't start, the row says so, and its arrow opens its page on the
gateway, where Retry is; `~/Library/Logs/connectors.log` has why. Other
local server takes only one exact version (like `1.2.3`): not a range,
`latest`, a git address or a URL.

**Hiding rows you don't use**: hover over a service, a suggestion under
Add a service, or Connect an agent, and click the **×** at its end. It only
leaves this panel: a hidden service is still on the gateway and agents can
still use it (sign it out on its page to stop that), and it no longer
lights the plug's dot. A **+ hidden** menu at the bottom of the panel lists
what you hid; pick one, or Show all, to bring it back. While Grok Bot has a
live key its row stays, so Revoke is always there. What's hidden is kept on
this Mac.

### Letting Grok Bot, Notion and other agents use Threadbear

With the Outside agents experiment on and your runner syncing to your
backend (Settings > Phone & browser), the Connectors panel has **Agents
that use Threadbear**. These agents run in the cloud, so they reach
Threadbear through your backend rather than this Mac.

1. Click **Connect an agent** and pick **Grok Bot**, **Notion** or
   **Slack**. Each says what it needs first, and its guide link opens that
   app's own page on adding an MCP server.
   - **Grok Bot** is xAI's agent. On a team plan, your admin's connector
     policy has to allow servers by address.
   - **Notion** works with its Custom Agents, on the Business or
     Enterprise plan. A workspace admin turns on custom MCP servers first,
     in Settings > Connections.
   - **Slack** can't connect yet. Slackbot only connects to servers that
     sign in with OAuth or Slack's own sign-in, and Threadbear's backend
     uses a key.
2. Tick **Can start Ask threads and message them** if you want it to do
   more than read, and **Can read all my threads** if it should see yours,
   not only the ones it started.
3. Click **Make key**. The panel lists the boxes of that app's form, each
   with a copy button. The key itself is never shown, only copied, so it
   doesn't end up in a screenshot. Copy what you need before you close the
   panel: after that the key is gone, and you'd make a new one.
4. Fill in the app's form:
   - In **Grok Bot**, open **Connect apps**, then **Add a custom
     connector**. **Name**: `threadbear`. **Type**: **Remote HTTPS** (not
     Command, which runs a program on your own computer). **Server URL**:
     the address from the panel, ending in `/mcp`. **HTTP headers**:
     `Authorization` in the left box, and in the right box `Bearer `, a
     space, then the key; the panel's Header value copies all of that in
     one go. Leave **Client ID** empty: it's for services you sign in to
     in a browser, and the key does that here.
   - In **Notion**, open your Custom Agent's settings, then **Tools &
     Access**, **Add connection**, **Custom MCP server**. Give it the name
     and server address from the panel, pick header-based auth rather than
     OAuth, and add the header: `Authorization`, with the panel's Header
     value.

   Save it, then ask the agent to list your Threadbear threads. Its row in
   the panel says "Used just now".

Threads it starts always run in Ask mode, so every tool asks you first,
and show who started them ("started from Grok Bot"). Its messages say
which agent sent them, not you. It can only message threads it started,
and only while they stay in Ask mode: switch one to Full access and it
stops. It never sees approvals, secrets, schedules or archived threads,
and it's held to a few calls a minute and ten new threads a day. Keys
expire after 30 days. If one leaks, or you lose it, click **Revoke** on
its row and Connect an agent again for a new one. **Revoke** ends a key
on its next call, and turning the experiment off ends every key. Then
remove the connection in the agent's own settings too. What an agent
reads goes to the company that runs it (xAI for Grok Bot, Notion for
Notion).

## The prompts macOS shows you
<!-- slug: macos-prompts -->

<!-- picture: /img/bear-sit.webp alt: macOS asks once for each thing, and what each answer means -->

- **Notifications.** The first time the app opens, macOS asks whether
  Threadbear may send notifications. Either answer is fine: with them on,
  the app tells you when a thread needs you and you aren't looking at it,
  which helps most when your threads run on Full access and you've
  wandered off. You can change it later in System Settings >
  Notifications.
- **Folder access.** When an agent reaches into a protected folder, such as
  Desktop, Documents, Downloads or Dropbox, macOS asks whether to allow it.
  Click Allow if you want agents working there, or Don't Allow if not. You
  can change your mind later in System Settings > Privacy & Security >
  Files and Folders. iCloud Drive and removable drives ask too. The prompt
  names Threadbear Runner when an agent asked, and Threadbear when the app
  itself opens something there for you, such as a pinned picture on your
  Desktop. Starting threads in `~/ThreadbearThreads` keeps these prompts
  away until you choose a folder that needs them.
- **A Mac nobody's sitting at.** The agent waits for that answer, so on a
  second Mac a folder prompt can hold a thread up with nobody there to click.
  Give Threadbear Runner Full Disk Access on that Mac, in System Settings >
  Privacy & Security, and the prompts stop. Do this when you set the Mac up:
  a waiting prompt can also hold up other apps using the same drive until
  someone answers it. The trade-off is that agents there can read everything
  your account can. If a thread does get stuck like this, it
  says so after a minute or two and goes in Needs you. Five minutes later
  Threadbear stops the turn, and your next message carries on from there.
- **A thread called "... needs someone at its screen".** Threadbear starts one
  when a macOS dialog has been waiting on a Mac for 5 minutes, or when a Mac
  without the Threadbear app doesn't have Full Disk Access yet. Open Screen
  Sharing to that Mac and answer the dialog, or turn on Threadbear Runner in
  Full Disk Access there. When the dialog's gone or the access is on, the
  thread says so within a couple of minutes and archives itself, unless
  you've written in it.

## Your phone and a browser
<!-- slug: phone-and-browser -->

<!-- picture: /clips/phone.mp4 alt: Your threads on your iPhone, answered from anywhere -->

The Threadbear iPhone app is for when you're away from the Mac: ask a
thread a question, steer it, answer the ones that need you, start new
ones, and see how they're all getting on. Its inbox sorts threads the
way the Mac's does, into Starred, Inbox, Running, Hibernating and
Archived. A thread that finished saying it needs nothing from you is done
there too: it doesn't rise to the top or count towards what needs you,
though it shows as unread until you open it. To star a thread, swipe its row to the right, or use the menu when
you press and hold it or the ... menu inside it. All but Starred and Inbox start folded, showing how many they hold; tap a
heading to open or fold it, and the phone remembers. Each heading stays at the top while you scroll through its threads, and a thread that changes heading travels there, so one finishing rises from Running into the Inbox. A folded Archived isn't
loaded at all, nor the hibernated threads in a folded Hibernating, so the
phone fetches less. A thread's finished turns
fold into one Worked line above the reply, as on the Mac; tap it to see
the work. Swipe right on a thread's messages to go back, and left to go
forward again to the thread you just left, with the Mac's card saying where
letting go lands. A code block or table that can still scroll keeps the
swipe. The search at the top of the inbox finds any thread, hibernating
and archived ones too, by its title, last reply, project, the librarian's
search words for it, or a word said anywhere in it. What a thread needs from you shows under its reply with the
likely answers as buttons (type in the box to say something else; the one you tap spins until the runner has it, and the card comes back if it's refused), and
a thread that's done offers its suggested next steps and Archive, which takes you back to the inbox as its row slides away. Hibernate from a thread's ... menu does the same, with its row sinking towards the Hibernating heading. Clear the last thread from the inbox and the bear waves at you, the once. Pull the inbox down and let go to reconnect to your Mac's mirror, if the list ever looks stuck. A new thread from the phone starts in your default folder; pick its model and effort under the message, or leave them on Default. With more than one runner it starts on the first one that's online in your order, so a sleeping Mac doesn't hold it up; pick another under Runner, or change the order in Settings > Runners > Change the order. The sheet closes as soon as you tap Start, and a row at the top of the inbox says Starting until that Mac has made the thread, which then opens. If it couldn't start, the row says why; tap it to open it again as you left it, photos and choices too. While a thread is working and nothing's typed, Send turns into Stop, as on the Mac, here and on the website; it spins until the turn has stopped. Anything typed while it works goes into that turn instead. Paste a photo straight into any message box, or add one with the photo button: it goes in where the cursor is as a chip, "Picture 1", as on the Mac, and pictures in your messages show as chips too. The photos you sent show over your message, where the Mac uploaded them for your website (on unless `"sync": { "pastedPictures": false }`); one that stayed on the Mac leaves a "[1 picture]" line instead. Pictures an agent puts in its reply or pins show in the thread, and animated GIFs play (one of hundreds of frames, or too big or fast to keep decoding, shows its first frame still, and Android plays four at once at most); tap one to see it full screen and pinch to zoom. An MP4 or MOV video an agent pins shows its first frame with a play button, and plays when you tap it. Anything over 10 MB, or from a thread that's been quiet for over two weeks, says it's on your Mac. Rename a thread from its ... menu or the menu when you press and hold its row: the new title shows straight away, and goes back with the error if your Mac can't take it. To point one thread at another, use Copy Thread ID in the same menus, and paste it into a message, here or on the Mac: it goes in as a link to that thread, with its title. When the Mac a thread runs on is asleep or offline, its message box says so, and what you send waits for it: each message shows faded with a Cancel under it until that Mac takes it, and Cancel puts its words back in the box. It can send notifications too,
though you don't need them for any of that. It pairs with your Mac by scanning a
QR code: on the Mac, Settings (⌘,) > Phone & browser > Show the code, then
scan it with Scan the code in the iPhone or Android app (not the Camera app:
the code isn't a link, so the Camera only shows it as text). Keep the code to
yourself: it works once, for ten minutes, and whoever scans it first can run
your agents. To pair a phone that's paired already, Unpair it in its Settings
first. The same tab lists each paired phone and browser with
when it was paired and until when (90 days), and Revoke signs one out and
stops its notifications; Settings > Unpair this phone on the iPhone forgets it from that side. The
terminal command setup used to print still works too:

    swift ios/scripts/pair-qr.swift --prod --url <your deployment URL>

If you have them on, notifications reach your phone only while you're away
from the Mac by default. Threadbear counts you as at the Mac when the app is open and you've
used the keyboard or mouse in the last two minutes, and as away when the Mac
is asleep. On the iPhone, Settings > Notify me changes it to Always or Never. A
notification goes away by itself once its thread no longer needs you: when
you open it on the phone, deal with it on the Mac, or archive or hibernate
it. A thread you've archived or hibernated sends none, even one held back
while you were at the Mac; a hibernated one that still needs you when it
wakes sends one then.

Ask on the iPhone: press the Action button, say a question, and hear the
answer from an archive server of your own, one that knows your past
threads. Mike's is Agent Archive, which isn't public yet: he'll be talking
about it soon and releasing it then. Until then, any server that speaks
[Ask's small protocol](https://github.com/mikecann/threadbear/blob/main/docs/ask-server.md)
works. Set it up in the iPhone's Settings > Ask: Scan the code reads a
setup code your server shows (Agent Archive's
`npm run phone-token -- issue --bruce --qr`), or type in the server's
address and a token from it. The phone has to be able to reach the server,
over Tailscale for one on your own Mac. Then set the Action button to
Threadbear's Ask shortcut. The waveform button in the inbox does the same.
Each answer is kept as a thread in your default folder, with the question,
the answer and a note that Ask answered it. It shows as done and already
read. Once it's kept, the phone opens that thread with a big mic at the
bottom: tap it and say a follow-up, which goes to the agent as an ordinary
reply on a small fast model (Haiku, or GPT-6-Luna with no Claude login; change
it in the thread if it needs more), and its answer is read out when it's done. The keyboard button
beside the mic brings back the text box. The agent is told the thread
started with Ask and picks up from there.

The Pocket is for what you'll want while you're out. Tell a thread you're
heading to the post office and its agent can file a card with the address and
what to take; the wallet button at the top of the inbox opens the deck, and
its badge says how many cards there are, in colour when one's been filed or
changed since you last opened it. The
cards are saved on the phone, so they open straight away with no signal. Tap
a card to bring it forward, then tap it again to fill the screen, black on
white, for reading out or holding up at a counter. Swipe a card left to remove
it, with a few seconds to Undo, or press and hold to copy it, open its thread
or remove it. Each card has its own icon and colour, chosen by whoever filed it. A card can name a place: while the app's
open the phone checks where you are, marks that card "here" and brings it to
the front (it asks first, and only the phone and Apple's map search see your
location). Agents replace or remove cards too, and the pencil
starts a thread to add one. The librarian looks after them as well: it updates
a thread's card when the facts change, takes it out once the thread says it's
done, and files one when you say you're going somewhere and the agent didn't.
It only touches cards from the thread it's looking at, adds at most one per
thread, and won't put an address or code on a card unless the thread said it.
The Librarian view lists what it did.

Your threads also have a website, on your own Convex deployment. To sign a
browser in, Settings > Phone & browser > Open signed in opens it in your
browser (or run `npm run web:sign-in` from your Threadbear folder). Each
thread has its own link. Open on the web in a thread's ... menu on the Mac
takes you there, and the Copy link button on the website copies it. It works
anywhere you're signed in. Its sidebar works
like the Mac's: drag its edge to resize it (double-click to reset), click a
heading to fold it, sort the Inbox from the arrows on its heading, and search
or filter from the top. The filter is only on the website: it picks threads
that need you, unread or shared ones, or one project or folder. The rail down
its left edge opens your schedules, projects, runners and each login's usage.
A schedule can be paused or run from there, and a runner shows how busy its
machine is, with an orange dot on its tile while it's low on memory or disk.
Other changes are done on the Mac or by asking an agent. The gear
has the appearance and Sign out. Pictures you paste show there too: they go to
your Convex deployment as they are, so nothing can scrub a secret from one, and
`"sync": { "pastedPictures": false }` in `runner.json` keeps them on the Mac. A tool line opens to what the tool did if its runner has full sync on (it starts off; `tb settings full on|off` changes it, and docs/on-your-mac.md says what it sends), and its screenshots show when you tap them. The pencil beside the search starts a new
thread on a runner's default folder and login. On your Mac,
`threadbear://thread/<id>` opens a thread in the app instead (the Open
thread button on a tab BrowserBear has marked for an agent uses it). With
the new desktop app installed beside the Mac app, those links open the new
one; `desktop/scripts/thread-links.sh swift` sends them back to the Mac app.

To let someone else read a thread, click the share button beside a
thread's ... menu (or Share with others in that menu's Share submenu) and
pick Anyone with the link, then Copy link. It's a
read-only page anybody can open, which follows the thread as it carries on.
It reads like a short write-up: each of your messages, the work folded
into one "Worked for" line (open it for one line per step), the answer,
and the pictures, but not tool output, the folder or the cost. Pasted
into Slack, X or Messages, the link shows the thread's title, the start
of its last reply and its first picture. Only you stops the link working (sharing again brings
the same link back), and the arrows beside the link make a new one, which
stops the old. The same Share button is on the website, on your thread's
page and on the shared page while you're signed in. On the iPhone and in
the Android app, it's Share… in a thread's menu: turn on Share with a
link, then copy it or send it from Messages, Mail or any app, and New link
makes a new one, which stops the old. With the Mac asleep the change
waits for it, and the sheet says so.

The Mac app also draws a picture an agent writes into its reply with a path
on your Mac (`![a chart](/path/to/chart.png)`), rather than showing a link.
A picture written as a web address (`![a chart](https://example.com/c.png)`)
is a link everywhere, on the Mac, the phone and the website: tap or click
it to open it in your browser. Threadbear never fetches one by itself, so
whoever runs that site can't tell you're reading. On the phone a reply
draws its first 10 pictures; any past that show as their names.

The website also has a Machines page, at `/machines` on your website's
address, for whoever's signed in there: it draws each of the backend's
state machines (so far, how a message or request from your phone or the
Mac travels to a runner: waiting, taken, done or failed, and lapsed when a
runner never says how it went; and how a thread moves to another Mac, from
asking through packing and unpacking to done; and another Mac's request
for a new thread, waiting for you, being started, allowed, denied or
closed; and a deleted thread's old messages being cleared away), with how many are in each state right now,
and any one's history. It's for looking into something that seems stuck; it
changes nothing.

### On Android

There's an Android app too, for the same things: your inbox, answering
threads, starting new ones and notifications. It isn't on Google Play yet,
so you install it yourself from GitHub, where each Threadbear release puts
a new copy:

1. On your Android phone, download
   [the latest Threadbear APK](https://github.com/mikecann/threadbear/releases/latest/download/threadbear-android.apk).
2. Open it from your downloads. The first time, Android asks whether your
   browser may install apps: allow it, then tap Install.
3. Open Threadbear and pair it with Scan the code, as above.

After that it looks for new releases itself. When one's out, its inbox
menu and Settings show "Update to" and the release's date, and tapping it
downloads the new APK to install over the top, still paired. Notifications
come through Google's Firebase, so a phone without Google Play services
gets everything but those.

## Skills

Skills give your agents extra know-how: how to draw a Mermaid diagram,
build an isometric explainer, write in a house style. Click Skills in the
bar on the left (under the Librarian), or press ⌘5, to see yours. The page opens over the
main panel; its X or Esc puts you back where you were. With a long list,
type part of a name in the search beside Your skills (rem finds every
remotion one); it narrows Suggested too. Skills shows only
while the Skills store experiment is on (below).

To add one, paste its GitHub link in Import a skill and press Import: a
repo (github.com/owner/repo), a folder in one, a SKILL.md, or even a
skill's own web page if it names its GitHub repo or `npx skills add` line.
A repo with several skills adds them all. Each is pinned to the commit it
was imported at, so it never changes under you; Update to the Latest in
its ... menu moves it on. Suggested lists ones worth having, such as Pele
(Mermaid diagrams, which comes with Threadbear) and Anatomy (interactive
drawings of how a machine works), with an Add button each. Make-print and
bundle-report come with Threadbear too: they're how a thread builds Prints
(charts, dashboards, reports, progress, and the Convex bundle, insights and
API map reports), so add them if you use Prints.

To write one of your own, press New Skill at the top of the page: give
it a name (its slash command: brand-voice becomes /brand-voice), say
when agents should use it if you like (its first line is used
otherwise), and paste or type its text: plain instructions, or a whole
SKILL.md, which keeps its own header. Save (or ⌘Return) adds it, Always
on, and it works like any other skill. Edit in its ... menu changes it,
name and all.

A skill only brings instructions. Anything in its header that would run
or allow something by itself (`hooks`, `allowed-tools`, `mcpServers`,
`permissionMode`) is left out of what threads get, for imported skills
and your own alike, so a skill can't pre-approve shell commands or start a
hook. A header Threadbear can't read line by line keeps only its name and
description.

Every skill works in every thread, on every login, Claude and Codex alike.
With Always on, agents use it whenever it fits what you ask. Turn it off
and it's there only when you ask by name: type its slash command, like
/pele, in a message. Typing / at the start of a word opens the slash
palette (see Slash commands), or, with Settings > General > Slash
commands set to the small list, lists your skills over the cursor, narrowing as you type: ↑ and ↓ to choose, Return or Tab
to put it in, Esc to close. A skill's command turns blue as you type it,
and shows as a link in your message once sent, so you can see a skill
will run. Skills page skills show under their own names, as `/pele`
(`/threadbear:pele` works too). That's any skill the thread can use: the Skills page's, and
the ones Claude or Codex find themselves, such as those in
`~/.claude/skills` or the folder's own. Click one for the Skills page,
or for one that isn't there, its SKILL.md. In the GPUI app, the link
opens its SKILL.md beside the thread. It works anywhere in
the message, in Claude and Codex threads alike. A change reaches threads
that are already running too: each gets a fresh start before its next
message, carrying on the same conversation with the new skills. Remove
takes a skill away everywhere. The store is an experiment, Skills store
in Settings (⌘,) > General > Experiments (the Mac app's Use the skills
store switch is the same one). Turn it off to stop giving threads any
skills at all; Skills goes from the bar in both apps and your skills are
kept for when you turn it back on.

## Notes

Notes, under Skills in the bar on the left (⌘6), is a place for your own
notes. New Note starts one called "Untitled note" (then "Untitled note
2" and so on); click its title above the text to rename it, and delete it
with the bin or its right-click menu. The text box has line numbers down
the side and basic Markdown coloured as you type (headings, **bold**, *italics*, `code`, lists,
links and quotes). Headings get bigger by level: `#` is the biggest, then
`##` and `###`. Start a line with `- [ ] ` for a task: click its box to
tick it (it turns to `[x]` and the task is struck through) or untick it,
and ⌘Z undoes a click. Return on a list line starts the next item (the
next number, or a fresh box on a task list); Return on an empty item ends
the list. A note stays plain Markdown text underneath, so it copies out as
it is. Notes save as you type. The sidebar button left of
the Notes title folds the list of notes away for more room to write, and
brings it back; Threadbear remembers which you left it on. To leave
Notes, press Esc, click its X, or go anywhere else: ⌘1 to ⌘4, or any
view in the bar (Threads, Scheduled, Projects, Librarian).

Your notes stay on this Mac and are yours alone: each is encrypted, with
a key Threadbear keeps beside its other secrets (readable only by you), so
a copy of the notes folder on its own can't be read, and they're never
sent to the runner, an agent, your phone or the website. Opening Notes
never asks for your Mac password. Notes are for ideas and to-dos, not
passwords: keep those in a password manager. To hide Notes, turn off Show Notes under
Experiments in Settings (⌘,); the notes are kept.

The GPUI desktop app has Notes too, in the bar under the Librarian (⌘6),
with the same notes as the Swift app. Back and ⌘W leave it as Esc does.
There a note reads like a doc rather than code: it's set in the app's own
type on roomy lines, with the line numbers level with each line. Under the
note's title, − and + make the text smaller or larger (12 to 22 points),
and Sans or Mono picks the type; it remembers both. Two things differ
there for now: headings are bold but not bigger, and you can't dictate
into a note.

With Voice on, you can dictate into any text box in Threadbear (the
message box, a note, a search field) from the keyboard:

- Hold Fn to dictate; let go to type in what you said.
- Shift Fn (or Fn Space) starts hands-free dictation, which types in
  each time you pause; Shift Fn or Fn Space again (or a tap of Fn) stops
  it. Two quick taps of Fn can start it too, once you turn that on: it's
  off at first, as macOS uses a double tap of Fn for emoji or dictation.
- Fn Control is Command Mode, once it's on (Settings > Voice > Command
  Mode): hold it and talk, and when you let go what you said goes to the
  open thread as your message (or starts one on a new thread).
- Esc cancels, with nothing typed in or sent.

A pill over the bottom of the window shows what it's heard. With no text
box in use, it types into the message box. Settings (⌘,) > Keyboard >
Voice changes the key (Fn, Right Option or Right Command), turns any of
these on or off, and picks the key held for Command Mode. If macOS opens emoji
or its own dictation when you press Fn, set System Settings > Keyboard >
Press 🌐 key to Do Nothing (Settings > Voice has a button that opens it), or use another key. Threadbear can't stop macOS acting on Fn itself. They only work while
Threadbear is the app in front, and not while Voice is on in a thread or
a note is listening.

You can talk into a note too, once Voice is on (see Talking with a thread):
turn on Use with Notes in Settings (⌘,) > Voice, and a microphone button
shows beside the note's bin. Press it (it turns red) and talk; each time
you pause, what you said is typed in at the cursor, as if you'd typed it,
so it saves and undoes the same way. The words you're saying show under
the note until they go in. Say "new line" or "new paragraph" on its own
for a line break. Press the button again, Done, or Esc to stop; going to
another note or closing Notes stops it too. Speech is turned into text on
your Mac by macOS, so a note's words still never leave it, and nothing is
downloaded for it beyond macOS's own speech files. Talking into a note
ends Voice on a thread, and starting Voice on a thread ends it, as there's
one microphone.

## Self-hosting your backend
<!-- slug: self-hosting -->

We recommend Convex cloud, which is what setup uses unless you ask for
something else. Your threads go to a Convex project in your own account,
which keeps working while your Mac is asleep, sends your phone
notifications, and needs nothing else installed. The free plan is enough
for one Mac's ordinary use.

<!-- video: /video/self-hosting.mp4 poster: /video/self-hosting.webp title: Setting up a self-hosted backend, read out by the Threadbear bear -->

If you'd rather keep everything on your own machine, setup can run
Convex's open source backend in Docker on your Mac instead, and reach it
over Tailscale (or your own way, below). It's the same backend and the same features, with these
limits:

- **No phone notifications.** The push relay only works with Convex
  cloud. The iPhone app shows what needs you when you open it, and its
  Settings say why nothing arrives.
- **Your phone needs Tailscale on**, as the backend is only on your
  tailnet.
- **Nothing works while your Mac sleeps** or Docker is stopped: the phone
  and the website can't reach the backend until it's back.
- **Share links only open for people on your tailnet**, so "Anyone with
  the link" means anyone on your tailnet.

You need Docker Desktop (or OrbStack), set to start when you log in, and
Tailscale, logged in, with MagicDNS and HTTPS certificates on for your
tailnet. Then run setup with `--self-hosted`, or ask your agent to
(INSTALL.md says how):

```sh
./threadbear setup --self-hosted --yes
```

It starts the backend and its dashboard in Docker, on this Mac only, and
puts them on your tailnet with `tailscale serve`: your threads' API on port
3210, your website on 3211 and the dashboard on 6791, all at
`https://<your Mac>.<your tailnet>.ts.net`. Then it deploys Threadbear's
backend and website there as it would to Convex cloud, and keeps them up
to date the same way. Pair your phone and open the website from Settings ›
Phone & browser as usual.

The dashboard's sign-in asks for the admin key, which is in
`secrets/convex-self-hosted.env` in Threadbear's data folder. Keep it to
yourself: it can do anything to your backend. If the phone or the website
can't reach it after a restart, run `./threadbear setup` again: it starts
the backend, or makes it again on the same data. docs/self-hosting.md in the
repo has how it's put together, backing up, upgrading and taking it out.

### On this Mac only, without Tailscale

If you won't use the phone, or you can't use Tailscale (your work keeps
you on its tailnet, and Tailscale only has one on at a time), add
`--local`:

```sh
./threadbear setup --self-hosted --local --yes
```

It's the same backend in Docker, on this Mac's `127.0.0.1` only: nothing
on your network or any tailnet can reach it, and Tailscale isn't needed
at all. That leaves out more:

- **No iPhone app.** Nothing off this Mac can reach the backend, so
  Settings › Phone & browser says why rather than showing a code, and
  there are no notifications.
- **No second Mac** joining this one.
- **The website and share links only open on this Mac**, at
  `http://127.0.0.1:13211`. "Anyone with the link" means anyone using
  this Mac.
- **Nothing works while your Mac sleeps** or Docker is stopped, as with
  the tailnet one.

The dashboard is at `http://127.0.0.1:16791`, with the same admin key.
Running setup again keeps it local. If you get Tailscale later and want
the phone, run `./threadbear setup --self-hosted --tailnet`: it moves the
backend onto your tailnet and your threads stay. `--local` moves it back.

### At your own address

If your phone already reaches your Mac your own way (Caddy, nginx,
cloudflared, your own Tailscale or a VPN), give setup the two https
addresses your proxy answers on, and Threadbear leaves the rest to you:

```sh
./threadbear setup --self-hosted --yes \
  --url https://threadbear.example.com \
  --site-url https://threadbear-site.example.com
```

The backend stays on this Mac's `127.0.0.1`, and your proxy points `--url`
at `127.0.0.1:13210` and `--site-url` at `127.0.0.1:13211` (your website
and share links). Setup checks both work, from this Mac and from inside
Docker, before it carries on, and says which one didn't. Then pair your
phone from Settings › Phone & browser as usual. That needs iPhone 1.0.5 or
later.

The limits are the tailnet way's, except that Tailscale isn't needed:
**no phone notifications**, **nothing works while your Mac sleeps**, Docker
is stopped or your proxy is down, and share links open for **whoever can
reach your address**. `--local` or `--tailnet` move it off your address,
and your threads stay. Take your proxy's rules for both addresses down
first: setup can't do that for you, and it stops, naming the address,
while either still reaches the backend from this Mac.

## Patches: your own buttons and panels
<!-- slug: patches -->

<!-- picture: /clips/patches.mp4 alt: A patch adds a button or a panel to a project's threads -->

A patch adds buttons and panels (beside the thread) for the threads in a
folder. A button can run a command, like your tests, send the agent a
message you use a lot, or open a web page in your browser (App Store
Connect, say), without adding anything to the thread. The web page
buttons show on the iPhone too, over the message box, and open there. A panel can show a web page, such as your app
running on localhost, or a list. A link in a panel's page to another site
opens where your web links' Default says (the browser, or over the panel
with a Back), so the patch's page stays put; if the page moves on to
another of its own pages, a Back shows in the panel's bar, and Reload goes
back to where it started. Closing a panel, or picking another thread,
pauses anything it's playing. A panel's address can carry the thread
it's beside, as `{threadId}` (docs/protocol/patches.md, "Patches"), and an agent
that has changed a panel's page can have it load afresh as it shows you
(`show_panel` with `reload`). The new desktop app shows a panel's page, and the Convex dashboard, with the same
Chromium it draws diagrams with: until that's installed the panel asks for
it (Install web runtime, or Not now) and has Open in browser, and there a
link to another site always opens in your browser.

On a Mac, click a Simulator picture panel to open Simulator. A patch can use
an application's icon, an icns file or a PDF for its buttons and panels too.

Buttons sit on shelves along the top edge of the message box, one shelf
for each place they came from, starting with the same badge the panels
wear (below). Buttons that don't fit go under a "+2" (or however many)
at the end, which lists them.

Each panel has a button on the right-hand rail, with a small badge in its
bottom corner (and on the panel's tab) saying where it came from: the project's own icon when it came
from one of the thread's projects' folders, a folder for the thread's own
folder, a house for `~/.threadbear`, a wrench for a tool patch (one that's
there because of a file, like a video's `.tandem`) and a box for one built
into Threadbear. Hover over the button to see it in words.

A thread in a Convex project gets a built-in Convex panel: the
deployment's own dashboard (data, functions, logs and so on), with a menu
to switch between Development and Production. It uses the Convex CLI on
the thread's Mac, signed in to your Convex account. If that isn't signed
in, the panel says so and has a Sign in to Convex button: your browser
opens Convex's page to approve it (for a thread on another of your Macs,
it opens on the Mac you're at), and the dashboard loads by itself once you
have. If the signed-in account can't reach the project, Sign in as someone
else switches to one that can.

A patch can also show in only some of a folder's threads: those in a
project, or with a word in their title. Ask for that when you ask for the
panel ("only in my acceptance testing threads").

You don't need to write one yourself. In a thread in that folder, ask
"add a button that runs the tests" or "add a panel showing localhost:3000",
and the agent writes the patch into the folder's `.threadbear/patches/`
(the format is under "Patches" in `docs/protocol/patches.md`, in your
Threadbear folder). It shows up within a few seconds. A patch is code that runs on your
Mac, so keep only the ones you'd be happy to run.

Patches only run once you've trusted them. One the agent writes for you in
that folder is trusted as it wrote it (in a Claude thread on Full access;
one written in Ask before edits or Auto, or by Codex, waits for you),
though not if you turn the edit down, not a change made to it afterwards
some other way, and not other files it uses that changed some other way.
A small edit to a patch you hadn't trusted yet (one from a repo you
cloned, or a pull) doesn't trust it either: the rest of it isn't the
agent's. One that arrives some other way, in a repo you've
cloned or with a `git pull`, waits: a "patch to trust" chip shows above the
composer, and Patches lists it as "Not run yet" (or "Changed since you
trusted it") with a Trust button. Open its file to read it first, and
any files it imports or runs (a setup script, say), which Patches lists
under it: a change to one of those asks again too. When it changed since you
trusted it, Patches says which files changed, in which commits (or that
it isn't committed), and shows the changed lines, so a re-ask after your
own merge reads differently from code you've never seen. In a folder
last trusted before Threadbear changed how it checks patches (on 6 Nov),
it says it can't tell what changed. A patch that loads code
by a name it works out as it runs (`import(name)`) can't be checked, so it
never runs and Patches says why: import its files by a plain path instead.
The same goes for a command a patch runs by itself, a worktree's setup or
teardown, or a hook: it has to be one script in the repo with plain words
after it, like `./scripts/setup.sh --fast`, so trusting the patch covers
what runs.
One like `cd scripts && ./setup.sh` or `npm run setup` never runs, and
Patches says to put the steps in a script. What that script runs in turn
(another script, a package.json script by name, a Makefile) is covered
too, so a change to any of them asks again. A script that runs something
worked out as it runs (`eval`, `bash -c "$x"`, `curl ... | sh`, a command
in a variable) can't be covered, so a setup or hook naming it never runs,
and Patches says which line. So does a chip's command: a patch with an
action that can't be followed all the way (or a `python3` one) never runs,
and pressing a chip checks the files again first, so one that's changed
since you trusted it doesn't run and asks in Patches. A patch's badges and list panels run their
commands from the folder holding its `.threadbear/`, and only while it's
the version you trusted, with only the files trusting it covered. A
thread's worktree counts as its repo, and patches that were already in
your folders when this arrived are trusted for you. Patches you trusted
before 6 Nov 2026 are asked about once more: the way their trust is
worked out changed, so the old yes can't carry over.

A repo's own Claude Code settings are asked about the same way. When its
`.claude/settings.json` or `.claude/settings.local.json` has hooks, a
status line or another command Claude runs by itself, or environment
settings (`env`, which can change where Claude sends your messages and
login, like `ANTHROPIC_BASE_URL`, or which keys it uses), or its
`.mcp.json` starts an MCP server, or one of its agents, skills or slash
commands has hooks or MCP servers of its own, Claude starts without those
until you trust them: the chip says "Hooks to trust", Patches lists
"claude-settings" with the files and the scripts they run, and the
thread says they were skipped. In a thread set to "Ask before edits" or
Auto, its permission rules that let tools run without asking (allow
rules, extra folders, a default mode, the sandbox, a skill's
`allowed-tools`) wait too, and Patches lists what they'd allow; in Full
they change nothing, so they don't ask there. The rest of the folder's
settings (deny rules, a model) and its CLAUDE.md still apply; its own
skills, agents and slash commands, and CLAUDE.md files in the folders
above it, wait too. Trust them and your next message starts Claude afresh
with all of it, carrying on the same conversation. A change to them, or
to a script they run, asks again; ones the agent writes for you on Full
access are trusted as it wrote them, as patches are. Ones that run something Threadbear can't
check (a package by name with `npx`, `python -m`, a formatter or test
runner that loads its own config and tests, a script that imports code by
a name worked out as it runs), or allow Threadbear's own tools,
can't be trusted at all, and Patches says why. Settings with none of this
in them never ask.

A project's page has its own panels and Patches on the right-hand rail:
the ones its folders give its threads.

## Changing Threadbear itself
<!-- slug: changing-threadbear -->

<!-- picture: /clips/fork.mp4 alt: Ask a thread to change Threadbear itself, and it lands on your own copy -->

You can change Threadbear by asking a thread to. Start one in your
Threadbear folder (your fork, the copy setup ran from) and say what you'd
like different. If a setting or a patch would do it, the agent uses that
rather than changing Threadbear's own code, so Mike's updates come in
cleanly later. A thread there gets its own copy of the code to work in, and when
the change is ready and its checks pass, the agent lands it on your `main`
branch. When several threads are ready together, their landings wait for
one another and can be checked together against the latest landed work.
The landing's progress message says when it is already being checked with
its group, with a rough estimate of the time left in that check or a note
that it is taking longer than usual.
If the group has landed and the command is still waiting for the landing
lock, the message says it has already landed.

If a check of several landings fails, the command that ran it shows the end
of its output straight away, then runs only what failed on each landing by
itself to find the one that broke it. That one is told at once and left out,
and the rest are checked again together. When that can't tell, it checks
smaller groups instead.
The agent can read any logs named there while those checks run, but must wait
for the command to finish before editing the worktree it's using.

The same goes for settings. The Settings window (⌘,) shows what's set and has
controls for only a few common things, and the ⓘ beside each section says
what it is. Click it to read it, or hover for a quick look. The bear with a
plus at the end of each section's heading (and Start a thread about this,
under the ⓘ's words) opens a new thread with the section named in its box,
so you can ask an agent to change it or explain it. Most settings live in `runner.json` in Threadbear's data folder,
and an agent can change them or set up anything more involved, so ask one
first. If you edit it by hand and leave it broken (a trailing comma, say),
the runner won't start until it's fixed, and never writes over it: `tb logs`
names the file and what's wrong.

Settings has its sections down a sidebar on the left, like System Settings,
and each one scrolls when it's taller than the window. Drag the window wider
or taller for more room.

When something goes wrong, the app says so in an orange bar at the top of
the thread, and the runner's own warnings show there too (a message it
couldn't start, an update to `main` that failed). Investigate on that bar starts a thread in your Threadbear
folder with the error, the thread it came up in, what you were doing and
which versions you're on, and asks it to find the cause and fix it. The new
thread opens and the bar goes once it's made.

Updates then arrive by themselves. Within a few minutes of a change landing
on `main`, the runner builds it and switches over without cutting off any
thread that's partway through a turn, and the app builds its new version in
the background. The app swaps in the new version when you next quit it, when
you choose Restart to Update in the Threadbear menu, or by itself when you've
left it alone for a few seconds. The window stays where it is
while it does: the new version opens the same thread underneath and takes
over, so you shouldn't see it go.

Whatever's typed in a message box and not sent is kept through a quit or a
restart, and so are your own answers in a card's Something else boxes, so
neither holds the new version back: it swaps in and puts them back in their
boxes. A few things still hold it: a picture still being copied into a box,
an earlier message you're editing, a message still on its way, and words on
the new-thread page (its folder and account choices aren't kept). When something else typed does (a
title being renamed, say), the Updates panel names the box and the thread,
with a button that opens the thread. While a runner restart waits for threads to finish, a quiet line at
the top says how many it's waiting for, and hovering it names them. A message
you send while the runner is restarting waits under its bubble and goes once
the new one is up. If one can't be sent, its words go
back in that thread's box and a note above the thread says which thread,
until you dismiss it.

If the runner hits an unexpected error, it restarts itself the same way, so
you may see that restart line with no update behind it; busy threads finish
their turns first, as in an update. If that happens three times in ten
minutes it stops restarting for it and says so in the error bar, and carries
on until an hour goes by without another.

A runner update lets a job a thread left running in the background finish, for
up to about 4 hours after the thread's last turn. If one goes on longer it's
stopped, and the thread gets told and wakes by itself to check on it, so you'll
see it pick back up rather than go quiet.

The new desktop app follows `main` too, from the
checkout you installed it from. Its build takes every core of a Mac for
minutes. It waits for ten minutes with no changes to the desktop app before
asking for a build, so several landings close together become one update.
The ten minutes count from when `main` last moved, so quitting and opening
the app again doesn't start them over, and the build starts as soon as
they're up. A new version that's ready but can't go in yet (say you've left
words typed in a card) gets replaced: once it has waited ten minutes and
`main` has moved past it, the app builds the newer one, and the waiting
version stays in place until the new one is made.
If the build Mac is busy or has tests or timings queued, it tries again
later, trying the other Macs on your build list first. While changes keep
landing or those Macs stay busy, the update keeps waiting. A build already
running finishes before anything else uses that Mac.
It builds on another Mac you name for it (`updateBuildHosts`
in `runner.json`, a list of ssh hosts) and only the finished app comes
back. That Mac's build is what yours runs next, so name one you trust as
much as this one: a check host (`checkHosts`) only runs tests and never
builds the app by being one. With neither set it builds on this Mac in the
background. With check hosts set but no update build host it builds
nothing, and its Updates panel says to name one. With update build hosts
set but none answering, the panel says it's waiting for a Mac and it tries
again later. When that Mac is busy with other builds and tests, the panel
names it and says how long it's been busy, then, once the build has taken
its place in that Mac's queue, how many runs are ahead of it and how long
it's waited; once it starts, how long it's been building. A new version waiting to go in that a Mac off that list built
is thrown away. An app installed before 12 Nov 2026 built its updates on a
check host, so it can't update itself past then: install it again once
(`desktop/scripts/install-app.sh`, with `--build-on <host>` to build on a
Mac you choose). The new version goes in when you quit it, when you choose Restart to
Update in its app menu or Updates panel, or by itself once nothing's typed
and nothing's open that a restart would close (a menu, a dialog, a drag, a
field you're typing in another of its windows, a Terminal shell running
something or with words at its prompt, a web panel you've used in the last
minute): straight away while you're in another app, and with it in front
once you haven't touched the Mac for five seconds. "Typed" is any box: a
message, a card's own answer, a project's goals being edited, a picture
still coming in. It opens again on the list and the thread you had, behind
the app you're in if that's where you were, with every thread you'd
scrolled up in where you left it rather than at its end. On a Mac its
window stays where it is: the new version opens the same thread underneath
and takes over, as the Swift app's does.

The Updates icon on the left-hand rail shows what's on its way for the app,
the runner and the Claude Code and Codex tools. While the app's next
version (or a runner release) is building, a short arc goes round the
icon; once the app's next version is ready to restart into, the icon turns
green. A grey dot means something else is in progress and will sort itself
out, and a coloured dot means it needs you. Right-click it for a quick
Restart to Update. Threads waiting for the app to restart onto what they
landed are listed in the panel's App section, each saying whether its fix
is in the version that's building, in the one that's ready, or still to
come in the next build; click one to open it.

A few parts of the runner (the program that keeps it going, and the small
Threadbear Runner app macOS names in its privacy prompts) don't come with
its updates. When one of them is behind, the runner updates it by itself,
in the background. That stops nothing: the new ones are put in place and
take over the next time your Mac restarts or you log in again, and until
then the Updates panel says they're waiting for that. If it doesn't work,
the panel says why, with Try again and the log. A failed one isn't tried
again by itself until a newer version comes along.

### Mike's updates

Your copy is a fork of Mike's, and it doesn't change unless you say so.
About once a day Mike publishes a release. Your runner checks for new ones
every few hours, and only trusts a release signed with the key setup saved
and already on Mike's main (not one signed on a side branch). Mike can
replace that key through a signed release: a new key counts a week after a
release signed with the old one names it, and a key he revokes stops
counting at once. When there's one your copy doesn't have, the Updates icon
gets a coloured dot, and its From Mike section shows what's new: the
biggest changes as cards from the [What's new page](https://threadbear.mikecann.app/changelog),
each with its picture or clip (hover one for what it does, click it to see
it on the page), how many things were added, changed and fixed, with See
all for the whole page, and how many important fixes there are (hover for
which). A release the page doesn't have yet, or any release when the page
can't be reached, is listed in plain words instead, and older releases
without notes show their commit subjects. Bring Them In starts a thread that works out how Mike's
changes merge with your own, without checking anything out, tells you what
they bring and how it would settle any clashes, and asks. Nothing from the
release runs until you say go: then it merges, runs the checks and lands. Let's Discuss starts one that
talks the release through first, what's in it and what it means for your
own changes, and changes nothing until you say. From a thread, `tb updates`
says the same, and `tb updates check` looks again now.
Once a release is in, the thread offers to send Mike a short check-up: which
release came in, how far behind you were, and how often Threadbear's own errors
came up, and how long your landing checks took, with anything of yours only
counted. You see the whole draft first,
and it goes as a comment on a public Discussion on Mike's repo, under your
GitHub login, only if you say Send. Never ask turns it off
(`"releases": { "checkup": false }` in `runner.json`).

You don't have to remember to look. Every 5 days, if Mike has a release
your copy doesn't have, Threadbear starts that thread for you: it reaches
your inbox with what's new and one question, whether to go ahead, and runs
nothing from the release before you answer. There's only ever one; if you leave it open,
a newer release goes into the same thread. A release Mike marks important
(a security or data fix) starts one straight away. Change how often, or
turn it off, in Settings (⌘,) > General > Updates from Mike.

### Moving to the new desktop app

A new install gets the new desktop app straight away: `./threadbear setup`
builds and installs it (`~/Applications/Threadbear GPUI.app`), never the old
Mac app. This part is for a Mac that already has the old one.

The Mac app is being replaced by a new desktop app that looks and works the
same and runs on Windows and Linux too. The Mac app is frozen now: it no
longer gets new features, and everything new goes into the new app. There's
no date yet for when it stops being updated; once there is, the window and
the card below say it too.

The Mac app tells you about it once when it starts, in a window with Not now
and Start using the new app, and keeps a card at the top of the sidebar until
you start or press Later (which hides it for a week); the card's Start using
it brings the window back. Start using the new app starts a thread, "Switch
to the new Threadbear app", in your checkout, and the window closes once it's
made. The card then shows that thread, what it's doing and Open the thread.
The thread builds the new app on your Mac from your checkout's `main` with
`desktop/scripts/switch-from-swift.sh`, at background priority, which takes
20 to 40 minutes the first time and fetches the exact Rust the repo pins
the first time it builds (docs/on-your-mac.md says what that puts where).
It needs about 5 GB free the first time, 4 after, and says so before it starts if
there isn't that much. If something stops
it (Xcode's command line tools, Rust, disk space, a build error) the thread
works out why and fixes it, or asks you. When the app's installed it asks
whether to switch now, with Switch now and Later, and only switches when you
say: it makes the new app the one `threadbear://` links open, and opens it.
If your `runner.json` has check hosts, it builds on a Mac named in
`updateBuildHosts` instead, as its updates do. The Mac app stays installed
and keeps working, and `desktop/scripts/thread-links.sh swift` gives the
links back to it. A few things still only work in the Mac app:
`docs/gpui-port/PARITY.md` lists them. Neither the window nor the card shows
once the new app is installed, except the card for a switch thread that
isn't archived yet.

The first time the new desktop app opens, it
brings across what the Mac app remembers: the zoom, folded sections, the
Inbox's order, the appearance, the panel layout, where the window was, the
ding, Ask before quitting, the activity log switch, what a click on a link
does, how files and folders show, and anything typed in a message box and
not sent. It only fills in what it hasn't got of its own, does it once, and
a note at the top says what came across. It also brings across whether
you finished the Mac app's tour, which of its pages have shown you their
intro, and whether you hid Finish setting up, so you only get the tour there
if you never finished it.

If your copy has changes of your own to the old Mac app (commits to `app/`
or `dock/` that Mike's copy doesn't have), they can't come across by
themselves: the new app is different code, so each has to be rebuilt in it.
The first time the new app connects, Threadbear looks for them, and if there
are any it starts one thread that tells you in plain words what you changed
and asks whether to rebuild them. It changes nothing until you say yes, then
rebuilds each one in `desktop/` and shows you the new app before and after.
Whatever's still to do is in the Updates panel under Not in the new app
yet: Port this starts a thread for that one change, and Dismiss leaves it
out. A change leaves the list once a commit that rebuilds it says so with a
`Ports: <commit>` line at the end of its message (the threads add one), or
once you dismiss it. A change you made in both apps at once (its commit
changed `desktop/` too) is already in the new app, so it isn't listed. In
a copy with no changes of its own there's nothing
to see.

### Light and dark, theme, accent, spacing and font

Settings (⌘,) > General > Appearance is how Threadbear looks, for Threadbear
alone. Each change shows straight away and stays picked the next time the
app opens.

- **Light or dark**: three pictures. Same as the Mac (Same as Windows on
  Windows) follows the system's own setting: System Settings > Appearance,
  or Settings > Personalization > Colors > Choose your mode on Windows.
- **Theme**: three pictures. Stock is the usual look and what you get
  until you pick another. Harbor is a charcoal frame with blue-slate code
  blocks and a sea-blue accent, and has a light and a dark side, so it
  follows Light or dark above. Squall is blue-grey all through with a
  steel-blue accent. It's always dark, so the other two looks are dimmed
  while it's on; your choice there is kept for when you change theme. A
  theme colours everything: the window, cards, menus, code and the
  Terminal. Both are Wayne Sutton's.
- **Accent**: the colour of buttons, ticks, the outline of a card that
  needs you and the tint behind your own messages. The first dot is the
  Mac's accent (blue elsewhere), or the theme's own while Harbor or Squall
  is on; the rest are bear colours (Fur, Honey, Berry, Rose, Plum, Sky,
  Moss and Slate), from Wayne Sutton's fork. A bear colour goes over a
  theme's accent too.
- **Spacing**: Roomy, or Compact for shorter sidebar rows and less room
  around each of your messages, so more fits.
- **Text size**: Smaller, Normal or Larger, the same as View > Zoom
  (⌘= and ⌘-), which goes further either way.
- **Font**: Allo (the default, [a free font](https://allo.joshpuckett.me)
  that comes with the app), System, Serif or Monospaced for everything but
  code, which keeps its own.

The website has the same three themes in its settings menu (the gear),
under the light and dark choices. It remembers the one you pick on that
browser.

On Mac, Increase Contrast in System Settings > Accessibility > Display
also strengthens text and borders. It is read at launch and when you return
to Threadbear.

### A background picture

To put a photo of yours behind a new thread, turn on Background picture
in the main window in Settings (⌘,) > General > Look and choose one. It's
off until you do. The photo's bottom edge sits on the window's bottom edge
as you resize it; drag it in Position to show another part, and Bottom puts
it back. Fade lays the window's own colour over it so text stays
readable in light and dark, and Show behind open threads too puts it behind
every thread's transcript as well, a little more faded. The picture is
copied, scaled down, into Threadbear's data folder (`looks/`), so the
original can move; Remove deletes the copy. An animated GIF works too, here
and as a page's background: it plays behind the window, is copied as it is
rather than scaled down, and holds still when Reduce Motion is on in System
Settings > Accessibility. The Librarian, Scheduled and
Projects pages have their own backgrounds and layouts (The librarian,
Schedules and Projects).

### A ding when a thread is done

To hear when a thread finishes, turn on Play a sound when a thread
finishes in Settings (⌘,) > General > Notifications. It's off until you
do. A soft ding then plays whenever a thread on this Mac stops working:
its reply is done, it's waiting for your answer, or it hit an error. It
plays wherever you are, in Threadbear or another app, and stays quiet for
a thread you stopped yourself and for one you're talking with in Voice,
which reads the reply out anyway. Several threads finishing at once make one
ding. Pick Chime, Bell or Droplet (each plays as you pick it), set the
volume, and press Play to hear it again.

### Talking with a thread

This is an experiment, off until you turn it on (see Experiments): there's
no button, no download and no microphone prompt while it's off.

Press the microphone button beside Send to talk with the thread out loud
(Settings > Voice > Button icon picks its icon: a microphone, a filled
one, a stage microphone or a waveform).
It's on a new thread's composer too: there, what you say first starts the
thread, and you carry on talking in it.
It turns red while Voice is on. Say what you want as you would to a
person; when you pause for about a second it goes to the thread as your
message, and the agent's reply is read out. You can keep talking while it
works (what you say goes into the turn it's on, as typing does), and if
you talk over it, it stops to listen. It's the same thread with the same
memory, tools and approvals. When it needs your OK it asks out loud, and
you can just say "yes", "go ahead", "no" or "always allow"; anything
longer ("no, use the other folder") goes to it as your message, as
typing would. Your answer only counts once it has finished reading the
request out and stopped talking: if another one comes in first, or you
answer before it's done, nothing is answered and it reads the newest one
to you. It only takes an answer by voice for a request it can read out
whole, a plain one-line command. Anything more (a longer command, an edit,
a plan, another Mac asking to start a thread) it names and leaves for you
to answer on screen, as you do question cards.
While Voice is on, the bear sits over the message box with what you're
saying beside it: it perks up while it listens, nods along as you talk,
looks up with thought dots while it works, and talks along with its
voice. Code and tables aren't read out; it
tells you they're on screen. Press the button again, or go to another
thread, to end Voice.

Use Voice in Settings (⌘,) > Voice turns it off, which takes the
microphone button away everywhere and turns off Voice's keys. Use with Notes, under it, lets you talk
into a note (see Notes); it's off until you turn it on. The same tab picks who it sounds like (Emily, Mike and about
twenty more), how fast it talks, the language, and the microphone. Play
a Sample tries a voice. The first time you use Voice, Threadbear
downloads its voice engine (Kyutai's pocket-tts) and a voice model of
about 240 MB, and macOS asks to let it use the microphone. Listening is
done on your Mac by macOS; nothing you say is recorded or kept in a log. On headphones
(AirPods and the like) you can talk over it any time. On the Mac's
speakers you can too, as long as macOS's echo cancellation starts; when
it won't, Voice stops listening while it speaks, so it doesn't hear
itself.

The GPUI app has Voice too, with the same experiment switch. Its
settings are in Settings (⌘,) > Voice & Audio: Use Voice, the button's
icon, Command Mode, the key the shortcuts use and which of them are on,
the voice, speed, language and microphone. The bear doesn't sit over the
message box there: the button's bars move with whoever's talking, and
pulse while it gets ready or the thread works, and a line just above the
message box shows what it heard as you talk. It sends once you pause for
1.5 seconds; Pause before sending in Settings > Voice & Audio makes that
1, 2 or 3. If it can't start, a note over the button says why; click it
to close it. Fn dictation works the same, with its pill just above the
message box. Notes has no dedicated microphone or Voice setting in the
GPUI app; keyboard dictation still works in its text boxes.

### Bears

An experiment (Settings > General > Experiments, Bears). Bears are named
teammates, like Researcher or Writer, each with a persona, a memory, a
face and its own routines. A bear is a folder in
`~/ThreadbearThreads/Bears`: a thread there is that bear. Its `CLAUDE.md`
is its persona and house rules (short replies, drafts before anything is
sent, say what it couldn't check), `memory/` is what it remembers, and
`Bears/memory/` holds facts about you that every bear shares.

The first time bears are listed, Head Bear is made as your chief of
staff: it can hand work to the others. `tb bears head <bear>` gives the
job to another one.

Add the bears skill once (Settings > Skills, where it waits under
Suggested): it teaches every thread, bears included, the `tb bears`
commands, faces and routines. Store skills you've added reach bear threads
like any other thread.

In the desktop app, the paw on a thread's right-hand rail opens Bears &
Tables. Bears and Tables have separate search boxes, selections and action
rows. Click normally to open an item; press **Select** to pick several with
plain clicks. ⌘-click toggles one, Shift-click selects a range within that
section, and **All** or ⌘A selects its visible results. Search changes clear
only that section's selection. **Done** or Esc ends selection.

**New** at the top of each section makes one. A new bear takes a name,
what it does, a personality and an optional photo; a new table takes a
title, a goal, the bears that sit at it, the project it's part of and an
optional photo. A table that's part of a project shows on that project's
page.

Right-click one for Open, Edit, Change in a Thread…, Tables & Projects…
(a bear) or Bears & Project… (a table), Set Photo…, Show in Finder, Copy
Folder Path, Refresh, Rename… or Delete…. Editing a bear opens its
configuration file; editing a table opens its project goals. **Change in a
Thread** asks what should change, then starts a thread that makes the
change with the bears skill and opens it. **Tables & Projects** seats a
bear at tables and picks the projects its new threads join; **Bears &
Project** seats bears at a table and picks its parent project. A bear's
photo is a copy kept in its folder (`photo-<time>.png`, or drop a
`photo.png` there yourself); a table's photo is its project icon. **Delete** in a section's selection bar asks
once about that section's selected items. Deleting a bear moves its folder
to the runner's trash; deleting a table leaves its chat threads and folder.

**Archive** also asks first. Archiving bears pauses their routines, revokes
webhooks and removes their table seats; their folders and threads stay.
Archiving the Head Bear leaves no active head. Choose **Archived** and
**Restore** to unarchive them; routines remain paused and webhooks and table
seats must be added again. Archiving or restoring tables preserves their
folders, threads and seated bears. An action in Bears never includes Tables,
or vice versa.

The rest of what bears do lives in `tb bears`:

- `tb bears new "Researcher" --does "Finds sources" --personality "dry and calm" --face round`
  makes one, with a daily "tidy memory" routine at 4:30. The routine runs
  in Auto for Claude, where the reviewer checks tool calls, or Ask for
  Codex. A bear created by another agent starts its tidy in Ask too.
- `tb bears rename`, `set`, `pin`, `move`, `archive`, `delete`. Renaming
  keeps its memory, routines and old threads. Deleting moves its folder
  to the runner's trash.
- `tb bears away <bear> 3d` hibernates it: its routines skip, webhooks
  hold their deliveries, and a mention gets "away until".
- `tb bears access <bear> --auto` lets its unattended runs (routines,
  webhooks, mentions) go past Ask. Only you can do that: a bear's own
  thread can't. Auto is Claude only: a bear on Codex (`--model gpt-...`)
  set to Auto still asks, and `tb bears access` says so.

Routines, webhooks and mentions share one unattended job per bear. New
work queues after the current turn, and each mention gets its own answer.
A repeating routine skips while its previous run is busy. Work with
different access or a different working folder waits (routines and webhooks), or asks you to retry
(mentions). Chats you start yourself stay separate.

Mentions also respect sandbox boundaries. A sandboxed thread cannot use a
bear to send work outside its sandbox. A routine waits when its chosen
login, pool, model or effort differs from the busy worker, then starts
with its own settings once that worker finishes.

New bears keep `memory/told.md`, a dated record of what they told you, and
`memory/tasks/` for unfinished jobs. They read these before starting again
so they can pick up the work without repeating the same report. These are
instructions for the agent, not a guarantee that every model follows them.
`tb bears refresh-rules` adds these rules to existing bears and updates
their built-in tidies. It preserves your persona text, custom routines
and paused schedules. A bear cannot run it to raise its own access.

An unattended bear can mark an empty run with `end_turn`'s `quiet: true`.
The runner archives it only after checking for unfinished work. Useful
reports, questions, failures and threads you have touched stay visible.
An empty final check cannot hide an earlier report in the same thread.

The separate **Late answers** experiment lets an answer arriving within a
day of a tool card expiring resume the thread. It passes your answer and
the original request to the agent, without replaying the old call or
changing access. The runner supports this now; a button for deliberately
answering an expired card is still planned for the GPUI app.

A bear on Codex can make pictures: Codex's own `image_gen` tool makes real
PNGs (a Designer bear on `gpt-6-astra` made 1024 by 1024 ones on 13 Nov).
That tool writes its picture into `~/.codex/generated_images` without
asking, even in Ask; copying it into the bear's folder asks as usual.

A bear's threads show its icon on its colour in the sidebar, where other
threads show a folder or their project. Right-click one of its threads and
pick Bear Colour to change the colour, or ask any thread to give a bear an
icon (`tb bears set Writer --icon pencil --color "#3E8ED0"`, an SF Symbol
name). The head bear shows a crown until you give it an icon of its own.

Type `@` in the message box and a list of your bears comes up over the
cursor, narrowing as you type: ↑↓ to move, Return or Tab to pick, Esc to
close. Picking one writes its name for you. It only lists your bears for
a thread on this Mac: a thread on another machine hands a mention to that
machine's own bears.

Write `@Researcher` in a message (or `@bear:<id>`), or run
`tb bears ask Researcher "..."` from a thread, and the bear gets the
message in a fresh thread in its folder, in the lower of its mode and
that thread's. Its reply comes back to the thread that asked, as a new
message there, while you watch. At most three bears a message.

With Bear notes on too, a bear you opt in (`tb bears set <bear> --notes both`)
can leave one short cheer or idea under the librarian's line in
Highlights, only when it's earned.

### Where Settings opens

Settings (⌘, or the gear at the bottom of the bar on the left) opens over
the main panel, with its tabs along the top. The thread or page you had
open stays as it was under it: the X at the top right, Esc, the gear again
or picking a thread closes Settings and puts you back. To have Settings in
a window of its own instead, set Open Settings in to Its own window in
Settings > General. The GPUI desktop app always opens it over the main
panel for now.

Over the main panel, Settings widens with the window, up to the width its
rows reach. Its own window can be dragged wider too; each tab still sets
how tall it is.

### The activity log

Turn on Keep an activity log in Settings (⌘,) > General and the app notes
down what you do in it, so the agents working on Threadbear can see how you
really use it: what you click, which threads and pages you open, the menu
items and shortcuts you use, the tooltips you stop to read, and when you
send a message or answer a prompt. It never notes what you type or the
words of a message, only which button, row or item it was, by its tooltip
or its name in the menu (so clicking a thread's row puts that thread's
title in it). An agent reads it with `tb activity`, or
`tb activity --summary` for what's used most.

It's off until you turn it on. It's kept in the `activity` folder of
Threadbear's data folder, a file a day, and each file is deleted after 30
days. Threadbear never sends it anywhere, though once an agent has read
some of it, that's part of its thread like anything else it reads. The new
desktop app keeps the same log in the same files,
with its own switch in its Settings, and marks its lines as its own.

### Keeping the Mac awake

While any thread is working, and for three minutes after, your Mac won't
go to sleep on its own, so a long turn, or one you started from your
phone, isn't cut off halfway. The screen still dims and locks as usual.
It stops below 15% battery when you're not plugged in, and closing the
lid on battery still puts the Mac to sleep. It's on unless you turn off
Keep the Mac awake while threads work in Settings (⌘,) > General > Sleep.

## Privacy
<!-- slug: privacy -->

<!-- picture: /img/bear-patch.webp alt: What goes where, and what stays on your Mac -->

Your threads are mirrored to a Convex deployment in your own Convex account,
which setup made: the thread list, the text of each message, one line for
each thing an agent did, and your logins' emails and usage. Anything that
looks like a secret is scrubbed out of the text on the way up, which helps
but isn't a guarantee. Full sync is off to start. Turned on,
your phone, the website and your other machines can show each tool's output and
diffs too, even when your Mac's asleep: its output, its input and diffs go up, after
the same scrubbing, and are kept 30 days; output from files and commands that
usually hold secrets stays on your Mac, and so do tool screenshots. Send tool output to Convex in Settings
(⌘,) > General > Full sync, or `tb settings full on|off`, sets it any time, from the
runner's next start. `tb status` estimates how much of Convex's Free plan you're using and
warns before you get near its limits. Pictures you paste into
a message go up as they are, so your website can show them, and nothing can
scrub a secret out of a picture: `"sync": { "pastedPictures": false }` in
`runner.json` keeps them on your Mac. A photo you send from your phone passes
through that deployment on its way to the Mac and is deleted once the Mac has
it; one you picked and never sent goes after a day. A message from the phone
waits up to two weeks for its Mac to take it.
A thread you share can be read by anyone with its link, pictures included,
and only that thread. The pictures are uploaded only while it's shared,
and only real pictures from its own media folder: your attachments, the
screenshots its tools took, and pictures the agent copies in there on
purpose (it's told where, as `THREADBEAR_MEDIA_DIR`). One an agent points
at anywhere else, the thread's own folder or a temp folder included,
stays on your Mac and the page shows its caption instead.
Notifications to your phone go through a small relay Mike runs, because
Apple only delivers notifications for his app through his key. The text is
locked with a key only your phone and your deployment have, so the relay
never sees what a notification says. The activity log (see
[Changing Threadbear itself](#changing-threadbear)) is off unless
you turn it on, and it stays on your Mac.

## Getting help
<!-- slug: getting-help -->

<!-- picture: /img/bear-wave.webp alt: Ask a thread first: it knows where this guide is -->

- Ask a thread. Its agent knows where this guide is and can read it, or
  look at the code and tell you how something works.
- Ask a thread to run `tb status`, which says what the runner is doing:
  its version, the threads that are busy, and any restart or update on the
  way.
- `threadbear` in Terminal opens the app, and quitting it with ⌘Q is always
  safe, since your threads keep running without it. Closing its window
  (⌘W or the red button) quits it too, without asking. Quit asks first, with
  a Don't ask again box; Settings > General > Quitting turns asking back
  on. If the app says it can't reach the runner, click Start runner.
- Threadbear > About Threadbear says which commit your copy was built from,
  along with a few numbers about your threads.

## The tour and Finish setting up
<!-- slug: finish-setting-up -->

<!-- picture: /docs-img/setup.webp alt: Finish setting up, bottom left: it ticks itself off as you go -->

The first time the new desktop app opens, a few small cards come up before
the app, one decision each. Return goes on, and each card has Back and
Skip; Esc puts them all away:

- **Your agents** lists the Claude Code and Codex logins it found on this
  Mac. One that isn't signed in has Sign in…, which opens Terminal on its
  own sign-in. Add makes room for another Claude login (a second
  subscription, say) and opens its sign-in the same way; the row spins
  until it's signed in. A thread moves to it when the first runs low. For
  a Claude login, the same Terminal window then makes a year-long token
  (Year-long token…, below), so you sign in once a year rather than every
  28 days.
- **Where threads start** shows the folder new threads open in, with
  Change…, and whether GitHub's CLI (`gh`) is signed in.
- **Check in from anywhere** shows a code to scan with the iPhone app, and
  ticks itself once your phone's paired. It only comes up when your
  Threadbear has a Convex deployment for a phone to reach.
- **A few things worth turning on**: review by another lab, keeping the
  Mac awake while threads work, Connectors (opened once you're done) and
  the app's look. Done opens a new thread, ready for your first message.

Help > Set Up Threadbear brings the cards back. Help > Take the Tour shows
you round the window, one part at a time: the window dims, one part lights
up, and a card beside it says what it is, with an ⓘ that opens its page of
these docs. A few made-up sample threads give it something to point at: a
sample's row with its star, moon and box out, your views down the left,
Highlights, moving a panel, and an example patch's button, Roll a die (it
lives in `~/ThreadbearThreads/Welcome`). The samples are deleted when the
tour ends. A Mac that joined a Threadbear you already use (setup's
`--existing`) skips both, as does one that finished the tour in the Mac
app.

While that first thread works, Finish setting up shows what's left after
setup:

- **Start your first thread**, which shows your Claude Code or Codex login
  works. If there's no login on the Mac yet, it says how to add one.
- **Pair your iPhone**, so you can ask questions and steer your threads
  while you're out (see [Your phone and a browser](#phone-and-browser)).
- **Open your website**, to check on your threads from any browser.

Each item ticks itself off once it's done, and the list's icon on the
left-hand rail goes once they all have, with confetti if the list is open
when the last one ticks. Hide this list puts it away, and
Help > Finish Setting Up brings it back. The phone and website items only
show when your Threadbear has a Convex deployment for them to use.

## Tasks (experiment)

Tasks keeps a simple shared list for your account. Turn on Tasks in Settings >
Experiments, then use Tasks below Notes or Command-7.

It opens on the list. Type a task in the box at the top and press Return; Shift-Return
starts a new line, and anything after the first line becomes the task's note.
Paste a Markdown list (`-`, `*`, `1.` or `- [ ]` items) and Return adds one task
per item, in order, at the top; the box says how many first. Indented items become
sub-tasks of the item above, ticked `- [x]` items go straight to Done, and
headings are skipped. Text that starts with a plain line stays one task with the
list as its note. On the Mac, a board column's add box takes a list too.
Tick a task's box to mark it done: it leaves the list, and Undo brings it back.
Show Done Tasks in Display (or the "done" link at the bottom) shows them again.
The list runs Building, Shaping, Observing, On Deck, Backlog and Paused, each a
section you can fold, with Done and Canceled after them.

Switch to Board for columns. A new board has eight:

| Column | What goes there |
|---|---|
| Backlog | Things you're considering but not pursuing yet |
| On Deck | Decided to do it, not started |
| Shaping | Working out the problem, the scope and the approach. Active work |
| Building | Being built. New tasks land here |
| Observing | Shipped, still checking results or refining. Optional: a task can go straight from Building to Done |
| Done | The goal is met, or the work is called complete. Track follow-ups as new tasks |
| Paused | Stopped for now |
| Canceled | Not doing it. Counts as closed, like Done |

Cards show the
title, the priority, the due date (red once it's passed, amber on the day), a
sub-task count like 1/3, a note mark and the project; drag a card to move it, or use + on a column
to add one there. Add a task at the foot of a column does the same. Click a
column name to rename it, or Add column to make another. Renaming keeps its
tasks and completion behavior. Delete a column from its ⋯ menu on the Mac, or
from its rename box on the web. Its tasks move to another column that means the
same thing (Canceled's go to Done), or to Building if none is left, and the
confirm says where. Every column can be renamed or deleted except Building,
because new tasks land there. Drag column headings, list sections and project
groups to reorder them; the board and list keep their own orders. Narrow
windows and phones scroll columns sideways within the board. In the Mac app,
Display > Wrap Columns puts the columns on as many rows as they need instead, so
every column shows without scrolling sideways.

Each task's ⋯ menu opens it, adds a note, moves it to a project, sets its
priority or due date (Today, Tomorrow, End of week, Pick a date… for any day on a
calendar, with a time if you tick At a time, or Clear), sets its status
or asks before deleting it (with Undo afterward). Send to thread starts a new
thread with the task title, notes, pictures (up to 6) and project. It shows Starting while queued,
then View thread; repeated clicks reuse the same request. Projects are optional,
and their pages link to their tasks. Display picks a project, groups by status or project, sets the order,
and makes the view you're in the one Tasks opens with. The sliders button next
to Display offers Light and Dark appearance, a photo with Fade and Position
sliders, and Notifications. A dot on it means a photo is set. (Which view
Tasks opens in is under Display.)

A task with a due time reminds you when it comes. Notifications sets how:
**Notify me on this Mac** shows an amber alert with a bell in Threadbear's
window, with your Mac's alert sound (click it to open the task; it stays until
you close it) and a Threadbear notification that opens the task too. A build
signed only on this Mac (ad-hoc) isn't allowed macOS notifications: add a free
Apple Development certificate (Xcode > Settings > Accounts), turn Threadbear
on in System Settings > Notifications, and reopen it. With Focus on, macOS
holds banners back; the in-window alert still shows.
**Remind** picks when (when it's due, or 5 minutes to a day before), and
**Also send** can add an email or a Slack message to the address or channel
in **To**. Email and Slack go through your connectors (Gmail or Slack on
claude.ai, or your Connectors gateway): connectors only work inside an agent's
turn, so a short turn in a **Task reminders** thread sends each one. If the
connector isn't connected, that thread says so. One Mac sends each reminder,
even with several running, and a reminder missed while the Mac slept goes out
late, up to a day late. Changing the time reminds again. Due times need
backend API 105.
Text follows the selected theme. On the board, swipe sideways anywhere under the
columns to scroll them.
A task can have a priority (Urgent, High, Normal or Low), a due date (Today,
Tomorrow, End of this week, In one week, or any day from the calendar),
sub-tasks (one level: "1/2 closed" counts the done ones), and links to a web
page, a print, a picture or a thread. List rows and board cards show the
priority and the due date. In the Mac app, a task with a due date also
shows in the Scheduled view under Tasks due, and clicking it there opens it. Activity under a task lists its comments and what
changed and who did it, you or an agent ("set priority to Urgent"); leave a comment in the box
at the bottom. Deleting a task deletes its sub-tasks, and Undo brings them back.
In the Mac app, a task in one of this Mac's projects also lists that project's
latest threads and prints. Agents can set a priority and due date and comment
through their task tools.
In the Mac app a task opens on a page of its own, in place of the list or
board: its title, then Status, Priority, Due date, Project, Created by, Threads
(Send to Thread, or View Thread once it has one) and Links, then its
description, sub-tasks (Add sub-task, then Return), links and attachments (Add
link with an optional title, Attach Print, or Attach image from a picker or by
dropping pictures on the section), what its project has been up to, and its
activity, with All or just Comments. Return sends a comment and Shift-Return
starts a new line. The title and description save when you leave them. Back or
Escape goes back to the list or board, and J and K (or Down and Up) step to the
next and previous task.

Paste a picture into the description (a screenshot, or pictures copied in
Finder) or drop one on it, and it attaches to the task like Attach image does.
The task's pictures show as previews under the description: click one to open
it, or hover and press the x to remove it.

Attach Print opens a search field: type part of a title, then click a result
or press Return for the first match. Escape closes the search.

In the list, Up and Down navigate tasks and Enter opens one. Arrow keys still
edit text normally while a field has focus.

**Favorites.** Choose Add to Favorites in a task's ⋯ menu, click the star on
its page, or press Option-F. The task stays in its own section or column and
also shows under Favorites, at the top of the list and as the board's first
column. Favorites can fold but can't be dragged; move the task in its own
section. Remove from Favorites (or Option-F again) takes it off.

**Keys on the list and board.** These follow Linear's and act on the task under
the pointer, or the one you picked with Up and Down. They work only while no
text field has focus, and any key the app itself uses wins.

| Key | Does |
|---|---|
| C | New task (the quick-add box, or a card in Building on the board) |
| S | Change status |
| P | Set priority |
| Shift-D | Set due date |
| Command-Shift-D | Remove due date |
| Command-Delete | Delete the task (asks first) |
| Shift-R | Rename (opens the task with its title ready) |
| Control-Shift-I | Edit the description |
| Option-F | Add to or remove from Favorites |

S, P and Shift-D open a picker: type to filter, use Up, Down and Return, or
press a row's number. Escape closes it. Linear's keys for assignees, labels,
estimates, templates and reminders aren't here, since tasks don't have those.

In a thread's Highlights, a row's right-click menu has Add to Tasks: it makes a
task named after the row, linking to the page, or to the thread for a file or
commit, and says so when it's in.

Open on web goes to your private `/tasks` page with the usual browser pairing.
Web task edits work while your Mac sleeps. Editing needs a connection; a change
that fails is put back with the reason, and a failed save in the task editor
keeps its draft to retry. Notes remains local and encrypted; Tasks is stored in
your configured Convex backend. Notion project boards stay separate. Ask a
thread to "add a task to review the launch checklist" to use the same collection
through its task tools.

### How tasks work

1. Turn on Tasks in Settings > Experiments. Open it with Tasks below Notes, or
   Command-7.
2. Type a task in the box at the top and press Return. Paste a Markdown list
   to add many at once; indented items become sub-tasks.
3. Switch between List and Board. Tick a task's box to mark it done, or drag a
   card to another column.
4. Open a task to set its priority, due date and time, project and reminders,
   add links, pictures, prints and comments, and see its activity.
5. Send to Thread starts a thread from the task's title and note, in its
   project's folder. Agents in threads can read and change tasks with their
   task tools.
6. Star the tasks you're focused on (Option-F, or Add to Favorites in the ⋯
   menu) to see them together under Favorites. Press S, P or Shift-D on a task
   to change its status, priority or due date from the keyboard.

### How tasks work with Linear

This is optional. Without Linear connected, Tasks looks and works the same, and
none of this shows. Connect Linear in the Connectors panel (Add a service, then
Linear) and it appears.

**Send a task to Linear.** Pick Send to Linear from a task's ⋯ menu, or on its
page. The sheet shows which Linear workspace it goes to. Pick a team, then a
project in that team (or none), check the title and the exact description, and
untick any pictures you don't want to send. Sub-tasks go in as a checklist,
web links go with the issue, and pictures upload to Linear and show in the
description. Prints, threads and anything else on your Mac stay here. Click
Create: the click is your approval, so it doesn't ask again. The sheet
remembers the team and project for that Threadbear project next time.

The task then shows its issue, like `ENG-123 · Todo`. Click it to open the issue.
A task is only ever sent once: a double click, a second Mac or a retry after a
crash links the issue it already made rather than making another.

**Keep it in sync.** "Keep in sync with Linear" is on by default for a task you
send, add or link, and you can turn it off on the task's page or its ⋯ menu.
For tasks you keep in sync, Threadbear copies from Linear:

- status (closing the issue ticks the task done, canceling moves it to
  Canceled, reopening moves it back)
- priority and due date
- the title
- comments, shown read-only in Activity with a Linear mark

It never changes your note, and it never changes anything in Linear. A field
only changes here when it changed in Linear, so your own edit stays until Linear
changes that same field. Threadbear checks every hour, or every 6 hours, 12
hours or once a day as you pick in Tasks settings ("Check Linear for changes"),
and again whenever you open the task or click Refresh. The checks are direct
calls from your Mac, with no model involved, so they cost no tokens. Your Mac
has to be awake: it catches up when it wakes. If the issue is deleted, the task
says "Not found in Linear" and stops syncing.

**Bring Linear issues in.** Add from Linear (in the Tasks toolbar, or "Start
from your Linear issues" on an empty board) opens a search of your Linear
issues: yours and open ones to start, by team or project if you like. Tick the
ones you want, pick a Threadbear project, and Add. Each becomes a task linked to
its issue, with its description copied once into the note, its comments in
Activity, and sync on. Tick "Include sub-issues as sub-tasks" to bring those
too. An issue that already has a task shows "In Tasks" and can't be added
twice. Nothing comes in on its own.

**Link a task you already have.** Link to Linear issue in a task's ⋯ menu takes
an issue key like `ENG-123` or its link. It links the two without changing
either one.

**Unlink** forgets the issue here and never deletes anything in Linear.

Threadbear talks to Linear through Connectors as its own login, which can read
your Linear, upload pictures and create issues, and nothing else. Changing an
existing issue still waits for your approval in Connectors, the same as it does
for agents in threads.
