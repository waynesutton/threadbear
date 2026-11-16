Feature: Settings
  As someone who wants Threadbear set up my way
  I want the runner's settings in one window, each saying what it's for
  So I can change the few that matter here, and ask an agent for the rest

  Background:
    Given the app is connected to a runner with the demo threads

  @settings.window @swift-pending
  Scenario: Settings stays inside the main window and Back restores the thread
    When I open the thread "Make uploads resume after a crash"
    And I type "Keep this draft" in the composer
    And I open Settings
    Then Settings is inside the main window
    When I open Settings on "keyboard"
    Then Settings is inside the main window
    When I leave the Settings page
    Then Settings is closed and thread "demo_upload_resume" is still open
    And the composer says "Keep this draft"

  @settings.window @swift-pending
  Scenario: Escape leaves Settings without closing the main window
    When I open the thread "Make uploads resume after a crash"
    And I open Settings
    And I press Escape
    Then Settings is closed and thread "demo_upload_resume" is still open

  @settings.window @settings.keyboard @swift-pending
  Scenario: Escape cancels recording before leaving Settings
    When I open Settings on "keyboard"
    And I click the key for "archive" in Settings
    And I press the keys "escape" in Settings
    Then Settings isn't listening for a key
    And Settings shows "keyboard"

  @settings.general @swift-pending
  Scenario: A failed Instructions save survives leaving Settings
    When I open Settings
    And the next Instructions save will fail
    And I replace Instructions with "Keep this instruction" and immediately leave Settings
    Then the Instructions save failure is shown in the main window
    And an automatic update waits for the Instructions edit
    When I open Settings
    Then Instructions still says "Keep this instruction"
    And the Instructions edit is saved after retrying

  @settings.window
  Scenario: Settings opens on General, and another section can be picked
    When I open Settings
    Then Settings shows "general"
    When I open Settings on "models"
    Then Settings shows "models"

  # A scripted Swift app's arrow keys go only to a text box (`arrow`), so the
  # list's arrows are checked on the GPUI app.
  @settings.window @swift-pending
  Scenario: Up and down move through Settings' sections
    When I open Settings
    And I press the down arrow in Settings
    Then Settings shows "models"
    When I press the down arrow in Settings
    Then Settings shows "pools"
    When I press the up arrow in Settings
    Then Settings shows "models"

  # The Swift app's Settings is an AppKit form: a scripted click on one of its
  # controls starts a tracking loop that waits for a real mouse-up, so the
  # command never answers. Those scenarios are driven on the GPUI app only.
  @settings.general @swift-pending
  Scenario: Turning off Keep the Mac awake changes the runner's setting
    When I open Settings
    Then Settings says the Mac is kept awake
    When I press "Stop your Mac sleeping on its own while a thread is working" in Settings
    Then Settings says the Mac isn't kept awake

  # Threads ask before a Print the message didn't ask for (Wayne, 11 Nov).
  @settings.general @swift-pending
  Scenario: Turning on Only make Prints when I ask changes the runner's setting
    When I open Settings
    Then Settings says Prints are made without asking
    When I press "Have threads ask before making a Print you didn't ask for" in Settings
    Then Settings says Prints are only made when I ask

  # The Swift app is frozen, so it never gets this switch (AGENTS.md).
  @settings.general @swift-pending
  Scenario: Turning on full sync changes the runner's setting
    When I open Settings
    Then Settings says full sync is off
    When I press "Send tool output and diffs to your Convex deployment" in Settings
    Then Settings says full sync is on

  @settings.models @swift-pending
  Scenario: A private folder can be taken off
    When I open Settings on "models"
    Then Settings lists the private folder "~/dev/clients/acme"
    When I press "Stop keeping this folder private" in Settings
    Then Settings lists no private folders

  @settings.general @swift-pending
  Scenario: A reviewer can be taken off the list of who reviews code
    When I open Settings
    Then Settings lists the reviewers "codex, claude"
    When I press "Take Codex off the list" in Settings
    Then Settings lists the reviewers "claude"
