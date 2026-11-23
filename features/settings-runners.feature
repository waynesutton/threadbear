Feature: Settings › Runners
  As someone whose threads run on more than one machine
  I want my machines listed in my order, and to choose where new threads start
  So new threads go where I want them, from this Mac and from my phone

  Background:
    Given the app is connected to a runner with the demo threads

  @settings.runners
  Scenario: The runners are in my order, with a star where new threads start
    When I open Settings on "runners"
    Then Settings lists the runners "laptop *: This Mac" and "mac-mini: Online"
    And Settings says new threads start on "This Mac"

  @settings.runners
  Scenario: Picking another machine for new threads moves the star
    When I open Settings on "runners"
    And I pick "mac-mini" for new threads in Settings
    Then Settings says new threads start on "mac-mini"
    And Settings lists the runners "laptop: This Mac" and "mac-mini *: Online"
    When I pick "This Mac" for new threads in Settings
    Then Settings lists the runners "laptop *: This Mac" and "mac-mini: Online"

  # The Swift list moves by a real drag, which its scripts can't make without
  # the real pointer (AGENTS.md, "Tests that need the real screen").
  @settings.runners @swift-pending
  Scenario: Dragging a runner changes the order, here and in the Runners panel
    When I open Settings on "runners"
    And I drag "mac-mini" onto "laptop" in Settings
    Then Settings lists the runners "mac-mini: Online" and "laptop *: This Mac"
    When I open the Runners panel
    Then the Runners panel lists "mac-mini" before "laptop"

  # Cloud runners: a Fly virtual runner set up from one token
  # (docs/virtual-runners.md, "Setting one up"). The Swift app is frozen, so
  # this is the GPUI app's alone.
  @settings.cloud @virtual-runners @swift-pending
  Scenario: Adding a cloud runner shows each step, then the runner
    When I open Settings on "runners"
    Then Settings shows no cloud runner yet
    When I add a cloud runner in Settings with a Fly token
    Then Settings lists the cloud runner "Cloud (Fly)" as "Fly · 1 CPU, 2 GB"
    And Settings shows the cloud step "ok Box image"
    And Settings shows the cloud step "warn Image follows main (GitHub): gh isn't signed in here, so the image updates only when you press Rebuild."
