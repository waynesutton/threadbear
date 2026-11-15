Feature: Skills store
  As someone who wants every agent to know the same things
  I want my skills on a page of their own, for every thread and login
  So I can add, write and switch them without editing folders by hand

  Background:
    Given the app is connected to a runner with the demo threads

  # The Swift app's script can't drive its Skills page.
  @skills.page @skills-store @swift-pending
  Scenario: ⌘5 opens Skills with the runner's skills, and Esc puts it away
    When I press ⌘5
    Then Skills is open
    And the skills listed are "pele"
    And "anatomy" is suggested
    When I press Escape on Skills
    Then Skills is closed

  @skills.page @skills-store @swift-pending
  Scenario: A new skill is saved under its slash name
    When I open Skills
    And I write a new skill called "Brand Voice" that says "Write warmly."
    Then the skills listed are "brand-voice, pele"

  @skills.page @skills-store @swift-pending
  Scenario: Adding a suggested skill moves it to your skills
    When I open Skills
    And I add the suggested skill "anatomy"
    Then the skills listed are "anatomy, pele"
    And nothing is suggested

  @skills.page @skills-store @swift-pending
  Scenario: Always on can be turned off for one skill
    When I open Skills
    And I turn off Always on for "pele"
    Then "pele" is not Always on

  @skills.page @swift-pending
  Scenario: With the experiment off there's no Skills
    Then there is no Skills button
    When I press ⌘5
    Then Skills is closed
