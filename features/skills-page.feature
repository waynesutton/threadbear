Feature: Shared agent skills
  Background:
    Given the app is connected to a runner with the demo threads
    And I open the thread "Morning brief"

  @skills.page @swift-pending
  Scenario: Skills covers the thread and closes without losing its draft
    When I type "Keep my draft" in the composer
    And I open the Skills page
    Then the Skills page lists "pele"
    When I press "Close Skills"
    Then the Skills page is closed
    And the composer says "Keep my draft"

  @skills.page @swift-pending
  Scenario: Write and edit a skill shared by every login
    When I open the Skills page
    And I press "New Skill"
    And I fill in a skill named "my-style" with "Use short clear sentences."
    And I press "Save skill"
    Then the Skills page lists "my-style"
    When I press "Edit my-style"
    Then the skill editor contains "Use short clear sentences."

  @skills.page @swift-pending
  Scenario: Import, change automatic use and remove a skill
    When I open the Skills page
    And I enter the skill link "https://github.com/example/writing"
    And I press "Import skill"
    Then the Skills page lists "writing"
    When I press "Always on writing: On"
    Then the skill "writing" is only used by its slash command
    When I scroll down the Skills page
    And I press "Remove writing"
    Then the Skills page does not list "writing"

  @skills.page @swift-pending
  Scenario: A bad import keeps the page usable and explains why
    When I open the Skills page
    And I enter the skill link "not a link"
    And I press "Import skill"
    Then the Skills page explains "GitHub"
    And the Skills page lists "pele"

  @skills.page @swift-pending
  Scenario: A duplicate name leaves the written instructions available
    When I open the Skills page
    And I press "New Skill"
    And I fill in a skill named "pele" with "Keep these instructions."
    And I press "Save skill"
    Then the Skills page explains "already"
    And the skill editor contains "Keep these instructions."

  @skills.page @swift-pending
  Scenario: General can disable the shared store without deleting skills
    When I open the Skills page
    Then the Skills page lists "pele"
    When I open Settings
    And I press "Enable shared agent skills" in Settings
    Then the shared skill store is disabled
    When I press "Enable shared agent skills" in Settings
    Then the shared skill store is enabled
