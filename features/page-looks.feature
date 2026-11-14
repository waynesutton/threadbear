# Swift has this UI, but its frozen scripted driver has no look command.
Feature: Customize the home pages
  Background:
    Given the app is connected to a runner with the demo threads

  @librarian.customize @swift-pending
  Scenario: A greeting and quote can be added without changing the standard page
    When I customize the Librarian page with a greeting for "Wayne" and a quote
    Then the customized page draws its greeting and quote
    When I ask for another quote
    Then the customized page draws a different quote
    When I turn off the Librarian page customization
    Then the standard Librarian page is shown and my look is kept

  @librarian.customize @swift-pending
  Scenario: The picture choice keeps a chosen picture through other changes
    When I choose my own Librarian picture
    And I customize the Librarian page with a greeting for "Wayne" and a quote
    Then my Librarian picture is still drawn

  @librarian.customize @swift-pending
  Scenario: A new background appears and leaves the quote button clickable
    When I customize the Librarian page with a greeting for "Wayne" and a quote
    And I choose a Librarian background
    Then the customized page draws its background
    And the customized page draws its greeting and quote
    When I ask for another quote
    Then the customized page draws a different quote
