Feature: Web addresses written as inline code
  Background:
    Given the app is connected to a runner with the demo threads
    And I open the thread "Morning brief"

  @rich.inline @rich.webmenu @swift-pending
  Scenario Outline: A code address keeps its code look and gets the web link menu
    When a reply contains the inline code "<code>"
    Then the menu of the link "<code>" offers "Open in Panel | Copy Link | Default"
    When I choose "Copy Link" from the menu of the link "<code>"
    Then the clipboard holds "<url>"
    Examples:
      | code                         | url                                  |
      | github.com/owner/repo         | https://github.com/owner/repo         |
      | threadbear.mikecann.app       | https://threadbear.mikecann.app       |
      | https://docs.convex.dev/home  | https://docs.convex.dev/home          |

  @rich.inline @swift-pending
  Scenario: My own code address is a link too
    When I send the formatted message
      """
      See `github.com/owner/repo` for the details.
      """
    Then my inline code address points to "https://github.com/owner/repo"
