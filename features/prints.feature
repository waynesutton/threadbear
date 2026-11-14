Feature: Diagram Prints
  As someone keeping diagrams with a project or a thread
  I want to edit their source and keep the previous versions
  So the diagram stays useful after the reply that made it

  # The mock runner seeds two Prints made by "Export last quarter's invoices
  # as CSV" (app/Tools/mock-runner/server.mjs, addDemoPrints): the chart
  # "Invoice totals by month", whose card that thread's reply shows, and the
  # diagram "How the export runs". The @manual flows need a dev runner still.
  @prints.diagram @manual @gpui-pending
  Scenario: Edit a diagram's source and keep the preceding version
    Given a diagram Print called "Task flow" contains Mermaid source
    When I choose Edit Text from that Print's menu
    Then the editor shows its Mermaid source
    When I change the source and save it
    Then the diagram redraws as a new version marked edited by you
    And its preceding version can still be opened and restored

  @prints.diagram @manual @gpui-pending
  Scenario: A malformed source edit keeps the current diagram
    Given a diagram Print called "Task flow" has a saved version
    When I save malformed Mermaid source in its Edit Text editor
    Then an error explains that the diagram could not be rendered
    And the current diagram and its version remain unchanged

  @prints.diagram @manual @gpui-pending
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

  @prints.whenasked @manual @gpui-pending
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

  @prints.list @manual @gpui-pending
  Scenario: Prints show in Highlights and on the rail with their projects
    Given a thread made a Print in the project "Demo Shop" and one in no project
    When I open that thread
    Then its Highlights list both under Prints, one saying "Demo Shop" and one "No project"
    And the rail shows a Prints button under Files with a count of 2
    When I click the Prints button
    Then it lists every Print with its projects
    When I click "Launch notes"
    Then it opens beside the thread

  @prints.todo @manual @gpui-pending
  Scenario: Tick a todo list and move a card on a board
    Given a thread made a todo list Print "Launch checklist" and a board Print "PR board"
    When I open "Launch checklist" beside the thread and click "Write the tests"
    Then it is ticked in a new version and the count of done items goes up
    When I open "PR board" and click a card's arrow to the next column
    Then the card is in that column in a new version
    And clicking in an older version says it changed and changes nothing
