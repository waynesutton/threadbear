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
  Scenario: Escape closes a task editor and leaves Tasks open
    When I open Tasks
    And I add the task "Keyboard task"
    Then Tasks contains "Keyboard task"
    When I edit the task "Keyboard task"
    And I press Escape in Tasks
    Then the Tasks editor is closed
    And Tasks shows the "list" view

  @tasks.page
  Scenario: Cancelling deletion keeps the task and editor
    When I open Tasks
    And I add the task "Keep this task"
    Then Tasks contains "Keep this task"
    When I edit the task "Keep this task"
    And I cancel deleting the visible task
    Then Tasks contains "Keep this task"
    And the Tasks editor is open

  @tasks.page
  Scenario: Arrow navigation opens the next task without entering text
    When I open Tasks
    And I add the task "First keyboard task"
    Then Tasks contains "First keyboard task"
    When I add the task "Second keyboard task"
    Then Tasks contains "Second keyboard task"
    When I edit the task "First keyboard task"
    And I press Escape in Tasks
    And I press "down" in Tasks
    And I press "enter" in Tasks
    Then the Tasks editor is open
    And the selected task is "Second keyboard task"

  @tasks.page
  Scenario: Sending a task shows its associated thread afterward
    When I open Tasks
    And I add the task "Start this task"
    Then Tasks contains "Start this task"
    When I edit the task "Start this task"
    And I send the visible task to a thread
    Then the task "Start this task" has a thread
    When I edit the task "Start this task"
    Then Tasks offers View Thread

  @tasks.page
  Scenario: A Tasks photo is read and displayed by the native app
    When I open Tasks
    And I choose a Tasks photo
    Then the Tasks photo is displayed

  @tasks.page
  Scenario: Tasks can be closed in a narrow window
    When I open Tasks
    And I narrow the Tasks window
    And I close and reopen Tasks
    Then Tasks shows the "list" view

  @tasks.page @tasks.detail
  Scenario: A task opens on its own page with priority, due date, sub-tasks, links and comments
    When I open Tasks
    And I add the task "Plan the launch"
    Then Tasks contains "Plan the launch"
    When I edit the task "Plan the launch"
    Then the task's page is open on "Plan the launch"
    When I set the task's priority to "high"
    And I set the task's due date to "today"
    Then the task "Plan the launch" has priority "high" and a due date
    When I add the sub-task "Book the room"
    Then the task "Book the room" is a sub-task of "Plan the launch"
    And the top level of Tasks does not list "Book the room"
    When I comment "Room 4 is free on Friday"
    Then the task's activity shows the comment "Room 4 is free on Friday"
    When I add the link "https://example.com/plan" titled "The plan"
    Then the task "Plan the launch" links to "The plan"
    When I press Escape in Tasks
    Then the Tasks editor is closed
    And Tasks shows "Due Today"

  @tasks.page @schedule.list
  Scenario: A task with a due date shows under Tasks due in Scheduled and opens from there
    When I open Tasks
    And I add the task "Send the invoice"
    Then Tasks contains "Send the invoice"
    When I edit the task "Send the invoice"
    And I set the task's due date to "today"
    Then the task "Send the invoice" has a due date
    When I click "Scheduled" in the activity bar
    Then the Scheduled list's Tasks due shows "Send the invoice" due "Today"
    When I click the due task "Send the invoice" in Scheduled
    Then the task's page is open on "Send the invoice"
    And the sidebar shows the "scheduled" view

  @tasks.page @tasks.detail
  Scenario: The due date picker offers quick picks and a calendar
    When I open Tasks
    And I add the task "Renew the domain"
    Then Tasks contains "Renew the domain"
    When I edit the task "Renew the domain"
    And I open the task's due date picker
    Then Tasks shows "Today"
    And Tasks shows "End of this week"
    When I set the task's due date to "clear"
    Then the task "Renew the domain" has no due date

  @tasks.page @tasks.detail
  Scenario: J and K step to the next and previous task on its page
    When I open Tasks
    And I add the task "First stepped task"
    Then Tasks contains "First stepped task"
    When I add the task "Second stepped task"
    Then Tasks contains "Second stepped task"
    When I edit the task "Second stepped task"
    And I press "k" in Tasks
    Then the task's page is open on "First stepped task"
    When I press "j" in Tasks
    Then the task's page is open on "Second stepped task"

  @tasks.page
  Scenario: The board can wrap its columns onto new rows
    When I open Tasks
    And I choose the Tasks Board view
    And I turn on Wrap Columns
    Then the Tasks board wraps its columns

  @tasks.page
  Scenario: Add to Tasks from a highlight makes a task that links back to it
    When I open the thread "Rebuild the three landing pages"
    And I choose "Add to Tasks" from the menu of the highlight "alex-example/site#48"
    Then the app says "Added to Tasks: alex-example/site#48"
    When I open Tasks
    Then Tasks contains "alex-example/site#48"
    And the task "alex-example/site#48" links to "alex-example/site#48"

  @tasks.page @tasks.detail
  Scenario: Find and attach a Print without scrolling through the full library
    Given the window is the size GitHub's Mac runner gives it
    When I open Tasks
    And I add the task "Attach a launch Print"
    Then Tasks contains "Attach a launch Print"
    When I edit the task "Attach a launch Print"
    And I open the task's Print search
    And I search the task's Prints for "unmatched-zebra"
    Then Tasks shows "No matching Prints"
    When I press Escape in Tasks
    Then the Tasks editor is open
    When I open the task's Print search
    And I search the task's Prints for "INVOICE TOTALS"
    Then the Print search offers "Invoice totals by month"
    When I press "enter" in Tasks
    Then the task "Attach a launch Print" links to "Invoice totals by month"
    When I open the task's Print search
    And I search the task's Prints for "Invoice totals"
    And I press "enter" in Tasks
    Then the Print search remains open
    When I press Escape in Tasks
    And I open the task's Print search
    And I search the task's Prints for "Launch notes"
    And I choose the matching Print "Launch notes"
    Then the task "Attach a launch Print" links to "Launch notes"
