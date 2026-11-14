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

  @scheduled.customize @swift-pending
  Scenario: Scheduled parts can sit at the bottom and individual numbers can be hidden
    When I customize the Scheduled page with its numbers at the bottom
    Then the Scheduled numbers are drawn near the bottom of the window

  @projects.customize @swift-pending
  Scenario: Hiding the Projects search does not leave a secret filter
    When I search for projects then hide the customized page header
    Then every project card is shown

  @librarian.customize @swift-pending
  Scenario: An animated background keeps both frames for playback
    When I choose an animated Librarian background
    Then both background frames are kept and the background is drawn
  @profile.page @swift-pending
  Scenario: Rapid profile edits save the latest draft and load again on reopening
    When I quickly edit my profile page twice
    Then the profile page has saved my latest name and bio
    When I reopen the Librarian Customize panel
    Then the profile page has loaded my latest name and bio

  @profile.page @swift-pending
  Scenario: A load started while typing cannot undo the completed save
    When I quickly edit my profile page twice
    Then the profile page has saved my latest name and bio
    When I reopen the profile panel while my latest bio saves
    Then the late load leaves my latest bio saved and on screen

  @profile.page @swift-pending
  Scenario: Opening profile controls keeps everything private until I change it
    When I open my profile controls without editing them
    Then no profile has been sent and all its public parts are off
