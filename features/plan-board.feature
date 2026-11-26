Feature: The plan board
  As someone starting a piece of work with an agent
  I want to agree the architecture on a board before anything is built
  So I can send each part to Claude Code or Codex once I've approved it

  # The planBoard experiment (runner/src/experiments/planBoard.md), GPUI only:
  # the Swift app is frozen. The demo is the mock's TB_MOCK_PLAN_BOARD: a
  # planner thread "Plan the Acme dashboard" whose agent drew a v1 plan, and
  # "Fix the login bug" with none.
  Background:
    Given the app is connected to a runner with the demo threads
    And the experiment "planBoard" is on

  @plan.board @swift-pending
  Scenario: A thread with no plan looks the same as before
    When I open the thread "Fix the login bug"
    Then there is no "Show plan" button

  @plan.board @swift-pending
  Scenario: The agent's plan is drawn on the board
    When I open the thread "Plan the Acme dashboard"
    And I press "Show plan"
    Then the board has the box "KPI cards"
    And the board has the group "SERVER · NODE"
    And the board has an arrow "every 5 s" from "GET /api/summary" to "KPI cards"
    And the board says "Draft v1"

  @plan.board @swift-pending
  Scenario: Show plan in the transcript opens the board
    When I open the thread "Plan the Acme dashboard"
    And I rest on the link "Show plan" in the note "Drew a plan"
    And I click the link "Show plan" in the note "Drew a plan"
    Then the board has the box "KPI cards"

  @plan.board @swift-pending
  Scenario: The board closes from its own bar
    When I open the thread "Plan the Acme dashboard"
    And I press "Show plan"
    Then the board has the box "KPI cards"
    When I press "Close the board"
    Then the board is closed

  @plan.board @swift-pending
  Scenario: The board pops out into its own window and goes back
    When I open the thread "Plan the Acme dashboard"
    And I press "Show plan"
    And I press "Open the board in its own window"
    Then the board is in its own window
    And the board has the box "KPI cards"
    When I put the board back beside the thread
    Then the board is beside the thread

  @plan.board @swift-pending
  Scenario: Highlights has a link that opens the board
    When I open the thread "Plan the Acme dashboard"
    And I click the highlight "Plan board"
    Then the board has the box "KPI cards"

  @plan.board @swift-pending
  Scenario: Selecting a box puts it in the message box
    When I open the thread "Plan the Acme dashboard"
    And I press "Show plan"
    And I click the box "KPI cards" on the board
    Then the composer has the chip "KPI cards"
    When I press Escape on the board
    Then the composer has no chip "KPI cards"

  @plan.board @swift-pending
  Scenario: Nothing can be sent until the plan is approved
    When I open the thread "Plan the Acme dashboard"
    And I press "Show plan"
    Then "Send to…" on the box "Revenue chart" is disabled
    When I press "Approve plan"
    Then the board says "Approved v1"
    And "Send to…" on the box "Revenue chart" is enabled

  @plan.board @swift-pending
  Scenario: A change after approval needs approving again
    Given the plan of "Plan the Acme dashboard" is approved at version 1
    When the agent of "Plan the Acme dashboard" sends a new plan
    Then the board says "Changed since you approved"
    And "Send to…" on the box "Revenue chart" is disabled

  @plan.board @swift-pending
  Scenario: Sending a box out starts one linked thread and the box follows it
    Given the plan of "Plan the Acme dashboard" is approved at version 1
    When I send the box "Revenue chart" to "Codex"
    Then the send sheet shows the brief for "Revenue chart"
    When I press Start twice
    Then one thread "Revenue chart" is nested under "Plan the Acme dashboard"
    And the box "Revenue chart" says "Working"

  @plan.board @swift-pending
  Scenario: A box that needs you shows in the inbox too
    Given the box "KPI cards" of "Plan the Acme dashboard" was sent to "Claude"
    When the thread "KPI cards" asks a question
    Then the box "KPI cards" says "Needs you"
    And "Plan the Acme dashboard" is in the Inbox

  @plan.board @swift-pending
  Scenario: Board skills go into the brief
    Given the plan of "Plan the Acme dashboard" is approved at version 1
    When I pick the board skill "component-canvas"
    And I send the box "KPI cards" to "Claude"
    Then the send sheet's brief names the skill "component-canvas"

  @plan.board @swift-pending
  Scenario: A bad plan is refused and the old one stays
    When the agent of "Plan the Acme dashboard" sends a plan that doesn't parse
    Then the board still shows version 1

  @plan.board @swift-pending
  Scenario: A change after approval greys every box
    Given the plan of "Plan the Acme dashboard" is approved at version 1
    And the box "Revenue chart" of "Plan the Acme dashboard" was sent to "Codex"
    When the agent of "Plan the Acme dashboard" sends a new plan that only changes "KPI cards"
    Then the board says "Changed since you approved"
    And "Send to…" on the box "KPI cards" is disabled
    And "Send to…" on the box "Live orders" is disabled
    And the box "Revenue chart" still says "Working"

  @plan.board @swift-pending
  Scenario: Done boxes do not wake the planner until you ask
    Given the box "KPI cards" of "Plan the Acme dashboard" was sent to "Claude"
    And the box "Revenue chart" of "Plan the Acme dashboard" was sent to "Codex"
    When the thread "KPI cards" finishes
    Then the box "KPI cards" says "Done"
    And the planner of "Plan the Acme dashboard" has no new message
    And there is no "Ask planner to review" button
    When the thread "Revenue chart" finishes
    Then the box "Revenue chart" says "Done"
    And the board shows "Ask planner to review"
    When I press "Ask planner to review"
    Then the planner of "Plan the Acme dashboard" has one new message asking it to review the plan

  @plan.board @plan.board.off @swift-pending
  Scenario: With the experiment off there is no board
    Given the experiment "planBoard" is off
    When I open the thread "Plan the Acme dashboard"
    Then there is no "Show plan" button
