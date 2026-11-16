Feature: Private notes
  Background:
    Given the app is connected to a runner with the demo threads

  # The frozen Swift driver does not expose this Notes page dump.
  @notes.edit @swift-pending
  Scenario: Write, rename and return to a note
    When I open my notes
    And I make a new note
    And I rename the note to "Shopping"
    And I write "Remember the milk" in the note
    Then my notes show "Shopping" with "Remember the milk"
    When I leave my notes
    And I open my notes
    Then my notes show "Shopping" with "Remember the milk"

  # The frozen Swift driver does not expose this Notes page dump.
  @notes.edit @swift-pending
  Scenario: More room to write
    When I open my notes
    And I make a new note
    And I fold the notes list
    Then the notes list is folded
    When I leave my notes
    And I open my notes
    Then the notes list is folded

  # The frozen Swift driver does not expose this Notes page dump.
  @notes.edit @swift-pending
  Scenario: The Threads button leaves Notes even when Inbox was already selected
    When I open my notes
    And I click the Threads button from Notes
    Then my notes are closed
