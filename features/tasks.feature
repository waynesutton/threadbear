@native-tasks @swift-pending
Feature: Tasks
  Tasks share one account collection across the desktop and browser.

  Background:
    Given the app is connected to a runner with the demo threads

  @tasks.page
  Scenario: Command 7 opens Tasks in the default board view
    When I press ⌘7
    Then Tasks shows the "board" view

  @tasks.page
  Scenario: A saved task appears in the board
    When I open Tasks
    And I add the task "Review the launch checklist"
    Then Tasks contains "Review the launch checklist"

  @tasks.page
  Scenario: The default view changes only when I set it
    When I open Tasks
    And I choose the Tasks List view
    Then Tasks shows the "list" view
    When I close and reopen Tasks
    Then Tasks shows the "board" view
    When I choose the Tasks List view
    And I make this the default Tasks view
    And I close and reopen Tasks
    Then Tasks shows the "list" view

  @tasks.page
  Scenario: A failed save keeps my draft when I leave Tasks
    When I open Tasks
    And the next task save will fail
    And I add the task "Keep my launch checklist"
    Then Tasks shows a save error
    When I close and reopen Tasks
    And I retry saving the task draft
    Then Tasks contains "Keep my launch checklist"

  @tasks.page
  Scenario: Editing a title preserves a change from another device
    When I open Tasks
    And I choose the Tasks List view
    And I add the task "Review launch"
    Then Tasks contains "Review launch"
    When I edit the task "Review launch"
    And another device completes the task "Review launch"
    And I save the task title as "Review launch copy"
    Then Tasks contains "Review launch copy"
    And the task "Review launch copy" is done

  @tasks.page
  Scenario: Completion and Undo use the same task
    When I open Tasks
    And I choose the Tasks List view
    And I add the task "Check the preview"
    Then Tasks contains "Check the preview"
    When I complete the task "Check the preview"
    Then the task "Check the preview" is done
    When I delete the visible task
    Then Tasks does not contain "Check the preview"
    When I undo the task deletion
    Then Tasks contains "Check the preview"
    And the task "Check the preview" is done
