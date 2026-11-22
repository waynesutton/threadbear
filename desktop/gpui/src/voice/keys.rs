// Voice's keyboard shortcuts at work (the Swift app's KeyDictation and
// VoiceKeyMonitor): hold Fn to dictate into whatever text box has the cursor
// and let go to type it in, Shift Fn or Fn Space for hands-free dictation that
// types in at each pause, two taps of Fn if that's on, and Fn Control for
// Command Mode, where what's said goes to the open thread as a message.
// Speech is turned into text on this Mac by macOS, and nothing is spoken back.
// One microphone user at a time: it won't start while Voice mode is on, and
// Voice mode starting ends it.
//
// native/voice_keys.m watches the keys with a local NSEvent monitor (only
// inside the app, so no permission is asked for) and calls `on_key` here,
// which runs the machine at once, since a swallowed key has to be said then,
// and hands what to do to the Voice entity.
// The monitor's half is the Mac's alone.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]
use super::{log, now, Listener, Voice};
use gpui_kit::*;
use serde_json::Value;
use std::cell::RefCell;
use std::time::Duration;
use threadbear_core::voice::{insertion, UtteranceTracker};
use threadbear_core::voice_keys::{
    Key, KeyAction, KeyEvent, KeyMode, VoiceKeyMachine, VoiceKeys, HOLD_DELAY,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum DictationPhase {
    #[default]
    Off,
    Starting(String),
    Listening,
    /// The words so far, not typed in or sent yet.
    Hearing(String),
    /// Let go: the last words are coming in.
    Finishing,
    Failed(String),
}

pub(crate) struct Dictation {
    pub(crate) phase: DictationPhase,
    pub(crate) mode: KeyMode,
    pub(crate) level: f32,
    tracker: UtteranceTracker,
    start: u64,
}

impl Default for Dictation {
    fn default() -> Dictation {
        Dictation {
            phase: DictationPhase::Off,
            mode: KeyMode::Idle,
            level: 0.0,
            tracker: UtteranceTracker::new(0.8),
            start: 0,
        }
    }
}

impl Dictation {
    pub(crate) fn is_on(&self) -> bool {
        !matches!(self.phase, DictationPhase::Off | DictationPhase::Failed(_))
    }
}

/// Waiting for the words heard so far before the microphone stops.
const FINISH_GRACE: Duration = Duration::from_millis(400);

// ---- The monitor ----

/// What the monitor needs at once, on the main thread, without waiting for
/// GPUI: the machine, whether Voice is on, and where its actions go.
struct Monitor {
    machine: VoiceKeyMachine,
    active: bool,
    trigger_down: bool,
    /// Fn-Space's space was swallowed: its key repeats are too.
    swallowing_space: bool,
    /// A text box had the cursor when the trigger went down, asked then, as
    /// nothing in GPUI is locked inside the monitor.
    had_text_box: bool,
    actions: Option<futures::channel::mpsc::UnboundedSender<KeyAction>>,
    typed: Option<futures::channel::mpsc::UnboundedSender<bool>>,
}

thread_local! {
    static MONITOR: RefCell<Monitor> = RefCell::new(Monitor { machine: VoiceKeyMachine::new(VoiceKeys::default()), active: false, trigger_down: false, swallowing_space: false, had_text_box: false, actions: None, typed: None });
}

#[cfg(target_os = "macos")]
extern "C" {
    fn tb_voice_keys_install(
        on_key: extern "C" fn(i32, u16, u64, f64, bool) -> bool,
        on_resign: extern "C" fn(),
    );
    fn tb_voice_uptime() -> f64;
    fn tb_voice_text_context(before: *mut u32, after: *mut u32) -> bool;
    fn tb_voice_type_later(
        utf8: *const std::ffi::c_char,
        compose: extern "C" fn(*const std::ffi::c_char, u32, u32) -> *mut std::ffi::c_char,
        free_text: extern "C" fn(*mut std::ffi::c_char),
        typed: extern "C" fn(bool),
    );
}

const FLAG_SHIFT: u64 = 1 << 17;
const FLAG_CONTROL: u64 = 1 << 18;
const FLAG_OPTION: u64 = 1 << 19;
const FLAG_COMMAND: u64 = 1 << 20;
const FLAG_FUNCTION: u64 = 1 << 23;
/// NX_DEVICERALTKEYMASK and NX_DEVICERCMDKEYMASK: the right-hand keys' own bits.
const FLAG_RIGHT_OPTION: u64 = 0x40;
const FLAG_RIGHT_COMMAND: u64 = 0x10;

pub(super) fn install(voice: WeakEntity<Voice>, cx: &mut App) {
    let (tx, mut rx) = futures::channel::mpsc::unbounded::<KeyAction>();
    let (typed_tx, mut typed_rx) = futures::channel::mpsc::unbounded::<bool>();
    MONITOR.with(|m| {
        let mut m = m.borrow_mut();
        m.actions = Some(tx);
        m.typed = Some(typed_tx);
    });
    let answers = voice.clone();
    cx.spawn(async move |cx| {
        use futures::StreamExt as _;
        while let Some(action) = rx.next().await {
            if voice.update(cx, |v, cx| v.key_action(action, cx)).is_err() {
                break;
            }
        }
    })
    .detach();
    cx.spawn(async move |cx| {
        use futures::StreamExt as _;
        while let Some(typed) = typed_rx.next().await {
            if answers
                .update(cx, |v, cx| v.dictation_typed(typed, cx))
                .is_err()
            {
                break;
            }
        }
    })
    .detach();
    #[cfg(target_os = "macos")]
    unsafe {
        tb_voice_keys_install(on_key, on_resign)
    };
}

/// Whether the trigger is held, by its own side's bit for the right-hand
/// keys, so the left one held as well doesn't keep it down.
fn trigger_down(keys: &VoiceKeys, flags: u64) -> bool {
    use threadbear_core::voice_keys::VoiceTriggerKey as T;
    match keys.trigger {
        T::Fn => flags & FLAG_FUNCTION != 0,
        T::RightOption => flags & FLAG_RIGHT_OPTION != 0,
        T::RightCommand => flags & FLAG_RIGHT_COMMAND != 0,
    }
}

/// A modifier key's flagsChanged that pressed it rather than let it go.
fn modifier_went_down(code: u16, flags: u64) -> bool {
    match code {
        59 | 62 => flags & FLAG_CONTROL != 0,
        58 | 61 => flags & FLAG_OPTION != 0,
        56 | 60 => flags & FLAG_SHIFT != 0,
        55 | 54 => flags & FLAG_COMMAND != 0,
        63 => flags & FLAG_FUNCTION != 0,
        _ => false,
    }
}

/// From the NSEvent monitor: `kind` 0 is flagsChanged, 1 keyDown. True
/// swallows the key.
extern "C" fn on_key(kind: i32, code: u16, flags: u64, timestamp: f64, repeat: bool) -> bool {
    MONITOR.with(|m| {
        let Ok(mut m) = m.try_borrow_mut() else {
            return false;
        };
        if !m.active {
            if m.trigger_down || m.machine.mode() != KeyMode::Idle {
                m.let_go();
            }
            return false;
        }
        let keys = m.machine.keys;
        if kind == 0 {
            if code == keys.trigger.key_code() {
                let down = trigger_down(&keys, flags);
                if down == m.trigger_down {
                    return false;
                }
                m.trigger_down = down;
                if !down {
                    m.swallowing_space = false;
                } else {
                    m.had_text_box = has_text_box();
                }
                if down {
                    log(&format!(
                        "{} down, a text box {}",
                        keys.trigger.display(),
                        if m.had_text_box {
                            "has the cursor"
                        } else {
                            "doesn't have the cursor"
                        }
                    ));
                }
                m.run(if down {
                    KeyEvent::TriggerDown(timestamp)
                } else {
                    KeyEvent::TriggerUp(timestamp)
                });
                // Shift held first, then the trigger: Shift + trigger.
                if down && flags & FLAG_SHIFT != 0 {
                    m.run(KeyEvent::ShiftDown);
                }
            } else if m.trigger_down && modifier_went_down(code, flags) {
                if keys.command_mode && keys.command.key_codes().contains(&code) {
                    m.run(KeyEvent::CommandKeyDown);
                } else if code == 56 || code == 60 {
                    m.run(KeyEvent::ShiftDown);
                } else {
                    m.run(KeyEvent::OtherModifierDown);
                }
            }
            return false;
        }
        if !m.trigger_down && m.machine.mode() == KeyMode::Idle {
            return false;
        }
        if repeat && code == 49 && m.swallowing_space {
            return true;
        }
        let key = match code {
            49 => Key::Space,
            53 => Key::Escape,
            _ => Key::Other,
        };
        let swallowed = m.run(KeyEvent::KeyDown(key));
        if key == Key::Space && swallowed {
            m.swallowing_space = true;
        }
        swallowed
    })
}

/// Switched away from the app with the key held.
extern "C" fn on_resign() {
    MONITOR.with(|m| {
        if let Ok(mut m) = m.try_borrow_mut() {
            m.let_go();
        }
    });
}

impl Monitor {
    fn run(&mut self, event: KeyEvent) -> bool {
        let outcome = self.machine.handle(event);
        if let Some(tx) = &self.actions {
            for action in outcome.actions {
                let _ = tx.unbounded_send(action);
            }
        }
        outcome.swallow
    }

    /// The key's up, whatever the events said: what was held finishes.
    fn let_go(&mut self) {
        self.swallowing_space = false;
        if !self.trigger_down {
            return;
        }
        self.trigger_down = false;
        #[cfg(target_os = "macos")]
        let at = unsafe { tb_voice_uptime() };
        #[cfg(not(target_os = "macos"))]
        let at = 0.0;
        self.run(KeyEvent::TriggerUp(at));
    }
}

/// Types it in at the cursor on the main queue's next turn (voice_keys.m
/// says why), spaced from the characters either side. Whether it went in
/// comes back through `typed`.
fn type_in(words: &str) {
    #[cfg(target_os = "macos")]
    {
        let Ok(words) = std::ffi::CString::new(words) else {
            return;
        };
        unsafe { tb_voice_type_later(words.as_ptr(), compose, free_text, typed) };
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (words, insertion);
}

extern "C" fn compose(
    words: *const std::ffi::c_char,
    before: u32,
    after: u32,
) -> *mut std::ffi::c_char {
    let words = unsafe { std::ffi::CStr::from_ptr(words) }.to_string_lossy();
    let side = |c: u32| char::from_u32(c).filter(|_| c != 0);
    insertion(&words, side(before), side(after))
        .and_then(|t| std::ffi::CString::new(t).ok())
        .map_or(std::ptr::null_mut(), std::ffi::CString::into_raw)
}

extern "C" fn free_text(text: *mut std::ffi::c_char) {
    if !text.is_null() {
        drop(unsafe { std::ffi::CString::from_raw(text) });
    }
}

extern "C" fn typed(done: bool) {
    MONITOR.with(|m| {
        if let Ok(m) = m.try_borrow() {
            if let Some(tx) = &m.typed {
                let _ = tx.unbounded_send(done);
            }
        }
    });
}

fn has_text_box() -> bool {
    #[cfg(target_os = "macos")]
    unsafe {
        let (mut before, mut after) = (0u32, 0u32);
        tb_voice_text_context(&mut before, &mut after)
    }
    #[cfg(not(target_os = "macos"))]
    false
}

// ---- Dictation ----

impl Voice {
    /// The monitor follows Settings, and is off while Voice isn't.
    pub(super) fn sync_keys(&mut self, cx: &mut Context<Self>) {
        let (keys, active) = (self.prefs.keys(), self.available(cx));
        MONITOR.with(|m| {
            if let Ok(mut m) = m.try_borrow_mut() {
                if m.active != active {
                    log(if active { "keys on" } else { "keys off" });
                }
                m.machine.keys = keys;
                m.active = active;
            }
        });
    }

    fn key_action(&mut self, action: KeyAction, cx: &mut Context<Self>) {
        match action {
            KeyAction::WaitToHold(pressed_at) => {
                self.after(Duration::from_secs_f64(HOLD_DELAY), cx, move |_, _| {
                    MONITOR.with(|m| {
                        if let Ok(mut m) = m.try_borrow_mut() {
                            m.run(KeyEvent::HoldDelayPassed(pressed_at));
                        }
                    });
                });
            }
            KeyAction::Start(mode) => self.start_dictation(mode, cx),
            KeyAction::Finish => self.finish_dictation(cx),
            KeyAction::Cancel => self.cancel_dictation(cx),
        }
    }

    fn machine_ended() {
        MONITOR.with(|m| {
            if let Ok(mut m) = m.try_borrow_mut() {
                m.machine.ended();
            }
        });
    }

    /// Starts listening for `mode`, or switches what's listening to it (Space
    /// during a hold carries on hands-free).
    fn start_dictation(&mut self, mode: KeyMode, cx: &mut Context<Self>) {
        if self.keys.is_on() && self.keys.phase != DictationPhase::Finishing {
            self.keys.mode = mode;
            cx.notify();
            return;
        }
        if !self.available(cx) || self.mode.is_on() {
            log("keyboard dictation not started: Voice is off, or talking with a thread");
            Self::machine_ended();
            return;
        }
        let start = self.keys.start + 1;
        self.keys = Dictation {
            start,
            mode,
            phase: DictationPhase::Starting("Asking for the microphone…".to_owned()),
            ..Dictation::default()
        };
        // No text box with the cursor: the open thread's message box.
        let had_box = MONITOR.with(|m| m.try_borrow().map(|m| m.had_text_box).unwrap_or(true));
        if mode != KeyMode::Command && !had_box {
            self.focus_composer(cx);
        }
        if let Err(e) = self.listen(Listener::Keys, false, cx) {
            self.dictation_failed(&e, cx);
        }
        cx.notify();
    }

    fn focus_composer(&self, cx: &mut Context<Self>) {
        let Some(app) = self.app.upgrade() else {
            return;
        };
        let (window, pane) = {
            let a = app.read(cx);
            (a.main_window, a.pane.clone())
        };
        let Some(pane) = pane else { return };
        cx.defer(move |cx| {
            window
                .update(cx, |_, window, cx| {
                    pane.update(cx, |p, cx| p.focus_composer(window, cx))
                })
                .ok();
        });
    }

    pub(super) fn dictation_listening(&mut self, event: &Value, cx: &mut Context<Self>) {
        match event["event"].as_str() {
            Some("listening") => {
                if self.keys.phase != DictationPhase::Finishing {
                    self.keys.phase = DictationPhase::Listening;
                }
                let start = self.keys.start;
                self.every(Duration::from_millis(200), cx, move |v, cx| {
                    if v.keys.start != start || !v.keys.is_on() {
                        return false;
                    }
                    v.dictation_pause(cx);
                    true
                });
                log("dictating from the keyboard");
                cx.notify();
            }
            Some("listenFailed") => self.dictation_failed(
                event["message"]
                    .as_str()
                    .unwrap_or("The microphone wouldn't start."),
                cx,
            ),
            Some("heard") => {
                if !self.keys.is_on() {
                    return;
                }
                let heard = threadbear_core::voice::Heard::new(
                    event["text"].as_str().unwrap_or(""),
                    event["start"].as_f64().unwrap_or(0.0),
                    event["end"].as_f64().unwrap_or(0.0),
                    event["final"].as_bool().unwrap_or(false),
                );
                if self.keys.tracker.add(heard, now())
                    && self.keys.phase != DictationPhase::Finishing
                {
                    self.keys.phase = DictationPhase::Hearing(self.keys.tracker.pending());
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    pub(super) fn dictation_level(&mut self, level: f32, cx: &mut Context<Self>) {
        let rounded = (level * 10.0).round() / 10.0;
        if rounded != self.keys.level {
            self.keys.level = rounded;
            cx.notify();
        }
    }

    /// Hands-free types in at each pause; a hold waits for the release.
    fn dictation_pause(&mut self, cx: &mut Context<Self>) {
        if self.keys.mode != KeyMode::HandsFree || self.keys.phase == DictationPhase::Finishing {
            return;
        }
        if let Some(text) = self.keys.tracker.take(now()) {
            self.type_dictated(&text, cx);
            if self.keys.is_on() {
                self.keys.phase = DictationPhase::Listening;
            }
            cx.notify();
        }
    }

    /// Let go, or hands-free stopped: what was said goes in, or to the
    /// thread, once the recogniser has caught up with the last words.
    fn finish_dictation(&mut self, cx: &mut Context<Self>) {
        if !self.keys.is_on() || self.keys.phase == DictationPhase::Finishing {
            return;
        }
        let start = self.keys.start;
        self.keys.phase = DictationPhase::Finishing;
        cx.notify();
        self.after(FINISH_GRACE, cx, move |v, cx| {
            if v.keys.start != start {
                return;
            }
            let text = v.keys.tracker.flush();
            let mode = v.keys.mode;
            v.end_dictation(cx);
            if let Some(text) = text {
                v.deliver(text, mode, cx);
            }
        });
    }

    /// Esc: nothing typed in or sent.
    pub(crate) fn cancel_dictation(&mut self, cx: &mut Context<Self>) {
        if !self.keys.is_on() {
            self.dismiss_dictation_failure(cx);
            return;
        }
        log("keyboard dictation cancelled");
        self.end_dictation(cx);
        Self::machine_ended();
    }

    fn dismiss_dictation_failure(&mut self, cx: &mut Context<Self>) {
        if matches!(self.keys.phase, DictationPhase::Failed(_)) {
            self.keys.phase = DictationPhase::Off;
            cx.notify();
        }
    }

    fn end_dictation(&mut self, cx: &mut Context<Self>) {
        self.stop_listening(Listener::Keys, cx);
        let start = self.keys.start + 1;
        self.keys = Dictation {
            start,
            ..Dictation::default()
        };
        cx.notify();
    }

    pub(super) fn dictation_failed(&mut self, message: &str, cx: &mut Context<Self>) {
        if !self.keys.is_on() {
            return;
        }
        log(&format!("keyboard dictation failed: {message}"));
        self.end_dictation(cx);
        self.keys.phase = DictationPhase::Failed(message.to_owned());
        Self::machine_ended();
        let start = self.keys.start;
        self.after(Duration::from_secs(5), cx, move |v, cx| {
            if v.keys.start == start {
                v.dismiss_dictation_failure(cx);
            }
        });
        cx.notify();
    }

    fn deliver(&mut self, text: String, mode: KeyMode, cx: &mut Context<Self>) {
        if mode != KeyMode::Command {
            self.type_dictated(&text, cx);
            return;
        }
        let Some(app) = self.app.upgrade() else {
            return;
        };
        let (window, pane) = {
            let a = app.read(cx);
            (a.main_window, a.pane.clone())
        };
        match pane {
            Some(pane) => cx.defer(move |cx| pane.update(cx, |p, cx| p.send(text, None, cx))),
            None => cx.defer(move |cx| {
                window
                    .update(cx, |_, window, cx| {
                        app.update(cx, |app, cx| {
                            app.send_new_thread(text, Vec::new(), String::new(), window, cx)
                        })
                    })
                    .ok();
            }),
        }
        log("sent a Command Mode message");
    }

    /// Typed in at the cursor (over a selection), as if he'd typed it.
    fn type_dictated(&mut self, words: &str, _cx: &mut Context<Self>) {
        type_in(words);
    }

    /// Whether what was dictated went in.
    fn dictation_typed(&mut self, typed: bool, cx: &mut Context<Self>) {
        if typed {
            return;
        }
        // `failed` needs it on; it's ended by now for a hold.
        if !self.keys.is_on() {
            self.keys.phase = DictationPhase::Listening;
        }
        self.dictation_failed(
            "There was no text box to type into. Click into one, then try again.",
            cx,
        );
    }
}
