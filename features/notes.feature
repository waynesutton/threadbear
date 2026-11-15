Feature: Notes
  As someone who keeps ideas and to-dos beside their threads
  I want my own notes in the bar on the left, encrypted on this Mac
  So I can jot things down without them going to an agent or the runner

  Background:
    Given the app is connected to a runner with the demo threads

  # The Swift app's script can't drive its Notes page yet.
  @notes.page @swift-pending
  Scenario: Notes opens over the thread and Esc puts the thread back
    When I open the thread "Make uploads resume after a crash"
    And I press ⌘6
    Then Notes is open
    And thread "demo_upload_resume" is still selected under Notes
    When I press Escape on Notes
    Then Notes is closed
    And thread "demo_upload_resume" is still selected under Notes

  @notes.page @swift-pending
  Scenario: New notes get the next free name and keep what's typed
    When I open Notes
    And I start a new note
    Then the open note is called "Untitled note"
    When I type "Buy milk" in the note
    And I start a new note
    Then the open note is called "Untitled note 2"
    When I rename the open note to "Groceries"
    Then the notes listed are "Groceries, Untitled note"
    When I close Notes with its X
    And I open Notes
    And I pick the note "Untitled note"
    Then the note says "Buy milk"

  @notes.page @swift-pending
  Scenario: Return carries a task list on and a click ticks its box
    When I open Notes
    And I start a new note
    And I type "- [ ] milk" in the note
    And I press Return in the note
    And I type "eggs" in the note
    Then the note says "- [ ] milk\n- [ ] eggs"
    When I click the box on line 1 of the note
    Then the note says "- [x] milk\n- [ ] eggs"
    When I press Return in the note
    And I press Return in the note
    Then the note says "- [x] milk\n- [ ] eggs\n"

  @notes.page @swift-pending
  Scenario: Deleting a note asks first
    When I open Notes
    And I start a new note
    And I press the note's bin
    Then Notes asks "Delete “Untitled note”?"
    When I confirm the delete
    Then the notes listed are ""

  @notes.page @settings.general @swift-pending
  Scenario: Show Notes off hides the button and closes the page
    When I open Notes
    And I start a new note
    And I turn Show Notes off in Settings
    Then the bar has no Notes button
    # Opening Settings already closed Notes, so ⌘6 is what's tried here.
    When I leave the Settings page
    And I press ⌘6
    Then Notes is closed
    When I turn Show Notes on in Settings
    And I open Notes
    Then the notes listed are "Untitled note"
