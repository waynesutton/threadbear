Feature: Bear tiles in the sidebar
  As someone with named bears
  I want each bear's threads to show its own icon and colour
  So I can tell a bear's thread from any other at a glance

  # The visual details are checked with the Bears mock and dark screenshots.
  @bears.tile @manual
  Scenario: A bear's thread shows its icon on its colour
    Given a bear called "Writer" with the icon "pencil" and the colour "#3E8ED0"
    When a thread runs in Writer's folder
    Then its sidebar row shows a white pencil on blue where a folder would be
    And hovering the tile says "Writer"

  @bears.tile @manual
  Scenario: The head bear shows a crown until it has an icon
    Given the head bear has no icon of its own
    Then its threads' tiles show a crown on its fur colour

  @bears.mention @manual
  Scenario: Pick a bear from the @ list
    Given a bear called "Writer"
    When I type "Hey @wr" in the message box
    Then a list over the cursor offers "@Writer" with what it does
    When I press Tab
    Then the message reads "Hey @Writer "
    And the list closes

  @bears.tablepage @manual
  Scenario: A table's project page lists its bears
    Given a table called "Head table" with Head Bear and Writer
    When I put a Designer thread in the Head table project
    Then Designer sits at the table
    And the Head table project page lists Head Bear, Writer and Designer under "Bears at this table"

  @bears.tablepage @manual
  Scenario: A project lists the bears of the tables among its Parts
    Given a project called "Launch" with the table "Launch crew" as one of its Parts
    And Writer sits at Launch crew
    When I open the Launch project's Project tab
    Then it lists Writer under "Bears at Launch crew"

  @bears.colour @manual
  Scenario: Pick a bear's colour from its thread's menu
    Given a thread in Writer's folder
    When I right-click its row and pick Bear Colour, then Moss
    Then the banner says Writer's colour is changing
    And every Writer thread's tile turns green

  @bears.tile
  Scenario: Named bear threads replace their folder tiles
    Then the bear thread "demo_landing_pages" has the tile "bear:Writer"
    And the bear thread "demo_tidy_imports" has the tile "bear:Head Bear"

  @bears.colour
  Scenario: A bear thread offers its colour menu
    When I open the visible Designer thread menu
    Then the Designer menu offers Bear Colour

  @bears.mention
  Scenario: Complete a bear mention without sending the message
    When I type a Writer mention in a local thread
    Then the bear picker offers Writer
    When I complete the bear mention with Tab
    Then the message box reads "Hey @Writer "

  @bears.mention
  Scenario: Escape keeps a bear mention list closed
    When I type a Writer mention in a local thread
    Then the bear picker offers Writer
    When I dismiss the bear picker
    Then the bear picker stays closed

  @bears.tablepage
  Scenario: A table page names the bears sitting at it
    When I open the Head table project page
    Then the table page shows Head Bear and Writer

  @bears.mention @remote-threads
  Scenario: Another runner does not offer this runner's bears
    When I type a Writer mention in another runner's thread
    Then the bear picker stays closed

  @bears.tablepage
  Scenario: A project shows the bears of a table among its Parts
    When I open the parent of Launch crew
    Then the project page shows Writer at its table

  @bears.tablepage
  Scenario: Adding a bear's thread to a table seats the bear
    When I put Designer's thread in Head table
    And I open the Head table project page
    Then the table page shows Designer

  @bears.mention
  Scenario: Shift-Return keeps its newline instead of picking a bear
    When I type a Writer mention in a local thread
    Then the bear picker offers Writer
    When I press Shift-Return over the bear list
    Then the message box keeps the typed bear name and a new line
    And the bear picker stays closed

  @bears.mention
  Scenario: Losing focus closes the bear list
    When I type a Writer mention in a local thread
    Then the bear picker offers Writer
    When I move focus out of the bear message box
    Then the bear picker stays closed

  # The panel needs a dev runner with bears and tables on, which the mock
  # doesn't have yet, so these were checked by hand with dev-app.sh's
  # {"cmd":"bears"} commands (bears_panel.rs) and screenshots.
  @bears.panel @manual @swift-pending
  Scenario: Bears and tables have a panel of their own on the right rail
    Given bears and tables are on, with Head Bear, Writer and the table "Launch crew"
    When I open Bears & Tables from the right rail
    Then it lists Head Bear (head bear) under Pinned, Writer under Bears and Launch crew under Tables
    When I click Writer
    Then Writer's newest thread opens

  @bears.panel @manual @swift-pending
  Scenario: Rename a bear from its menu
    When I right-click Writer in Bears & Tables and pick Rename…
    And I type "Scribe" and press Rename
    Then the row says Scribe at once
    And if the runner refuses, it says Writer again and the error shows

  @bears.panel @manual @swift-pending
  Scenario: Delete several bears and tables with one sheet
    When I ⌘-click Writer and Shift-click Launch crew in Bears & Tables
    Then the rows from Writer to Launch crew are picked and a bar says "Delete 3"
    When I right-click one of them
    Then the menu offers "Delete 3 items…"
    When I pick it
    Then one sheet lists the bears and tables that go
    When I press Delete
    Then they leave the list at once, and any the runner refuses come back with the error

  @bears.panel @manual @swift-pending
  Scenario: Esc lets go of the picked rows
    Given two bears are picked in Bears & Tables
    When I press Esc
    Then none are picked
