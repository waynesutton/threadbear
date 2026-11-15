// Voice's keyboard shortcuts (Settings > Voice), ported from the Swift app's
// ThreadbearCore/Voice/VoiceKeys.swift: which key they're built on, and the
// machine that turns presses of it into what they do, with time passed in so
// it can be tested. The app feeds it key events from an NSEvent monitor
// (desktop/gpui/native/voice_keys.m) and does the actions
// (desktop/gpui/src/voice/keys.rs).

/// The key Voice's shortcuts are built on. Fn by default, as other dictation
/// apps have it; the right-hand Option or Command for a keyboard without one,
/// or a Mac where macOS keeps Fn for itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VoiceTriggerKey {
    #[default]
    Fn,
    RightOption,
    RightCommand,
}

impl VoiceTriggerKey {
    pub const ALL: [VoiceTriggerKey; 3] = [VoiceTriggerKey::Fn, VoiceTriggerKey::RightOption, VoiceTriggerKey::RightCommand];

    /// The Swift app's raw value, as saved.
    pub fn name(self) -> &'static str {
        match self {
            VoiceTriggerKey::Fn => "fn",
            VoiceTriggerKey::RightOption => "rightOption",
            VoiceTriggerKey::RightCommand => "rightCommand",
        }
    }

    pub fn from_name(name: &str) -> Option<VoiceTriggerKey> {
        Self::ALL.into_iter().find(|k| k.name() == name)
    }

    /// How a key cap writes it: "Fn", "Right ⌥".
    pub fn display(self) -> &'static str {
        match self {
            VoiceTriggerKey::Fn => "Fn",
            VoiceTriggerKey::RightOption => "Right ⌥",
            VoiceTriggerKey::RightCommand => "Right ⌘",
        }
    }

    /// For the picker.
    pub fn title(self) -> &'static str {
        match self {
            VoiceTriggerKey::Fn => "Fn (Globe)",
            VoiceTriggerKey::RightOption => "Right Option",
            VoiceTriggerKey::RightCommand => "Right Command",
        }
    }

    /// The key's virtual key code, as a flagsChanged event has it.
    pub fn key_code(self) -> u16 {
        match self {
            VoiceTriggerKey::Fn => 63,
            VoiceTriggerKey::RightOption => 61,
            VoiceTriggerKey::RightCommand => 54,
        }
    }
}

/// The key held with the trigger for Command Mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VoiceCommandKey {
    #[default]
    Control,
    Option,
    Shift,
}

impl VoiceCommandKey {
    pub const ALL: [VoiceCommandKey; 3] = [VoiceCommandKey::Control, VoiceCommandKey::Option, VoiceCommandKey::Shift];

    pub fn name(self) -> &'static str {
        match self {
            VoiceCommandKey::Control => "control",
            VoiceCommandKey::Option => "option",
            VoiceCommandKey::Shift => "shift",
        }
    }

    pub fn from_name(name: &str) -> Option<VoiceCommandKey> {
        Self::ALL.into_iter().find(|k| k.name() == name)
    }

    pub fn display(self) -> &'static str {
        match self {
            VoiceCommandKey::Control => "⌃",
            VoiceCommandKey::Option => "⌥",
            VoiceCommandKey::Shift => "⇧",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            VoiceCommandKey::Control => "Control",
            VoiceCommandKey::Option => "Option",
            VoiceCommandKey::Shift => "Shift",
        }
    }

    /// Both sides' key codes: either one counts.
    pub fn key_codes(self) -> [u16; 2] {
        match self {
            VoiceCommandKey::Control => [59, 62],
            VoiceCommandKey::Option => [58, 61],
            VoiceCommandKey::Shift => [56, 60],
        }
    }
}

/// Voice's keyboard shortcuts, as Settings has them. They only work while
/// Voice is on (the experiment and Settings > Voice > Use Voice).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoiceKeys {
    pub trigger: VoiceTriggerKey,
    /// Hold the trigger to dictate; let go to type it in.
    pub hold_to_dictate: bool,
    /// Trigger + Space starts and stops hands-free dictation.
    pub hands_free_with_space: bool,
    /// Shift + trigger starts and stops hands-free dictation too.
    pub shift_toggle: bool,
    /// Two quick taps of the trigger start hands-free dictation. Off by
    /// default: a double tap of Fn is macOS's emoji or dictation too.
    pub double_tap: bool,
    /// Command Mode: off until turned on.
    pub command_mode: bool,
    pub command: VoiceCommandKey,
}

impl Default for VoiceKeys {
    fn default() -> VoiceKeys {
        VoiceKeys { trigger: VoiceTriggerKey::Fn, hold_to_dictate: true, hands_free_with_space: true, shift_toggle: true, double_tap: false, command_mode: false, command: VoiceCommandKey::Control }
    }
}

impl VoiceKeys {
    /// Settings' key caps: "Hold Fn", "Fn Space".
    pub fn hold_display(&self) -> String {
        format!("Hold {}", self.trigger.display())
    }

    pub fn hands_free_display(&self) -> String {
        format!("{} Space", self.trigger.display())
    }

    pub fn shift_display(&self) -> String {
        format!("⇧ {}", self.trigger.display())
    }

    pub fn double_tap_display(&self) -> String {
        format!("{0} {0}", self.trigger.display())
    }

    pub fn command_display(&self) -> String {
        format!("{} {}", self.trigger.display(), self.command.display())
    }

    /// What the pill says while each mode listens.
    pub fn listening_line(&self, mode: KeyMode) -> String {
        let key = self.trigger.display();
        match mode {
            KeyMode::Command => format!("Command Mode: let go of {key} to send it to the thread. Esc cancels."),
            KeyMode::HandsFree => {
                let stop = if self.shift_toggle {
                    self.shift_display()
                } else if self.hands_free_with_space {
                    self.hands_free_display()
                } else {
                    format!("tap {key}")
                };
                format!("Hands-free: {stop} to stop, Esc to cancel")
            }
            KeyMode::Dictating | KeyMode::Idle => format!("Listening: let go of {key} to type it in. Esc cancels."),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyMode {
    #[default]
    Idle,
    /// Held: typed in on release.
    Dictating,
    /// Typed in at each pause until stopped.
    HandsFree,
    /// Held: sent to the thread on release.
    Command,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Space,
    Escape,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyEvent {
    TriggerDown(f64),
    TriggerUp(f64),
    /// The hold delay for the press that started at this time ran out.
    HoldDelayPassed(f64),
    /// The Command Mode key went down.
    CommandKeyDown,
    /// Shift went down with the trigger (either first).
    ShiftDown,
    /// Another modifier went down.
    OtherModifierDown,
    KeyDown(Key),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyAction {
    /// Wait `HOLD_DELAY`, then send HoldDelayPassed for this press.
    WaitToHold(f64),
    /// Start listening for this, or switch what's listening to it.
    Start(KeyMode),
    /// Done: type in, or send, what was heard.
    Finish,
    /// Stop with nothing typed in or sent.
    Cancel,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeyOutcome {
    pub actions: Vec<KeyAction>,
    /// The key goes no further: Fn-Space doesn't type a space.
    pub swallow: bool,
}

impl KeyOutcome {
    fn of(actions: Vec<KeyAction>) -> KeyOutcome {
        KeyOutcome { actions, swallow: false }
    }

    fn swallowing(actions: Vec<KeyAction>, swallow: bool) -> KeyOutcome {
        KeyOutcome { actions, swallow }
    }
}

pub const HOLD_DELAY: f64 = 0.25;
pub const DOUBLE_TAP_GAP: f64 = 0.4;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Press {
    Up,
    /// Down, not doing anything yet: a tap or a hold to come.
    Down(f64),
    /// Holding to dictate or to command.
    Acting,
    /// Pressed while hands-free is on: a tap stops it.
    Stopping,
    /// Used for something else (a modifier, Fn-Space): nothing on release.
    Spent,
}

/// Turns presses of the trigger key into what Voice's shortcuts do.
///
/// - Hold the trigger: after `HOLD_DELAY` it starts listening, and letting go
///   types in what was said. A shorter press is a tap, and so is the trigger
///   used as a modifier (Fn-Delete, Fn-arrow): neither listens.
/// - Trigger + Space, or Shift + trigger, starts hands-free dictation, and
///   again stops it. Two taps within `DOUBLE_TAP_GAP` start it too, if that's
///   on; a tap of the trigger while it's on stops it.
/// - Trigger + the Command Mode key, held: what's said goes to the thread as a
///   message when you let go.
/// - Esc while any of them is on cancels it, with nothing typed or sent.
#[derive(Clone, Debug)]
pub struct VoiceKeyMachine {
    pub keys: VoiceKeys,
    mode: KeyMode,
    press: Press,
    last_tap: Option<f64>,
}

impl VoiceKeyMachine {
    pub fn new(keys: VoiceKeys) -> VoiceKeyMachine {
        VoiceKeyMachine { keys, mode: KeyMode::Idle, press: Press::Up, last_tap: None }
    }

    pub fn mode(&self) -> KeyMode {
        self.mode
    }

    pub fn trigger_held(&self) -> bool {
        self.press != Press::Up
    }

    /// Listening ended some other way (it failed, or Voice took the
    /// microphone): back to idle, and a key still held does nothing more.
    pub fn ended(&mut self) {
        self.mode = KeyMode::Idle;
        if self.press != Press::Up {
            self.press = Press::Spent;
        }
    }

    pub fn handle(&mut self, event: KeyEvent) -> KeyOutcome {
        match event {
            KeyEvent::TriggerDown(at) => {
                if self.mode == KeyMode::HandsFree {
                    self.press = Press::Stopping;
                    return KeyOutcome::default();
                }
                if self.mode != KeyMode::Idle {
                    return KeyOutcome::default();
                }
                if self.keys.double_tap && self.last_tap.is_some_and(|t| at - t <= DOUBLE_TAP_GAP) {
                    self.last_tap = None;
                    self.press = Press::Spent;
                    self.mode = KeyMode::HandsFree;
                    return KeyOutcome::of(vec![KeyAction::Start(KeyMode::HandsFree)]);
                }
                self.press = Press::Down(at);
                if self.keys.hold_to_dictate { KeyOutcome::of(vec![KeyAction::WaitToHold(at)]) } else { KeyOutcome::default() }
            }
            KeyEvent::HoldDelayPassed(pressed_at) => {
                if self.press != Press::Down(pressed_at) || self.mode != KeyMode::Idle || !self.keys.hold_to_dictate {
                    return KeyOutcome::default();
                }
                self.press = Press::Acting;
                self.mode = KeyMode::Dictating;
                KeyOutcome::of(vec![KeyAction::Start(KeyMode::Dictating)])
            }
            KeyEvent::CommandKeyDown => {
                if !self.keys.command_mode {
                    return self.other_modifier();
                }
                let can = matches!(self.press, Press::Down(_)) || (self.press == Press::Acting && self.mode == KeyMode::Dictating);
                if !can {
                    return KeyOutcome::default();
                }
                self.press = Press::Acting;
                self.mode = KeyMode::Command;
                KeyOutcome::of(vec![KeyAction::Start(KeyMode::Command)])
            }
            KeyEvent::OtherModifierDown => self.other_modifier(),
            KeyEvent::ShiftDown => {
                if !self.keys.shift_toggle {
                    return self.other_modifier();
                }
                self.toggle_hands_free(false).unwrap_or_default()
            }
            KeyEvent::KeyDown(Key::Space) => {
                if !self.keys.hands_free_with_space {
                    return self.other_key();
                }
                match self.toggle_hands_free(true) {
                    Some(outcome) => outcome,
                    None => self.other_key(),
                }
            }
            KeyEvent::KeyDown(Key::Escape) => {
                if self.mode == KeyMode::Idle {
                    return KeyOutcome::default();
                }
                self.mode = KeyMode::Idle;
                if self.press != Press::Up {
                    self.press = Press::Spent;
                }
                KeyOutcome::swallowing(vec![KeyAction::Cancel], true)
            }
            KeyEvent::KeyDown(Key::Other) => self.other_key(),
            KeyEvent::TriggerUp(at) => {
                let press = std::mem::replace(&mut self.press, Press::Up);
                match press {
                    // Let go before the hold delay: a tap.
                    Press::Down(_) => {
                        self.last_tap = Some(at);
                        KeyOutcome::default()
                    }
                    Press::Acting | Press::Stopping => {
                        self.last_tap = None;
                        self.mode = KeyMode::Idle;
                        KeyOutcome::of(vec![KeyAction::Finish])
                    }
                    Press::Up | Press::Spent => {
                        self.last_tap = None;
                        KeyOutcome::default()
                    }
                }
            }
        }
    }

    /// Trigger + Space or Shift: hands-free on, or off if it's on. None if
    /// the trigger isn't held for it.
    fn toggle_hands_free(&mut self, swallow: bool) -> Option<KeyOutcome> {
        match self.press {
            Press::Stopping => {
                self.press = Press::Spent;
                self.mode = KeyMode::Idle;
                Some(KeyOutcome::swallowing(vec![KeyAction::Finish], swallow))
            }
            Press::Down(_) | Press::Acting => {
                self.press = Press::Spent;
                self.mode = KeyMode::HandsFree;
                Some(KeyOutcome::swallowing(vec![KeyAction::Start(KeyMode::HandsFree)], swallow))
            }
            Press::Up | Press::Spent => None,
        }
    }

    fn other_modifier(&mut self) -> KeyOutcome {
        if matches!(self.press, Press::Down(_)) {
            self.press = Press::Spent;
        }
        KeyOutcome::default()
    }

    /// A key typed with the trigger held is the trigger used as a modifier
    /// (Fn-Delete, Fn-arrow): it never starts listening, and it ends a hold
    /// under way. Typing during hands-free is left alone.
    fn other_key(&mut self) -> KeyOutcome {
        match self.press {
            Press::Down(_) | Press::Stopping => {
                self.press = Press::Spent;
                self.last_tap = None;
                KeyOutcome::default()
            }
            Press::Acting => {
                self.press = Press::Spent;
                self.mode = KeyMode::Idle;
                KeyOutcome::of(vec![KeyAction::Cancel])
            }
            Press::Up | Press::Spent => KeyOutcome::default(),
        }
    }
}
