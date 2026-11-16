Feature: Markdown in my messages
  Background:
    Given the app is connected to a runner with the demo threads
    And I open the thread "Morning brief"

  @rich.inline @rich.blocks @swift-pending
  Scenario: My formatting renders while my line breaks stay visible
    When I send the formatted message
      """
      **Note**
      line two
      - one
      - two
      then this
      """
    Then my rendered message contains "Note"
    And my rendered message does not contain "**"
    And my message keeps a hard break after "**Note**"
    And a plain line after my list starts a new paragraph

  @rich.inline @swift-pending
  Scenario: A message without formatting stays plain
    When I send the formatted message
      """
      snake_case_names and 2*3 are fine
      """
    Then my latest message uses plain text
