@native-tasks @swift-pending
Feature: Tasks
  Tasks share one account collection across the desktop and browser. The list
  is the default: type a task, press Return, tick it off. The board has
  Backlog, Todo, Doing and Done, and everything that has an order can be dragged.

  Background:
    Given the app is connected to a runner with the demo threads

  @tasks.page
  Scenario: Command 7 opens Tasks in the default list view
    When I press ⌘7
    Then Tasks shows the "list" view

  @tasks.page
  Scenario: Typing a task and pressing Return adds it to the list
    When I open Tasks
    And I type "Reply to the YC batch" in the quick-add box and press Return
    Then Tasks contains "Reply to the YC batch"
    And the task "Reply to the YC batch" is in "todo"

  @tasks.page
  Scenario: A saved task appears in the board
    When I open Tasks
    And I add the task "Review the launch checklist"
    And I choose the Tasks Board view
    Then Tasks shows the "board" view
    And Tasks contains "Review the launch checklist"

  @tasks.page
  Scenario: The default view changes only when I set it
    When I open Tasks
    And I choose the Tasks Board view
    Then Tasks shows the "board" view
    When I close and reopen Tasks
    Then Tasks shows the "list" view
    When I choose the Tasks Board view
    And I make this the default Tasks view
    And I close and reopen Tasks
    Then Tasks shows the "board" view

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
  Scenario: A failed quick-add keeps the typed task for retry
    When I open Tasks
    And the next task save will fail
    And I type "Keep the quick draft" in the quick-add box and press Return
    Then Tasks shows a save error
    And the quick-add box contains "Keep the quick draft"
    When I close and reopen Tasks
    And I retry the quick-add
    Then Tasks contains "Keep the quick draft"
    And the quick-add box contains ""

  @tasks.page
  Scenario: Editing a title preserves a change from another device
    When I open Tasks
    And I add the task "Review launch"
    Then Tasks contains "Review launch"
    When I edit the task "Review launch"
    And another device moves the task "Review launch" to "doing"
    And I save the task title as "Review launch copy"
    Then Tasks contains "Review launch copy"
    And the task "Review launch copy" is in "doing"

  @tasks.page
  Scenario: Ticking a task off hides it from the list until Undo
    When I open Tasks
    And I type "Check the preview" in the quick-add box and press Return
    Then Tasks contains "Check the preview"
    When I complete the task "Check the preview"
    Then the task "Check the preview" is in "done"
    And Tasks offers to undo "Done: Check the preview"
    And the list does not show "Check the preview"
    When I press Undo
    Then the task "Check the preview" is in "todo"

  @tasks.page
  Scenario: Deleting a task can be undone
    When I open Tasks
    And I add the task "Old idea"
    Then Tasks contains "Old idea"
    When I edit the task "Old idea"
    And I delete the visible task
    Then Tasks does not contain "Old idea"
    When I press Undo
    Then Tasks contains "Old idea"

  @tasks.page
  Scenario: Tasks move between statuses, and columns and sections reorder
    When I open Tasks
    And I type "Ship the board" in the quick-add box and press Return
    And I drag the task "Ship the board" to "doing"
    Then the task "Ship the board" is in "doing"
    When I choose the Tasks Board view
    And I drag the "done" column onto "backlog"
    Then the board's columns are "done, backlog, todo, doing"
    When I choose the Tasks List view
    And I drag the "backlog" section onto "doing"
    Then the list's sections are "backlog, doing, todo, done"

  @tasks.page
  Scenario: Tasks can be closed in a narrow window
    When I open Tasks
    And I narrow the Tasks window
    And I close and reopen Tasks
    Then Tasks shows the "list" view
