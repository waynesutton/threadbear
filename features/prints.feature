Feature: Prints
  As someone keeping what agents make with a project or a thread
  I want each Print listed, opened, edited and versioned where I work
  So it stays useful after the reply that made it scrolls away

  # The mock runner seeds Prints made by "Export last quarter's invoices as
  # CSV" (app/Tools/mock-runner/server.mjs, addDemoPrints and
  # addDemoPrintLists): the chart "Invoice totals by month", whose card that
  # thread's reply shows, the diagram "How the export runs", and in the
  # Threadbear project the doc "Launch notes", the todo list "Launch
  # checklist" and the board "PR board". The GPUI scenarios drive its `print`
  # command and read its dump's `prints`, which the frozen Swift app's driver
  # doesn't have, so they're @swift-pending. The @manual flows need a dev
  # runner, the web runtime (a card's picture, a click in the page itself, a
  # PDF) or a person (Publish's alert).
  @prints.diagram @manual
  Scenario: Edit a diagram's source and keep the preceding version
    Given a diagram Print called "Task flow" contains Mermaid source
    When I choose Edit Text from that Print's menu
    Then the editor shows its Mermaid source
    When I change the source and save it
    Then the diagram redraws as a new version marked edited by you
    And its preceding version can still be opened and restored

  @prints.diagram @manual
  Scenario: A malformed source edit keeps the current diagram
    Given a diagram Print called "Task flow" has a saved version
    When I save malformed Mermaid source in its Edit Text editor
    Then an error explains that the diagram could not be rendered
    And the current diagram and its version remain unchanged

  @prints.diagram @manual
  Scenario: A diagram remains accessible without a project
    Given a diagram Print was created in a thread with no project
    When I reopen its card in that thread
    Then I can open the diagram and edit its source
    And assigning it to a project lists the same Print there

  # The Swift dump doesn't say what the switch reads (AppDump.swift, frozen).
  @prints.whenasked @swift-pending
  Scenario: Settings has Only make Prints when I ask while Prints is on
    Given the app is connected to a runner with the demo threads
    When I open Settings on "general"
    Then Settings says threads make Prints without asking first
    When I press "Have threads ask before making a Print you didn't ask for" in Settings
    Then Settings says threads make Prints only when I ask

  @prints.whenasked @swift-pending @pages-off
  Scenario: With Prints off there is no Prints switch in Settings
    Given the app is connected to a runner with the demo threads
    When I open Settings on "general"
    Then Settings has no Prints switch

  # Off, a long task starts straight away with no progress Print and no question.
  @prints.progress @swift-pending
  Scenario: Settings can turn progress Prints for long tasks off
    Given the app is connected to a runner with the demo threads
    When I open Settings on "general"
    Then Settings says long tasks get progress Prints
    When I press "Let threads keep a progress Print on long tasks. Off, they start the work without one" in Settings
    Then Settings says long tasks start without a progress Print

  @prints.progress @manual
  Scenario: With progress Prints off, a long task starts without one
    Given Prints is on and "Progress prints for long tasks" is off in Settings
    When I ask a thread for a task with more than five steps
    Then it starts the work without making a progress Print
    And no card asks me about one
    And other Prints, like a chart I ask for, are still made

  @prints.whenasked @manual
  Scenario: Only make Prints when I ask holds back a Print nobody asked for
    Given Prints is on and "Only make Prints when I ask" is on in Settings
    When a thread goes to make a Print after a message that asks for none
    Then an approval card asks "Make the print" with its title first
    When I choose No on that card
    Then no Print is made and the thread is told I said no
    When I ask a thread to "make a chart of this data"
    Then it makes the Print without a card

  # The Swift dump doesn't list a reply's Print cards (AppDump.swift, frozen).
  @prints.card @swift-pending
  Scenario: A reply shows a Print as a card with its title, kind and version
    Given the app is connected to a runner with the demo threads
    When I open the thread "Export last quarter's invoices as CSV"
    Then the reply shows the Print card "Invoice totals by month" saying "Chart · v1"
    And the card offers Open and Refresh

  @prints.card @swift-pending @pages-off
  Scenario: With Prints off a print block stays code
    Given the app is connected to a runner with the demo threads
    When I open the thread "Morning brief"
    And a reply with a print block streams in
    Then the reply shows its print block as code

  # The Swift app opens a reply's Print in a window of its own; the side pane
  # is the GPUI app's (and the guide's) Open. Its dump has no card to press.
  @prints.pane @swift-pending
  Scenario: A Print opened from its card follows new versions in the side pane
    Given the app is connected to a runner with the demo threads
    When I open the thread "Export last quarter's invoices as CSV"
    Then the reply shows the Print card "Invoice totals by month" saying "Chart · v1"
    When I click Open on the Print card
    Then the side pane shows version 1 of "pg_invoice_totals"
    When the thread saves version 2 of "pg_invoice_totals"
    Then the side pane shows version 2 of "pg_invoice_totals"
    And the reply shows the Print card "Invoice totals by month" saying "Chart · v2"

  @prints.list @manual
  Scenario: Prints show in Highlights and on the rail with their projects
    Given a thread made a Print in the project "Demo Shop" and one in no project
    When I open that thread
    Then its Highlights list both under Prints, one saying "Demo Shop" and one "No project"
    And the rail shows a Prints button under Files with a count of 2
    When I click the Prints button
    Then it lists every Print with its projects
    When I click "Launch notes"
    Then it opens beside the thread

  # Tried in the GPUI app with the web runtime on 14 Nov: a click in the page
  # (webPanel click, a person's gesture) ticked "Write the tests" in v2 and
  # the pane moved on. The automated scenario below goes through the same
  # handler without the runtime.
  @prints.todo @manual
  Scenario: Tick a todo list and move a card on a board
    Given a thread made a todo list Print "Launch checklist" and a board Print "PR board"
    When I open "Launch checklist" beside the thread and click "Write the tests"
    Then it is ticked in a new version and the count of done items goes up
    When I open "PR board" and click a card's arrow to the next column
    Then the card is in that column in a new version
    And clicking in an older version says it changed and changes nothing

  @prints.project @swift-pending
  Scenario: A project's page lists its Prints, newest first
    Given the app is connected to a runner with the demo threads
    When I open the project page for "Threadbear"
    Then its Prints section lists "Launch notes Doc · v3"
    And its Prints section lists "Launch checklist Todo list · v1"
    And its Prints section doesn't list "Invoice totals by month"

  @prints.list @swift-pending
  Scenario: The thread's Prints are in Highlights, and every Print on the rail
    Given the app is connected to a runner with the demo threads
    When I open the thread "Export last quarter's invoices as CSV"
    Then Highlights lists the Print "Launch notes" in "Threadbear"
    And Highlights lists the Print "Invoice totals by month" in "No project"
    And the rail shows Prints with a count of 5

  @prints.menu @swift-pending
  Scenario: Edit Text saves a doc as a new version marked edited by me
    Given the app is connected to a runner with the demo threads
    When I open the project page for "Threadbear"
    And I edit the text of the Print "Launch notes" to "# Launch notes\n\nShipping Thursday."
    Then its Prints section lists "Launch notes Doc · v4 · now · edited by you"

  @prints.diagram @swift-pending
  Scenario: A malformed diagram edit keeps the current version and says why
    Given the app is connected to a runner with the demo threads
    When I open the project page for "Threadbear"
    And I edit the text of the Print "How the export runs" to "not a diagram"
    Then the Print editor says "Couldn't render the diagram"
    And the Print "How the export runs" reads "Diagram · v1"

  @prints.todo @swift-pending
  Scenario: A tick in a todo list beside the thread saves a version the pane follows
    Given the app is connected to a runner with the demo threads
    When I open the thread "Export last quarter's invoices as CSV"
    And I open the Print "Launch checklist" beside the thread
    And I click the todo list's item 1 in version 1
    Then the side pane shows version 2 of "pg_launch_checklist"
    And the Print "Launch checklist" reads "Todo list · v2 · now · edited by you"

  # Needs the web runtime and the Save dialog. Tried in the GPUI app on 14 Nov
  # through the `print` command's `pdf`: US Letter, light look.
  @prints.pdf @manual
  Scenario: Download a Print as a PDF
    Given a Print "Launch notes" is open on a project's page
    When I choose Download as PDF from its menu and pick where
    Then a US Letter PDF of it in the light look is saved there and shown in the Finder

  # Publishing asks first in a system alert, which a scripted app never
  # answers, and needs sync on: not tried in the GPUI app yet.
  @prints.publish @manual @gpui-pending
  Scenario: Publish a Print and copy its link
    Given sync is on and a Print "Launch notes" isn't published
    When I choose Publish from its menu and say Publish
    Then its link is copied and its row says "published"
    And its menu offers Copy Link, Open on the Web and Unpublish

  # Several at once (Wayne, 17 Nov): ⌘-click and Shift-click in the rail's
  # Prints, Highlights' Prints or a project's page pick them, Esc lets them
  # go, and the bar or a picked row's menu acts on them all.
  @prints.list @swift-pending
  Scenario: ⌘-click and Shift-click pick several Prints, and Esc lets them go
    Given the app is connected to a runner with the demo threads
    When I "⌘"-click the Print "Launch notes"
    And I "Shift"-click the Print "How the export runs"
    Then the Prints picked are "Launch notes, Invoice totals by month, How the export runs"
    When I "⌘"-click the Print "Invoice totals by month"
    Then the Prints picked are "Launch notes, How the export runs"
    When I press Esc over the Prints
    Then the Prints picked are ""

  @prints.list @prints.menu @swift-pending
  Scenario: Deleting picked Prints asks once and takes them all away
    Given the app is connected to a runner with the demo threads
    When I "⌘"-click the Print "Invoice totals by month"
    And I "⌘"-click the Print "How the export runs"
    And I delete the picked Prints
    Then Delete asks once about "Invoice totals by month, How the export runs"
    When I say Delete
    Then no Print is called "Invoice totals by month"
    And no Print is called "How the export runs"
    And the Prints picked are ""

  # Return says Delete and Esc says Cancel (Wayne, 18 Nov): Delete is the
  # filled button, and the confirm takes the keyboard even from a Print's
  # page beside the list. That page holding the keyboard needs the web
  # runtime; the dump's `keyboard` shows who has it.
  @prints.list @prints.menu @swift-pending
  Scenario: Return deletes the picked Prints and Esc keeps them
    Given the app is connected to a runner with the demo threads
    When I "⌘"-click the Print "Launch checklist"
    And I delete the picked Prints
    Then Delete asks once about "Launch checklist"
    And the app has the keyboard
    When I press Esc in the Delete confirm
    Then nothing is asking about deleting Prints
    And a Print is called "Launch checklist"
    When I delete the picked Prints
    Then Delete asks once about "Launch checklist"
    When I press Return in the Delete confirm
    Then no Print is called "Launch checklist"

  @prints.list @prints.menu @swift-pending
  Scenario: Picked Prints come out of a project together
    Given the app is connected to a runner with the demo threads
    When I "⌘"-click the Print "Launch checklist"
    And I "⌘"-click the Print "PR board"
    And I take the picked Prints out of "Threadbear"
    Then the Print "Launch checklist" is in "No project"
    And the Print "PR board" is in "No project"
    And the Print "Launch notes" is in "Threadbear"

  # A click in the page itself needs the web runtime and a real pointer.
  # Tried by hand: the rules are threadbear_web's preview_rules
  # (`opens_in_browser`), tested there.
  @prints.pane @manual
  Scenario: A web link in a Print opens in the browser
    Given a Print with a link to "https://example.com" is open beside the thread or in a window
    When I click the link
    Then example.com opens in my browser
    And the Print stays where it was

  @prints.list @swift-pending
  Scenario: Sidebar search scopes visible selection and reversible archive
    Given the app is connected to a runner with the demo threads
    When I open the "Prints" bulk sidebar
    And I search the "Prints" sidebar for " LAUNCH "
    Then the "Prints" sidebar has 0 selected and 2 visible
    When I use "Select" in the "Prints" sidebar
    And I click the sidebar item "Launch notes"
    And I click the sidebar item "Launch checklist"
    Then the "Prints" sidebar has 2 selected and 2 visible
    And the sidebar keeps the current thread open
    When I use "Delete" in the "Prints" sidebar
    And I press Esc in the Delete confirm
    Then the "Prints" sidebar has 2 selected and 2 visible
    When I use "Archive" in the "Prints" sidebar
    Then the "Prints" sidebar has 0 selected and 0 visible
    When I use "Archived" in the "Prints" sidebar
    Then the "Prints" sidebar has 0 selected and 2 visible
    When I use "All" in the "Prints" sidebar
    And I use "Restore" in the "Prints" sidebar
    And I use "Active" in the "Prints" sidebar
    Then the "Prints" sidebar has 0 selected and 2 visible

  @prints.list @swift-pending
  Scenario: Changing a Print search drops hidden selections before Delete
    Given the app is connected to a runner with the demo threads
    When I open the "Prints" bulk sidebar
    And I use "Select" in the "Prints" sidebar
    And I use "All" in the "Prints" sidebar
    And I search the "Prints" sidebar for "missing title"
    Then the "Prints" sidebar has 0 selected and 0 visible
    When I search the "Prints" sidebar for "Invoice"
    And I use "All" in the "Prints" sidebar
    Then the "Prints" sidebar has 1 selected and 1 visible
    When I use "Delete" in the "Prints" sidebar
    Then Delete asks once about "Invoice totals by month"
    When I press Return in the Delete confirm
    Then the "Prints" sidebar has 0 selected and 0 visible
    And a Print is called "Launch notes"

  @prints.list @prints.menu @swift-pending
  Scenario: A filtered sidebar menu drops Prints selected on another surface
    Given the app is connected to a runner with the demo threads
    When I open the "Prints" bulk sidebar
    And I search the "Prints" sidebar for "Invoice"
    And I "⌘"-click the Print "Invoice totals by month"
    And I "⌘"-click the Print "Launch notes"
    Then the "Prints" sidebar has 1 selected and 1 visible
    When I open the filtered Print row menu
    Then the Prints picked are ""
    And a Print is called "Launch notes"
