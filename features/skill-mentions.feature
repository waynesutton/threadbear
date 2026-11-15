Feature: Skill names in messages
  # The frozen Swift dump does not expose its highlighted text ranges.
  Background:
    Given the app is connected to a runner with the demo threads
    And the thread "Rename the Settings tabs" works in a folder of its own
    When I open the thread "Rename the Settings tabs"
    And the listed deslop skill has a file

  @composer.slash @swift-pending
  Scenario: A listed skill turns blue before any palette is opened and links to its file
    When I type "Please use /deslop " in the composer
    Then the composer highlights the skill "/deslop"
    When I press Return
    Then the transcript shows my message "Please use /deslop"
    When I open the sent skill "/deslop"
    Then the pane is showing the file ".claude/skills/deslop/SKILL.md"
    And the pane's page has the words "Skill preview fixture"

  @composer.slash @swift-pending
  Scenario: Longer words and paths stay ordinary editable words
    When I type "src/deslop /deslop-more and /threadbear:deslop " in the composer
    Then the composer highlights only the skill "/threadbear:deslop"

  @composer.slash @rich.threadchips @swift-pending
  Scenario: A skill link still works beside another thread's chip
    When I send "Please use /deslop and [Tidy imports](thread:demo_tidy_imports)"
    Then my message "Please use /deslop" shows the chips "thread: Tidy imports"
    When I open the sent skill "/deslop"
    Then the pane is showing the file ".claude/skills/deslop/SKILL.md"
    And the pane's page has the words "Skill preview fixture"
