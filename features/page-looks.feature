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

  @looks.scheduled @swift-pending
  Scenario: The Scheduled page takes a look and keeps it when Customize goes off
    When I customize the Scheduled page without its bear or its On number
    Then the customized Scheduled page draws its title and numbers without them
    When I open Customize on the Scheduled page
    Then its panel has the Layout and Numbers sections
    When I turn off the Scheduled page customization
    Then the standard Scheduled page is shown and my Scheduled look is kept

  @looks.projects @swift-pending
  Scenario: The Projects page shows every project as plain cards when its header is hidden
    When I customize the Projects page without its header or the cards' icon tiles
    Then the customized Projects page draws its cards without them
    When I open Customize on the Projects page
    Then its panel has the Layout and Cards sections
    When I turn off the Projects page customization
    Then the standard Projects page is shown and my Projects look is kept
