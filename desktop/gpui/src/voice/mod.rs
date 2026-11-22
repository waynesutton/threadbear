// Voice in the GPUI app: talking with a thread out loud (mode.rs), dictating
// into any text box with the Fn keys (keys.rs), the button beside Send and the
// dictation pill (button.rs), and Settings > Voice
// (settings_window/voice.rs). The Swift app's app/Sources/Threadbear/Voice,
// without the bear avatar. Its rules are desktop/core/src/voice.rs and
// voice_keys.rs; the microphone, speech recognition and the speaker are the
// Swift helper's (helper.rs).
//
// Behind the voiceMode experiment and Settings > Voice > Use Voice. Off,
// there's no button, the keys do nothing, nothing downloads and macOS never
// asks for the microphone. Everything here is in its own files, and the rest
// of the app reaches it through a few calls (`receive`, `button::render`,
// `button::pill`, the settings tab), so the port merges cleanly as the app
// around it changes.
pub(crate) mod button;
mod helper;
pub(crate) mod keys;
pub(crate) mod mode;

use crate::app::Threadbear;
use gpui_kit::*;
use serde_json::{json, Value};
use std::time::Duration;
use threadbear_core::reducer::event::Event;
use threadbear_core::voice::VoicePrefs;

/// The experiment's id (runner/src/experiments/voiceMode.md).
pub(crate) const EXPERIMENT: &str = "voiceMode";

/// What the microphone is doing for, so what it hears goes to the right one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Listener {
    Mode,
    Keys,
}

pub(crate) struct Voice {
    app: WeakEntity<Threadbear>,
    pub(crate) prefs: VoicePrefs,
    helper: Option<helper::Helper>,
    /// Which helper is the current one: a late word from one that's gone
    /// mustn't end the one started after it.
    helper_gen: u64,
    /// Who the microphone is listening for, and the number its results come
    /// back with.
    listener: Option<(Listener, i64)>,
    session: i64,
    pub(crate) mode: mode::ModeState,
    pub(crate) keys: keys::Dictation,
    /// The Mac's microphones for Settings, as uid and name, once asked for.
    pub(crate) microphones: Vec<(String, String)>,
    mics_asked: bool,
    /// Play a Sample's progress, or why it couldn't.
    pub(crate) preview_status: Option<String>,
}

struct VoiceGlobal(Entity<Voice>);

impl Global for VoiceGlobal {}

/// Voice, made the first time anything asks for it (the composer drawing).
pub(crate) fn get(app: &WeakEntity<Threadbear>, cx: &mut App) -> Entity<Voice> {
    if let Some(found) = cx.try_global::<VoiceGlobal>() {
        return found.0.clone();
    }
    let app = app.clone();
    let voice = cx.new(|cx: &mut Context<Voice>| {
        if let Some(entity) = app.upgrade() {
            cx.observe(&entity, |voice: &mut Voice, app, cx| {
                voice.app_changed(app, cx)
            })
            .detach();
        }
        Voice {
            app,
            prefs: load_prefs(),
            helper: None,
            helper_gen: 0,
            mics_asked: false,
            listener: None,
            session: 0,
            mode: mode::ModeState::default(),
            keys: keys::Dictation::default(),
            microphones: Vec::new(),
            preview_status: None,
        }
    });
    cx.set_global(VoiceGlobal(voice.clone()));
    keys::install(voice.downgrade(), cx);
    voice.update(cx, |v, cx| v.sync_keys(cx));
    voice
}

/// Voice, if anything has made it yet.
pub(crate) fn existing(cx: &App) -> Option<Entity<Voice>> {
    cx.try_global::<VoiceGlobal>().map(|g| g.0.clone())
}

/// Every live event of every open thread (ThreadPane::event). Deferred, as
/// the pane is mid-update: Voice may answer one of its cards.
pub(crate) fn receive(event: &Event, cx: &mut App) {
    let Some(voice) = existing(cx) else { return };
    if !voice.read(cx).mode.is_on_for(&event.thread_id) {
        return;
    }
    let event = event.clone();
    cx.defer(move |cx| voice.update(cx, |v, cx| v.mode_event(&event, cx)));
}

/// The new-thread page made the thread its spoken words asked for
/// (new_thread.rs, `finish_new_thread`), before it opens: Voice carries on
/// in that thread, and only that one.
pub(crate) fn thread_started(id: &str, cx: &mut App) {
    if let Some(voice) = existing(cx) {
        voice.update(cx, |v, cx| v.mode_thread_started(id, cx));
    }
}

/// The runner says the experiment is on: Settings' switch first, as it
/// changes without a new hello, else the hello's list (experiment_on).
pub(crate) fn experiment_on(app: &Threadbear) -> bool {
    let switch = app.runner_settings.as_ref().and_then(|s| {
        s["experiments"]
            .as_array()?
            .iter()
            .find(|e| e["id"] == EXPERIMENT)
            .map(|e| e["on"].as_bool() == Some(true))
    });
    switch.unwrap_or_else(|| app.remote.experiments.iter().any(|e| e == EXPERIMENT))
}

fn prefs_path() -> std::path::PathBuf {
    threadbear_client::config::RunnerConfig::from_env()
        .home
        .join("desktop-voice.json")
}

fn load_prefs() -> VoicePrefs {
    std::fs::read(prefs_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Voice's own log, ~/Library/Logs/Threadbear/voice.log, as the Swift app's
/// VoiceLog writes it: what happened, never what was said.
pub(crate) fn log(line: &str) {
    use std::io::Write as _;
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    let path = std::path::PathBuf::from(home).join("Library/Logs/Threadbear/voice.log");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 256_000) {
        let _ = std::fs::remove_file(&path);
    }
    if let Ok(mut f) = threadbear_core::private_file::append(&path) {
        let stamp = threadbear_core::chrono::Utc::now()
            .to_rfc3339_opts(threadbear_core::chrono::SecondsFormat::Secs, true);
        let _ = writeln!(f, "{stamp} [gpui] {line}");
    }
}

/// Seconds on a clock of the app's own, for the pauses between words.
pub(crate) fn now() -> f64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_secs_f64()
}

impl Voice {
    /// The experiment's on, Use Voice is on and this build can listen.
    pub(crate) fn available(&self, cx: &App) -> bool {
        self.prefs.enabled()
            && helper::built_in()
            && self
                .app
                .upgrade()
                .is_some_and(|a| experiment_on(a.read(cx)))
    }

    pub(crate) fn set_prefs(&mut self, edit: impl FnOnce(&mut VoicePrefs), cx: &mut Context<Self>) {
        let before = self.prefs.clone();
        edit(&mut self.prefs);
        if self.prefs == before {
            return;
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.prefs) {
            let _ = threadbear_core::private_file::write(&prefs_path(), &bytes);
        }
        if !self.prefs.enabled() {
            self.stop_mode(cx);
            self.cancel_dictation(cx);
        }
        if before.language != self.prefs.language || before.microphone != self.prefs.microphone {
            self.mode_restart(cx);
        }
        self.sync_keys(cx);
        cx.notify();
    }

    fn app_changed(&mut self, app: Entity<Threadbear>, cx: &mut Context<Self>) {
        let selected = app.read(cx).selected.clone();
        self.mode_selection_changed(selected, cx);
        self.sync_keys(cx);
    }

    // ---- The helper ----

    /// Sends it a command, starting it first if need be. False if it
    /// couldn't be started, with why.
    fn tell(&mut self, command: Value, cx: &mut Context<Self>) -> Result<(), String> {
        if self.helper.is_none() {
            let (helper, mut rx) = helper::Helper::start()?;
            self.helper = Some(helper);
            self.helper_gen += 1;
            let gen = self.helper_gen;
            cx.spawn(async move |this, cx| {
                use futures::StreamExt as _;
                while let Some(event) = rx.next().await {
                    if this
                        .update(cx, |v, cx| {
                            if v.helper_gen == gen {
                                v.on_helper(event, cx)
                            }
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();
        }
        let sent = self.helper.as_mut().is_some_and(|h| h.send(&command));
        if sent {
            Ok(())
        } else {
            self.helper = None;
            Err("Voice's helper stopped. Try again.".to_owned())
        }
    }

    /// Listening for `who`: the microphone on, with the number its results
    /// come back with.
    fn listen(&mut self, who: Listener, echo: bool, cx: &mut Context<Self>) -> Result<(), String> {
        self.session += 1;
        self.listener = Some((who, self.session));
        let language = self.prefs.language();
        let command = json!({ "cmd": "listen", "session": self.session, "locale": language.locale, "microphone": self.prefs.microphone(), "echo": echo });
        self.tell(command, cx)
    }

    fn stop_listening(&mut self, who: Listener, cx: &mut Context<Self>) {
        if self.listener.is_some_and(|(w, _)| w == who) {
            self.listener = None;
            let _ = self.tell(json!({ "cmd": "stopListening" }), cx);
        }
    }

    fn listening_for(&self, event: &Value) -> Option<Listener> {
        let (who, session) = self.listener?;
        (event["session"].as_i64() == Some(session)).then_some(who)
    }

    fn on_helper(&mut self, event: Value, cx: &mut Context<Self>) {
        match event["event"].as_str().unwrap_or("") {
            "gone" => {
                self.helper = None;
                if self.listener.take().is_some() || self.mode.is_on() {
                    log("the helper stopped");
                    self.mode_failed("Voice stopped unexpectedly. Try again.", cx);
                    self.dictation_failed("Voice stopped unexpectedly. Try again.", cx);
                }
            }
            "status" | "engineReady" | "engineFailed" => self.mode_engine(&event, cx),
            "listening" | "listenFailed" | "heard" => match self.listening_for(&event) {
                Some(Listener::Mode) => self.mode_listening(&event, cx),
                Some(Listener::Keys) => self.dictation_listening(&event, cx),
                None => {}
            },
            "inputLevel" | "outputLevel" => {
                let level = event["level"].as_f64().unwrap_or(0.0) as f32;
                let output = event["event"] == "outputLevel";
                match self.listener.map(|(w, _)| w) {
                    Some(Listener::Mode) => self.mode_level(level, output, cx),
                    Some(Listener::Keys) if !output => self.dictation_level(level, cx),
                    _ => {}
                }
            }
            "speaking" => self.mode_speaking(
                event["on"].as_bool().unwrap_or(false),
                event["at"].as_f64().unwrap_or(0.0),
                cx,
            ),
            "played" => self.mode_played(
                event["id"].as_str().unwrap_or(""),
                event["at"].as_f64().unwrap_or(0.0),
                cx,
            ),
            "microphones" => {
                self.microphones = event["list"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|m| {
                        Some((
                            m["uid"].as_str()?.to_owned(),
                            m["name"].as_str()?.to_owned(),
                        ))
                    })
                    .collect();
                cx.notify();
            }
            "previewStatus" => {
                self.preview_status = event["text"].as_str().map(str::to_owned);
                cx.notify();
            }
            _ => {}
        }
    }

    /// Settings' list of microphones, asked for when it opens.
    pub(crate) fn load_microphones(&mut self, cx: &mut Context<Self>) {
        if std::mem::replace(&mut self.mics_asked, true) {
            return;
        }
        let _ = self.tell(json!({ "cmd": "microphones" }), cx);
    }

    /// Play a Sample: a line in the chosen voice at the chosen speed.
    pub(crate) fn preview(&mut self, cx: &mut Context<Self>) {
        let voice = self.prefs.voice();
        let command = json!({ "cmd": "preview", "text": format!("Hi, I'm {}. Ask me anything, and I'll keep working while we talk.", voice.name), "voice": voice.engine_voice, "speed": self.prefs.speed(), "language": self.prefs.language().id });
        if let Err(e) = self.tell(command, cx) {
            self.preview_status = Some(e);
            cx.notify();
        }
    }

    /// A timer of the app's own, every `every`, while `keep` says so.
    fn every(
        &self,
        every: Duration,
        cx: &mut Context<Self>,
        tick: impl Fn(&mut Voice, &mut Context<Voice>) -> bool + 'static,
    ) {
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(every).await;
            match this.update(cx, |v, cx| tick(v, cx)) {
                Ok(true) => {}
                _ => break,
            }
        })
        .detach();
    }

    /// Once, after `after`.
    fn after(
        &self,
        after: Duration,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Voice, &mut Context<Voice>) + 'static,
    ) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(after).await;
            this.update(cx, |v, cx| then(v, cx)).ok();
        })
        .detach();
    }
}
