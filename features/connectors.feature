Feature: The Connectors panel
  As someone whose agents use services through my Connectors gateway
  I want to see which services are signed in, and sign one back in from there
  So an agent never stalls on a service I didn't know had signed out

  Background:
    Given the app is connected to a runner with the demo threads

  @connectors.panel
  Scenario: Each service says whether it's signed in, and a signed-out one has Connect
    When I open the Connectors panel
    Then the Connectors panel's heading is "bruce"
    And the Connectors panel lists "Notion: Signed in"
    And the Connectors panel lists "Sentry: Signed out [Connect]"
    And the Connectors panel lists "  The sign-in expired."

  # What Set up makes runs on this Mac: "This Mac", not "127" (Wayne, 8 Nov).
  @connectors.panel
  Scenario: A gateway on this Mac is called This Mac
    When I open the Connectors panel
    And the gateway moves to this Mac
    Then the Connectors panel's heading is "This Mac"

  # X search is built into Threadbear, not the gateway (Wayne, 11 Nov). The
  # Swift app's dump doesn't list its row.
  @connectors.panel @swift-pending
  Scenario: X search says whether an xAI key is saved, and Forget key deletes it
    When I open the Connectors panel
    Then the Connectors panel lists "X search: xAI key saved [Forget key]"
    When I press "Delete the saved xAI key" in the Connectors panel
    Then the Connectors panel lists "X search: Asks for an xAI key on the first search"

  @connectors.panel
  Scenario: It says when the gateway can't be reached, and catches up once it's back
    When I open the Connectors panel
    And the gateway can't be reached
    Then the Connectors panel lists "Can't reach bruce: timed out."
    When every service on the gateway is signed in
    Then the Connectors panel lists "Sentry: Signed in"
    And the Connectors panel doesn't list "Can't reach bruce: timed out."

  # The Swift dump predates the setup, catalog, agent and hidden sections.
  @connectors.panel @swift-pending
  Scenario: Set up is offered without a gateway and opens its thread
    When I open the Connectors panel
    And no gateway is set up and setup is available
    Then the Connectors panel lists "Set up"
    When I press "Start a thread that installs" in the Connectors panel
    Then the connector setup thread is open

  @connectors.panel @swift-pending
  Scenario: Adding a catalog service opens the thread that adds it
    When I open the Connectors panel
    And the connector catalog offers Slack
    Then the Connectors panel lists "Slack [Add]"
    When I press "Start a thread that adds Slack" in the Connectors panel
    Then the connector setup thread is open

  @connectors.panel @swift-pending
  Scenario: A new agent key is copied through its form and can be revoked
    When I open the Connectors panel
    And I press "Let Grok Bot, Notion" in the Connectors panel
    And I press "Make a key for Grok Bot" in the Connectors panel
    Then the Connectors panel lists "Header value: Bearer tba_•••••••• [Copy]"
    And the Connectors panel never displays the new secret
    When I press "End this key" in the Connectors panel
    Then the Connectors panel doesn't list "Grok Bot: Read only, reads threads it started"
    And the Connectors panel doesn't list "Header value: Bearer tba_•••••••• [Copy]"

  @connectors.panel @swift-pending
  Scenario: Hiding a service removes its alert and the hidden menu restores it
    When I open the Connectors panel
    And I press "Hide Sentry from this panel" in the Connectors panel
    Then the Connectors panel lists "+ 1 hidden"
    And the Connectors panel doesn't list "Sentry: Signed out [Connect]"
    And the Connectors rail has no sign-in alert
    When I restore the hidden Sentry connector
    Then the Connectors panel lists "Sentry: Signed out [Connect]"

  @connectors.panel @swift-pending
  Scenario: A local server shows its exact command before it is added
    When I open the Connectors panel
    And the connector catalog offers a local server
    Then the Connectors panel lists "Blender: On this Mac [Add...]"
    When I press "See the exact command Blender" in the Connectors panel
    Then the Connectors panel lists "uvx mcp-for-blender@2.2.7"
    And the Connectors panel lists "It stays at this version until you add a newer one."
    When I press "Don't add it" in the Connectors panel
    Then the Connectors panel doesn't list "uvx mcp-for-blender@2.2.7"
    When I press "See the exact command Blender" in the Connectors panel
    And I press "Start a thread that adds it to the gateway with exactly this command" in the Connectors panel
    Then the connector setup thread is open
    And the Connectors panel doesn't list "uvx mcp-for-blender@2.2.7"

  @connectors.panel @swift-pending
  Scenario: Closing the panel puts the one-time key away
    When I open the Connectors panel
    And I press "Let Grok Bot, Notion" in the Connectors panel
    And I press "Make a key for Grok Bot" in the Connectors panel
    Then the Connectors panel lists "Header value: Bearer tba_•••••••• [Copy]"
    When I close and reopen the Connectors panel
    Then the Connectors panel doesn't list "Header value: Bearer tba_•••••••• [Copy]"
    And the Connectors panel lists "Grok Bot: Read only, reads threads it started"

  @connectors.panel @swift-pending
  Scenario: Slack explains why it cannot take a key
    When I open the Connectors panel
    And I press "Let Grok Bot, Notion" in the Connectors panel
    And I press "Connect Slack to Threadbear" in the Connectors panel
    Then the Connectors panel doesn't list "Make key"
    And the Connectors panel lists "Slackbot only connects to servers that sign in with OAuth or with Slack's own sign-in, through a Slack app. It can't take a key, so Threadbear can't connect to it yet."
