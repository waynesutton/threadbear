Feature: Diagram Prints
  As someone keeping diagrams with a project or a thread
  I want to edit their source and keep the previous versions
  So the diagram stays useful after the reply that made it

  # The demo-runner driver does not seed local Prints yet. These flows
  # are checked with an isolated dev runner until that driver supports them.
  @prints.diagram @manual @gpui-pending
  Scenario: Edit a diagram's source and keep the preceding version
    Given a diagram Print called "Task flow" contains Mermaid source
    When I choose Edit Text from that Print's menu
    Then the editor shows its Mermaid source
    When I change the source and save it
    Then the diagram redraws as a new version marked edited by you
    And its preceding version can still be opened and restored

  @prints.diagram @manual @gpui-pending
  Scenario: A malformed source edit keeps the current diagram
    Given a diagram Print called "Task flow" has a saved version
    When I save malformed Mermaid source in its Edit Text editor
    Then an error explains that the diagram could not be rendered
    And the current diagram and its version remain unchanged

  @prints.diagram @manual @gpui-pending
  Scenario: A diagram remains accessible without a project
    Given a diagram Print was created in a thread with no project
    When I reopen its card in that thread
    Then I can open the diagram and edit its source
    And assigning it to a project lists the same Print there
