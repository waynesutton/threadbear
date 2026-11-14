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

  # The GPUI app catches the click in its web view and makes the call (the
  # automated scenario below goes through the same handler), but nobody has
  # clicked in a real page yet: Wayne's Mac mini has no web runtime installed.
  @prints.todo @manual @gpui-pending
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

  # Needs the web runtime to draw the page, and the save dialog.
  @prints.pdf @manual @gpui-pending
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
