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

  # The frozen Swift driver does not expose this Notes page dump.
  @notes.markdown @swift-pending
  Scenario: Headings get smaller by level
    When I open my notes
    And I make a new note
    And I write this note:
      """
      # Biggest
      ## Middle
      ### Smaller
      Plain text
      """
    Then the note's heading sizes decrease down to plain text

  # The frozen Swift driver does not expose this Notes page dump.
  @notes.markdown @swift-pending
  Scenario: Task clicks save and can be undone
    When I open my notes
    And I make a new note
    And I write this note:
      """
      🐻 Errands
      - [ ] milk
      """
    And I click the note's task "milk"
    Then the saved note ends with "- [x] milk"
    When I undo my note edit
    Then the saved note ends with "- [ ] milk"

  # The frozen Swift driver does not expose this Notes page dump.
  @notes.markdown @swift-pending
  Scenario: Return carries lists on and ends empty items
    When I open my notes
    And I make a new note
    And I write "9) milk" in the note
    And I press Return at the end of the note
    Then the saved note ends with "\n10) "
    When I press Return at the end of the note
    Then the saved note ends with "9) milk\n"

  # The frozen Swift driver does not expose this Notes page dump.
  @notes.edit @swift-pending
  Scenario: Back puts Notes away before navigating
    When I open the thread "Compare account picker designs"
    And I open the thread "Make uploads resume after a crash"
    And I open my notes
    And I go Back from Notes
    Then my notes are closed
    And Notes has left me on "Make uploads resume after a crash"
