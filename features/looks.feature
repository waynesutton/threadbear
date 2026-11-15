Feature: Customizing the Librarian and Scheduled pages
  As someone who looks at the Librarian and Scheduled pages every day
  I want to change their pictures, their background and what shows on them
  So the pages feel like mine and show what I care about

  Background:
    Given the app is connected to a runner with the demo threads

  # The Swift app's dump doesn't say how a customized page is laid out.
  @looks.librarian @swift-pending
  Scenario: Customize is off until it's turned on in Settings
    When I click "Librarian" in the activity bar
    Then the Librarian page is in place of the thread
    And the page isn't customized
    When I turn on "Customize the Librarian page" in Settings
    And I click "Librarian" in the activity bar
    Then the page is customized with "picture, about, numbers, settings"

  @looks.librarian @swift-pending
  Scenario: The Librarian panel hides a part and puts it back
    Given "Customize the Librarian page" is on in Settings
    When I click "Librarian" in the activity bar
    And I open Customize on the page
    Then the Customize panel is open
    When I switch "Numbers" off in Customize's layout
    Then the page is customized with "picture, about, settings"
    When I reset the page in Customize
    Then the page is customized with "picture, about, numbers, settings"

  @looks.scheduled @swift-pending
  Scenario: The Scheduled page takes the bear off
    Given "Customize the Scheduled page" is on in Settings
    When I click "Scheduled" in the activity bar
    And I open Customize on the page
    And I choose "No picture on the page" in Customize
    Then the page is customized with "about, numbers"
