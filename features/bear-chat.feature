Feature: The sidebar bear's chat
  Background:
    Given the app is connected to a runner with the demo threads

  @sidebar.bearchat @swift-pending
  Scenario: The bear starts a local draft and remembers its chat
    When I press "Chat with the bear"
    Then the bear draft uses its own local folder
    And the bear's instructions have been prepared
    When I type "Remember that I like tea" in the composer
    And I press Return
    Then the bear chat is open
    When I open the thread "Morning brief"
    And I press "Chat with the bear"
    Then the same bear chat is open

  @sidebar.bearchat @swift-pending
  Scenario: Hiding the button keeps its instructions
    When I press "Chat with the bear"
    Then the bear's instructions have been prepared
    When I open Settings
    And I press "Show the bear in the sidebar" in Settings
    Then the bear's button is hidden
    And the bear's instructions have been prepared
    When the app is quit and opened again
    Then the bear's button is hidden

  @sidebar.bearchat @swift-pending
  Scenario: The bear's menu opens chat and its instructions folder
    When I open the bear's menu
    Then exactly one context menu offers "Open Chat | Show the Bear's Folder"
