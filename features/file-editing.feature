Feature: Working on a file in the side pane
  As someone fixing what an agent wrote
  I want to type into a file beside the conversation, saved as I go
  So a small fix doesn't need another app, and nobody's work is lost

  Background:
    Given the app is connected to a runner with the demo threads

  @file.editor
  Scenario: A text file opens ready to type in, and what's typed is saved by itself
    When I open the thread "Rebuild the three landing pages"
    And I open "logs/docs-build.txt" in the thread's folder in the pane
    Then the pane's file is being edited
    And the pane's status bar says "Line 1, Column 1"
    When I type "Fixed in 4b7d0a8." at the end of the pane's file
    Then the file "logs/docs-build.txt" on disk ends with "Fixed in 4b7d0a8."
    And the pane's status bar says "Saved"
    When I'm done editing the pane's file
    Then the pane's file can be edited
    And the pane shows a "text" page with the words "Fixed in 4b7d0a8."

  @file.editor
  Scenario: With Edit files like an editor off, a file opens to read and the pencil edits it
    When I open Settings
    And I press "Open text files ready to type in, with line numbers and a line and column bar" in Settings
    And I open the thread "Rebuild the three landing pages"
    And I open "logs/docs-build.txt" in the thread's folder in the pane
    Then the pane's file can be edited
    When I start editing the pane's file
    Then the pane's file is being edited

  @file.editor
  Scenario: With Save files automatically off, typing waits for ⌘S, and the file's name has a dot until then
    When I open Settings
    And I press "Write what you type a second after you stop" in Settings
    And I open the thread "Rebuild the three landing pages"
    And I open "src/hero.ts" in the thread's folder in the pane
    And I type "// later" at the end of the pane's file, without waiting
    Then the pane's status bar says "Edited"
    And the pane's file is marked unsaved
    And the file "src/hero.ts" on disk still ends with "}"
    When I save the pane's file
    Then the file "src/hero.ts" on disk ends with "// later"
    And the pane's file isn't marked unsaved

  @file.editor
  Scenario: Leaving a file with changes not saved asks first: Cancel stays, Don't Save goes without them
    When I open Settings
    And I press "Write what you type a second after you stop" in Settings
    And I open the thread "Rebuild the three landing pages"
    And I open "src/hero.ts" in the thread's folder in the pane
    And I type "// maybe" at the end of the pane's file, without waiting
    And I press Back in the pane, answering "Cancel"
    Then the pane is showing the file "src/hero.ts"
    When I press Back in the pane, answering "Don't Save"
    Then the pane shows Highlights
    And the file "src/hero.ts" on disk still ends with "}"

  @file.editor
  Scenario: Leaving a file with changes not saved, Save writes them first
    When I open Settings
    And I press "Write what you type a second after you stop" in Settings
    And I open the thread "Rebuild the three landing pages"
    And I open "src/hero.ts" in the thread's folder in the pane
    And I type "// kept" at the end of the pane's file, without waiting
    And I press Back in the pane, answering "Save"
    Then the pane shows Highlights
    And the file "src/hero.ts" on disk ends with "// kept"

  @file.editor @file.markdown
  Scenario: ⌘S saves at once, in a Markdown file's own text
    When I open the thread "Rebuild the three landing pages"
    And I click the highlight "copy-review.md"
    And I pick "markdown" from the pane's Markdown menu
    And I type "- Footer links checked." at the end of the pane's file, without waiting
    And I save the pane's file
    Then the file "docs/copy-review.md" on disk ends with "- Footer links checked."

  @file.conflict
  Scenario: A file changed on disk under unsaved typing asks which to keep, and Keep mine writes mine
    When I open the thread "Rebuild the three landing pages"
    And I open "src/hero.ts" in the thread's folder in the pane
    And I start editing the pane's file
    And I type "// mine" at the end of the pane's file, without waiting
    And "src/hero.ts" in the thread's folder is rewritten as "// theirs"
    Then the pane says "Changed on disk while you were editing."
    When I keep mine in the pane
    Then the file "src/hero.ts" on disk ends with "// mine"

  @file.conflict
  Scenario: Taking the file's side shows what's on disk, and writes nothing
    When I open the thread "Rebuild the three landing pages"
    And I open "src/hero.ts" in the thread's folder in the pane
    And I start editing the pane's file
    And I type "// mine" at the end of the pane's file, without waiting
    And "src/hero.ts" in the thread's folder is rewritten as "// theirs"
    Then the pane says "Changed on disk while you were editing."
    When I take the file's side in the pane
    Then the pane's file reads "// theirs"
    And the file "src/hero.ts" on disk ends with "// theirs"

  @foundation.filewatch @file.textview
  Scenario: A file an agent rewrites shows its new words by itself
    When I open the thread "Rebuild the three landing pages"
    And I open "logs/docs-build.txt" in the thread's folder in the pane
    Then the pane shows a "text" page with the words "Could not resolve image"
    When "logs/docs-build.txt" in the thread's folder is rewritten as "Build passed in 9.8s."
    Then the pane shows a "text" page with the words "Build passed in 9.8s."

  @file.find
  Scenario: While a file is being edited, the find bar replaces one match or every one
    When I open the thread "Rebuild the three landing pages"
    And I open "src/hero.ts" in the thread's folder in the pane
    And I find "hero" in the pane's file, to replace with "banner"
    Then the find bar says "hero: 1 of 2"
    When I replace the match in the pane's file
    Then the find bar says "hero: 1 of 1"
    When I replace every match in the pane's file
    Then the find bar says "hero: not found"
    And the file "src/hero.ts" on disk has the words "export function banner("
