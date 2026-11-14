Feature: A saved plan opens beside the thread
  As someone who asked for a plan first
  I want the plan the thread saved to open beside it, and a link to it under the reply
  So I can read it at once and find it again later

  Background:
    Given the app is connected to a runner with the demo threads

  @transcript.savedplan
  Scenario: The saved plan opens in the pane, and the note under the reply opens it again
    When I open the thread "Rebuild the three landing pages"
    And I'm looking at the app
    And the agent of "Rebuild the three landing pages" saved its plan
    Then the pane is showing the saved plan
    And the transcript shows the note "Saved the plan in" outside the turn's work
    When I press Back in the pane
    And I click the link "plans/" in the note "Saved the plan in"
    Then the pane is showing the saved plan
