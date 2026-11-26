Feature: Giving a plan board box to the project coordinator
  As someone whose project has a coordinator
  I want to hand an approved box to it as a to-do
  So the coordinator runs it with the rest of the project's work

  # The planBoardCoordinators experiment (runner/src/experiments/planBoardCoordinators.md),
  # which needs planBoard and Mike's project coordinators. On a copy without
  # coordinators (Wayne's fork) these are skipped (@needs.coordinators,
  # features/support/world.ts), and the destination never shows.
  Background:
    Given the app is connected to a runner with the demo threads

  @plan.board @needs.coordinators @swift-pending
  Scenario: The destination is absent without a coordinator
    Given the plan of "Plan the Acme dashboard" is approved at version 1
    When I send the box "KPI cards" to "Claude"
    Then the send sheet has no coordinator destination

  @plan.board @needs.coordinators @swift-pending
  Scenario: Start hands the box to the coordinator, once
    Given the project of "Plan the Acme dashboard" has an active coordinator
    And the plan of "Plan the Acme dashboard" is approved at version 1
    When I give the box "KPI cards" to the project coordinator
    And I press Start twice
    Then the coordinator has one to-do for "KPI cards"
    And the box "KPI cards" says "Working"
