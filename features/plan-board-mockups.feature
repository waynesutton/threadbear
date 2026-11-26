Feature: Screens on the plan board
  As someone agreeing a plan with an agent
  I want its screens for a box as pictures beside it, a few takes side by side
  So I can pick one and point at it before anything is built

  # The planBoardMockups experiment (runner/src/experiments/planBoardMockups.md),
  # which needs planBoard. GPUI only. Stills are drawn sealed by
  # threadbear_web::draw_page_at; no page runs on the board.
  Background:
    Given the app is connected to a runner with the demo threads

  @plan.board @plan.board.mockups @web-runtime @swift-pending
  Scenario: Three takes of a box's screen sit side by side
    Given the web runtime is installed
    And the agent of "Plan the Acme dashboard" drew screens A, B and C for "kpi"
    When I open the thread "Plan the Acme dashboard"
    And I press "Show plan"
    And I open the screens of "KPI cards"
    Then the board shows 3 screens for "KPI cards"
    And every screen is drawn as a picture

  @plan.board @plan.board.mockups @swift-pending
  Scenario: Pick B puts it in the message box
    Given the agent of "Plan the Acme dashboard" drew screens A, B and C for "kpi"
    When I open the thread "Plan the Acme dashboard"
    And I press "Show plan"
    And I open the screens of "KPI cards"
    And I pick screen "B"
    Then screen "B" is picked
    And the composer has the chip "KPI cards · B"

  @plan.board @plan.board.mockups @swift-pending
  Scenario: A new version of a screen says Changed
    Given the agent of "Plan the Acme dashboard" drew screens A, B and C for "kpi"
    And the agent of "Plan the Acme dashboard" drew screens A for "kpi"
    When I open the thread "Plan the Acme dashboard"
    And I press "Show plan"
    And I open the screens of "KPI cards"
    Then screen "A" says Changed

  @plan.board @swift-pending
  Scenario: With mockups off there is no Screens…
    When I open the thread "Plan the Acme dashboard"
    And I press "Show plan"
    Then the board has no Screens…
