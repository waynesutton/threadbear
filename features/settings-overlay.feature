Feature: Settings over the main panel
  Background:
    Given the app is connected to a runner with the demo threads
    And I open the thread "Morning brief"

  @settings.window @swift-pending
  Scenario: Closing Settings returns to the same thread and unsent message
    When I type "Keep this draft" in the composer
    And I press the Settings shortcut
    Then Settings covers the main panel on "general"
    And no separate Settings window is open
    When I press "Close Settings"
    Then Settings no longer covers the main panel
    And the open thread is "Morning brief"
    And the composer says "Keep this draft"

  @settings.window @swift-pending
  Scenario: Escape and the gear close Settings, and choosing a thread leaves it
    When I press the Settings shortcut
    Then Settings covers the main panel on "general"
    When I press Escape
    Then Settings no longer covers the main panel
    When I press the Settings shortcut
    And I press the Settings gear
    Then Settings no longer covers the main panel
    When I press the Settings shortcut
    And I open the thread "Tidy the imports in the uploader"
    Then Settings no longer covers the main panel
    And the open thread is "Tidy the imports in the uploader"

  @settings.keyboard @swift-pending
  Scenario: The Keyboard page works inside the main panel
    When I press the Settings shortcut
    And I press "Keyboard"
    Then Settings covers the main panel on "keyboard"
    When I search the embedded Keyboard page for "archive"
    Then the embedded Keyboard page lists "Archive"
    When I press "Close Settings"
    Then the open thread is "Morning brief"

  @settings.general @swift-pending
  Scenario: Settings can use its own window after reopening the app
    When I open Settings
    And I press "Use a separate Settings window" in Settings
    And I close the standalone Settings window
    And the app is quit and opened again
    And I press the Settings shortcut
    Then Settings shows "general"
    And Settings no longer covers the main panel

  @settings.window @swift-pending
  Scenario: Back dismisses Settings before changing the thread
    When I type "Keep this draft" in the composer
    And I press the Settings shortcut
    And I press Back from Settings
    Then Settings no longer covers the main panel
    And the open thread is "Morning brief"
    And the composer says "Keep this draft"
