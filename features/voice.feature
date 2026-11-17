Feature: Voice
  Talk with a thread out loud, and dictate into any text box with the Fn
  keys, in the GPUI app on a Mac. Behind the voiceMode experiment and
  Settings > Voice > Use Voice. Speech is turned into text on the Mac by
  macOS, and pocket-tts reads replies back (docs/guide.md, "Talking with a
  thread" and "Dictating with the keyboard").

  # A real microphone, speakers and Fn key: checked by hand on a Mac with the
  # voiceMode experiment on. The Swift app has no Voice of its own to drive
  # here since Mike took it out (bbf4bdf4), and the mock runner has no
  # experiments.
  @voice.button @manual @swift-pending
  Scenario: The Voice button sits beside Send while Voice is on
    Given the voiceMode experiment is on
    And Use Voice is on in Settings > Voice
    When I open a thread
    Then a microphone button sits to the left of Send
    And hovering it says "Talk with this thread out loud"
    When I turn Use Voice off
    Then the microphone button is gone

  @voice.talk @manual @swift-pending
  Scenario: What I say goes to the thread, and its reply is read out
    Given Voice is on for a thread
    When I say "what's in the readme" and pause
    Then "what's in the readme" is sent to the thread as my message
    And the button pulses while the thread works
    When the thread replies
    Then the reply is read out, with code and tables left for the screen

  @voice.talk @manual @swift-pending
  Scenario: Talking over the reply stops it
    Given Voice is reading a reply out
    When I start talking
    Then the reading stops

  @voice.approvals @manual @swift-pending
  Scenario: A plain command's card is answered out loud
    Given Voice is on for a thread
    When the thread asks to run "npm test"
    Then Voice says "I need your OK. npm test. Say yes, no, or always allow."
    When I say "yes"
    Then the card is allowed
    And Voice says "OK, going ahead."

  @voice.approvals @manual @swift-pending
  Scenario: A card Voice can't read out whole stays on screen
    Given Voice is on for a thread
    When the thread asks to edit a file
    Then Voice says it needs my OK on screen
    When I say "yes"
    Then the card is still waiting
    And Voice says "That one's on screen for you to answer."

  @voice.keys @manual @swift-pending
  Scenario: Hold Fn to dictate into the box with the cursor
    Given Use Voice is on
    When I click into a thread's message box
    And I hold Fn and say "check the logs"
    Then a pill over the window says "Listening: let go of Fn to type it in. Esc cancels."
    When I let go of Fn
    Then the message box reads "Check the logs"

  @voice.keys @manual @swift-pending
  Scenario: Esc cancels dictation with nothing typed
    Given I'm holding Fn and talking
    When I press Esc
    Then nothing is typed and the pill goes

  @voice.keys @manual @swift-pending
  Scenario: Shift Fn starts and stops hands-free dictation
    Given Use Voice is on
    When I press Shift Fn
    Then a pill says "Hands-free: ⇧ Fn to stop, Esc to cancel"
    When I say a sentence and pause
    Then it's typed in at the cursor
    When I press Shift Fn again
    Then dictation stops

  @voice.settings @manual @swift-pending
  Scenario: Settings > Voice has the keys, the icon and the voice
    Given the voiceMode experiment is on
    When I open Settings > Voice
    Then I can turn Use Voice and Command Mode on or off
    And I can pick the Button icon: Microphone, Microphone filled, Stage microphone or Waveform
    And I can pick the Key: Fn (Globe), Right Option or Right Command
    And I can turn Hold to dictate, Key and Space, Shift and the key, and Double tap on or off
    And I can pick the Voice, Speed, Language and Microphone
    And Play a Sample reads a line in the chosen voice

  @voice.talk @manual @swift-pending
  Scenario: A short pause doesn't send half a sentence
    Given Voice is on for a thread
    And Pause before sending is 1.5 seconds
    When I say "check the logs", pause for a second, and say "then the tests"
    Then "check the logs then the tests" is sent as one message
