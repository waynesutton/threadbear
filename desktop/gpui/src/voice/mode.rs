// Voice mode: talking with a thread out loud (the Swift app's VoiceMode). It's
// another way in and out of the same thread, not another agent: what Mike
// says is sent as his message (into the running turn if there is one, as
// typing during a turn is), and the agent's replies are read out. So the
// thread keeps its memory, tools and approvals, and goes on working while he
// talks. Talking over the agent stops it speaking. One thread at a time, and
// opening another thread ends it.
use super::{log, now, Listener, Voice};
use crate::thread::ThreadPane;
use gpui_kit::*;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use threadbear_core::reducer::event::{ApprovalDecision, ApprovalOption, ApprovalRequest, Event, EventPayload};
use threadbear_core::reducer::row::RowContent;
use threadbear_core::voice::{self as rules, Heard, SpokenApprovals, SpokenOutcome, UtteranceTracker};

/// The thread id Voice uses for the new-thread page: what he says first
/// starts the thread, and Voice carries on in it.
pub(crate) const NEW_THREAD: &str = "new";

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum Phase {
    #[default]
    Off,
    /// Downloading or starting the engine, or asking for the microphone.
    Starting(String),
    Listening,
    /// He's talking: the words so far.
    Hearing(String),
    /// Sent; the agent is working on it.
    Thinking,
    Speaking,
    Failed(String),
}

pub(crate) struct ModeState {
    pub(crate) phase: Phase,
    pub(crate) thread: Option<String>,
    /// 0...1: him while listening, the agent while it speaks.
    pub(crate) level: f32,
    tracker: UtteranceTracker,
    /// Replies already spoken, so a catch-up doesn't say one twice.
    spoken: HashSet<String>,
    started_at: Option<threadbear_core::chrono::DateTime<threadbear_core::chrono::Utc>>,
    /// The agent's turn is running.
    working: bool,
    /// The turn's newest message, said when it ends unless a final reply came.
    last_message: Option<(String, String, String)>,
    approvals: SpokenApprovals,
    /// Cards being read out, by the id their reading's `played` comes back with.
    reading: HashMap<String, ApprovalRequest>,
    next_reading: u64,
    /// The newest start: one that's been overtaken gives way to it.
    start: u64,
    /// Without echo cancellation: ignore the microphone until then (the
    /// helper's clock).
    quiet_until: f64,
    echo_cancels: bool,
    speaking: bool,
    /// When the microphone's audio began, on the helper's clock.
    clock_start: Option<f64>,
    waiting_words: Vec<String>,
    making_thread: bool,
}

impl Default for ModeState {
    fn default() -> ModeState {
        ModeState {
            phase: Phase::Off,
            thread: None,
            level: 0.0,
            tracker: UtteranceTracker::new(1.0),
            spoken: HashSet::new(),
            started_at: None,
            working: false,
            last_message: None,
            approvals: SpokenApprovals::default(),
            reading: HashMap::new(),
            next_reading: 0,
            start: 0,
            quiet_until: 0.0,
            echo_cancels: false,
            speaking: false,
            clock_start: None,
            waiting_words: Vec::new(),
            making_thread: false,
        }
    }
}

impl ModeState {
    /// Running, or getting ready to. A failure is off, with its message
    /// showing until it's closed.
    pub(crate) fn is_on(&self) -> bool {
        !matches!(self.phase, Phase::Off | Phase::Failed(_))
    }

    pub(crate) fn is_on_for(&self, thread: &str) -> bool {
        self.is_on() && self.thread.as_deref() == Some(thread)
    }

    /// The button pulses: getting ready, or the agent working.
    pub(crate) fn busy(&self) -> bool {
        matches!(self.phase, Phase::Starting(_) | Phase::Thinking)
    }

    pub(crate) fn failure_for(&self, thread: &str) -> Option<&str> {
        match &self.phase {
            Phase::Failed(m) if self.thread.as_deref() == Some(thread) => Some(m),
            _ => None,
        }
    }
}

impl Voice {
    pub(crate) fn toggle_mode(&mut self, thread: &str, cx: &mut Context<Self>) {
        if self.mode.is_on_for(thread) {
            self.stop_mode(cx);
        } else {
            self.start_mode(thread, cx);
        }
    }

    pub(crate) fn start_mode(&mut self, thread: &str, cx: &mut Context<Self>) {
        if !self.available(cx) {
            return;
        }
        self.stop_mode(cx);
        // The microphone is one thing at a time.
        self.cancel_dictation(cx);
        let start = self.mode.start + 1;
        self.mode = ModeState { start, thread: Some(thread.to_owned()), started_at: Some(threadbear_core::chrono::Utc::now()), phase: Phase::Starting("Getting ready…".to_owned()), ..ModeState::default() };
        let language = self.prefs.language();
        if let Err(e) = self.tell(json!({ "cmd": "prepare", "language": language.id, "start": start }), cx) {
            self.mode_failed(&e, cx);
        }
        cx.notify();
    }

    /// The language or microphone changed in Settings: the engine and the
    /// recogniser are per language, so Voice starts over, on the same thread.
    pub(crate) fn mode_restart(&mut self, cx: &mut Context<Self>) {
        if let Some(thread) = self.mode.thread.clone().filter(|_| self.mode.is_on()) {
            self.start_mode(&thread, cx);
        }
    }

    pub(crate) fn stop_mode(&mut self, cx: &mut Context<Self>) {
        self.dismiss_mode_failure(cx);
        if !self.mode.is_on() {
            return;
        }
        let at = self.spoken_at();
        if let (Some(thread), Some(text)) = (self.mode.thread.clone(), self.mode.tracker.flush()) {
            self.send_spoken(&thread, text, at, cx);
        }
        self.stop_listening(Listener::Mode, cx);
        let _ = self.tell(json!({ "cmd": "stopEngineSoon" }), cx);
        if let Some(thread) = &self.mode.thread {
            log(&format!("off for {thread}"));
        }
        let start = self.mode.start + 1;
        self.mode = ModeState { start, ..ModeState::default() };
        cx.notify();
    }

    pub(super) fn mode_failed(&mut self, message: &str, cx: &mut Context<Self>) {
        if !self.mode.is_on() {
            return;
        }
        log(&format!("failed: {message}"));
        self.stop_listening(Listener::Mode, cx);
        let (start, thread) = (self.mode.start + 1, self.mode.thread.clone());
        self.mode = ModeState { start, thread, phase: Phase::Failed(message.to_owned()), ..ModeState::default() };
        cx.notify();
    }

    /// The error under the button goes when it's closed.
    pub(crate) fn dismiss_mode_failure(&mut self, cx: &mut Context<Self>) {
        if matches!(self.mode.phase, Phase::Failed(_)) {
            self.mode.phase = Phase::Off;
            self.mode.thread = None;
            cx.notify();
        }
    }

    /// Another thread opened: Voice ends. On the new-thread page, the thread
    /// its first words made has opened, and Voice carries on there.
    pub(super) fn mode_selection_changed(&mut self, selected: Option<String>, cx: &mut Context<Self>) {
        if !self.mode.is_on() {
            return;
        }
        let Some(thread) = self.mode.thread.clone() else { return };
        if thread == NEW_THREAD {
            // Another thread opened: Voice ends. The one its words made is
            // taken on before it opens (`mode_thread_started`).
            if selected.is_some() {
                self.stop_mode(cx);
            }
            return;
        }
        if selected.as_deref() != Some(thread.as_str()) {
            self.stop_mode(cx);
        }
    }

    /// The thread the new-thread page made for what he said.
    pub(super) fn mode_thread_started(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.mode.is_on() || !self.mode.making_thread || self.mode.thread.as_deref() != Some(NEW_THREAD) {
            return;
        }
        self.mode.making_thread = false;
        self.mode.thread = Some(id.to_owned());
        log(&format!("started {id} by voice"));
        let words = std::mem::take(&mut self.mode.waiting_words);
        if words.is_empty() {
            return;
        }
        // Once it's open, with a pane to send through.
        let id = id.to_owned();
        cx.spawn(async move |this, cx| {
            this.update(cx, |v, cx| {
                for w in words {
                    v.send_to_pane(&id, w, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn mode_engine(&mut self, event: &Value, cx: &mut Context<Self>) {
        if !matches!(self.mode.phase, Phase::Starting(_)) || event["start"].as_u64() != Some(self.mode.start) {
            return;
        }
        match event["event"].as_str() {
            Some("status") => {
                self.mode.phase = Phase::Starting(event["text"].as_str().unwrap_or("Getting ready…").to_owned());
                cx.notify();
            }
            Some("engineReady") => {
                self.mode.phase = Phase::Starting("Asking for the microphone…".to_owned());
                if let Err(e) = self.listen(Listener::Mode, true, cx) {
                    self.mode_failed(&e, cx);
                }
                cx.notify();
            }
            Some("engineFailed") => self.mode_failed(event["message"].as_str().unwrap_or("The voice engine didn't start."), cx),
            _ => {}
        }
    }

    pub(super) fn mode_listening(&mut self, event: &Value, cx: &mut Context<Self>) {
        match event["event"].as_str() {
            Some("listening") => {
                self.mode.echo_cancels = event["echoCancels"].as_bool().unwrap_or(false);
                self.mode.phase = Phase::Listening;
                let start = self.mode.start;
                self.every(Duration::from_millis(200), cx, move |v, cx| {
                    if v.mode.start != start || !v.mode.is_on() {
                        return false;
                    }
                    v.check_pause(cx);
                    true
                });
                if let Some(thread) = &self.mode.thread {
                    log(&format!("on for {thread}"));
                }
                cx.notify();
            }
            Some("listenFailed") => self.mode_failed(event["message"].as_str().unwrap_or("The microphone wouldn't start."), cx),
            Some("heard") => self.mode_heard(event, cx),
            _ => {}
        }
    }

    fn mode_heard(&mut self, event: &Value, cx: &mut Context<Self>) {
        if !self.mode.is_on() {
            return;
        }
        if let Some(start) = event["clockStart"].as_f64() {
            self.mode.clock_start = Some(start);
        }
        let heard = Heard::new(event["text"].as_str().unwrap_or(""), event["start"].as_f64().unwrap_or(0.0), event["end"].as_f64().unwrap_or(0.0), event["final"].as_bool().unwrap_or(false));
        // Without echo cancellation the microphone hears the agent: what it
        // hears while speaking, and just after, is its own voice. Headphones
        // don't leak, so he can still talk over it there. By when the words
        // were captured as well as now: the recogniser can hand over what
        // the microphone heard a while ago.
        if !self.mode.echo_cancels && event["headphones"].as_bool() != Some(true) {
            let helper_now = event["now"].as_f64().unwrap_or(f64::INFINITY);
            let captured = self.mode.clock_start.map(|s| s + heard.start);
            if self.mode.speaking || helper_now < self.mode.quiet_until || captured.is_none_or(|c| c < self.mode.quiet_until) {
                return;
            }
        }
        if self.mode.tracker.add(heard, now()) {
            // He's talking: the agent stops.
            if self.mode.speaking {
                let _ = self.tell(json!({ "cmd": "stopSpeaking" }), cx);
            }
            self.mode.phase = Phase::Hearing(self.mode.tracker.pending());
            cx.notify();
        }
    }

    pub(super) fn mode_level(&mut self, level: f32, output: bool, cx: &mut Context<Self>) {
        // Him while listening, the agent while it speaks.
        if output != self.mode.speaking {
            return;
        }
        // In tenths: the app redraws only for a change it could show.
        let rounded = (level * 10.0).round() / 10.0;
        if rounded != self.mode.level {
            self.mode.level = rounded;
            cx.notify();
        }
    }

    pub(super) fn mode_speaking(&mut self, on: bool, at: f64, cx: &mut Context<Self>) {
        if !self.mode.is_on() {
            return;
        }
        self.mode.speaking = on;
        if on {
            self.mode.phase = Phase::Speaking;
        } else {
            // A moment for sound still on its way out, longer when the
            // microphone can hear it.
            self.mode.quiet_until = at + if self.mode.echo_cancels { 0.4 } else { 0.8 };
            if self.mode.phase == Phase::Speaking {
                self.mode.phase = if self.mode.working { Phase::Thinking } else { Phase::Listening };
            }
        }
        cx.notify();
    }

    pub(super) fn mode_played(&mut self, id: &str, at: f64, _cx: &mut Context<Self>) {
        if let Some(card) = self.mode.reading.remove(id) {
            self.mode.approvals.read_out(&card, at);
        }
    }

    fn check_pause(&mut self, cx: &mut Context<Self>) {
        let at = self.spoken_at();
        let Some(thread) = self.mode.thread.clone() else { return };
        if let Some(text) = self.mode.tracker.take(now()) {
            self.send_spoken(&thread, text, at, cx);
        }
    }

    /// When he started saying what's waiting, on the clock approvals use.
    /// Not known counts as long ago, which can't answer a card.
    fn spoken_at(&self) -> f64 {
        match (self.mode.tracker.pending_start(), self.mode.clock_start) {
            (Some(start), Some(clock)) => clock + start,
            _ => f64::NEG_INFINITY,
        }
    }

    fn send_spoken(&mut self, thread: &str, text: String, spoken_at: f64, cx: &mut Context<Self>) {
        // The new-thread page: the first words start the thread.
        if thread == NEW_THREAD {
            self.mode.working = true;
            self.mode.phase = Phase::Thinking;
            if self.mode.making_thread {
                self.mode.waiting_words.push(text);
                return;
            }
            self.mode.making_thread = true;
            self.start_thread_spoken(text, cx);
            cx.notify();
            return;
        }
        // A yes or a no on its own answers the card he heard, and only that
        // one (SpokenApprovals says why).
        let quiet_since = if self.mode.speaking { f64::INFINITY } else { self.mode.quiet_until };
        match self.mode.approvals.take(&text, spoken_at, quiet_since) {
            SpokenOutcome::Answer(card, decision) => {
                // Last look at what the thread itself shows: a card settled
                // from the phone meanwhile, or changed, is only known there.
                if self.open_approval(thread, &card.approval_id, cx).as_ref() != Some(&card) {
                    log("an approval answered by voice had gone; not answered");
                    self.speak_now("That request has gone, so I didn't answer it.", cx);
                    self.idle();
                    return;
                }
                let raw = match decision {
                    ApprovalDecision::Deny => "deny",
                    ApprovalDecision::AllowAlways => "allowAlways",
                    _ => "allow",
                };
                if let Some(pane) = self.pane(thread, cx) {
                    let id = card.approval_id.clone();
                    cx.defer(move |cx| pane.update(cx, |p, cx| p.respond_approval(&id, raw, None, None, cx)));
                }
                log(&format!("answered an approval by voice: {raw}"));
                self.speak_now(if decision == ApprovalDecision::Deny { "OK, I won't." } else { "OK, going ahead." }, cx);
                self.mode.working = true;
                self.mode.phase = Phase::Thinking;
                // Another card still waiting: its turn to be heard.
                if let Some(next) = self.mode.approvals.open().last().cloned() {
                    self.read_out(&next, cx);
                }
            }
            SpokenOutcome::ReadAgain(card) => {
                log("heard an answer to an approval not read out yet; reading it");
                self.idle();
                self.read_out(&card, cx);
            }
            SpokenOutcome::OnScreen(_) => {
                self.speak_now("That one's on screen for you to answer.", cx);
                self.idle();
            }
            SpokenOutcome::Message => {
                self.send_to_pane(thread, text, cx);
                self.mode.working = true;
                self.mode.phase = Phase::Thinking;
            }
        }
        cx.notify();
    }

    /// The open thread's pane, if it's this thread.
    fn pane(&self, thread: &str, cx: &App) -> Option<Entity<ThreadPane>> {
        let app = self.app.upgrade()?;
        app.read(cx).pane.clone().filter(|p| p.read(cx).summary.id == thread)
    }

    /// The card as the thread shows it, while it's still waiting.
    fn open_approval(&self, thread: &str, approval_id: &str, cx: &App) -> Option<ApprovalRequest> {
        let pane = self.pane(thread, cx)?;
        pane.read(cx).feed.model().rows().iter().find_map(|r| match &r.content {
            RowContent::Approval(a) if a.request.approval_id == approval_id && a.resolution.is_none() => Some(a.request.clone()),
            _ => None,
        })
    }

    /// What he said, sent as his message, as typing it would be.
    fn send_to_pane(&mut self, thread: &str, text: String, cx: &mut Context<Self>) {
        let text = text.trim().to_owned();
        if text.is_empty() {
            return;
        }
        match self.pane(thread, cx) {
            Some(pane) => cx.defer(move |cx| pane.update(cx, |p, cx| p.send(text, None, cx))),
            None => self.mode_failed("That thread isn't open any more, so that wasn't sent.", cx),
        }
    }

    /// Starts a thread with what he said on the new-thread page, with its
    /// folder, login and model as the page has them; Voice follows it when it
    /// opens (`mode_selection_changed`).
    fn start_thread_spoken(&mut self, text: String, cx: &mut Context<Self>) {
        let Some(app) = self.app.upgrade() else { return };
        let window = app.read(cx).main_window;
        let start = self.mode.start;
        let watched = app.downgrade();
        cx.defer(move |cx| {
            window
                .update(cx, |_, window, cx| {
                    app.update(cx, |app, cx| app.send_new_thread(text, Vec::new(), String::new(), window, cx));
                })
                .ok();
        });
        // The page refused it, or the runner couldn't make it: say so, rather
        // than sit on "working". Making it, the page says it's creating.
        self.every(Duration::from_millis(500), cx, move |v, cx| {
            if v.mode.start != start || !v.mode.making_thread {
                return false;
            }
            let state = watched.upgrade().map(|a| { let a = a.read(cx); (a.new_thread.creating, a.new_thread.error.clone()) });
            match state {
                Some((true, _)) => true,
                Some((false, error)) => {
                    let why = error.map_or_else(|| "That thread couldn't be started. Try again, or type it.".to_owned(), |e| format!("That thread couldn't be started. {e}"));
                    v.mode_failed(&why, cx);
                    false
                }
                None => false,
            }
        });
    }

    /// Back from hearing him, with nothing sent.
    fn idle(&mut self) {
        self.mode.phase = if self.mode.working { Phase::Thinking } else { Phase::Listening };
    }

    // ---- The agent's side ----

    /// A live event of the voice thread (`super::receive`).
    pub(super) fn mode_event(&mut self, event: &Event, cx: &mut Context<Self>) {
        if !self.mode.is_on_for(&event.thread_id) {
            return;
        }
        match &event.payload {
            EventPayload::TurnStarted { .. } => {
                self.mode.working = true;
                self.mode.last_message = None;
                if self.mode.phase == Phase::Listening {
                    self.mode.phase = Phase::Thinking;
                }
            }
            EventPayload::TurnCompleted(_) => {
                self.mode.working = false;
                // A thread without one reply: its last message was the reply.
                if let Some((id, text, ts)) = self.mode.last_message.take() {
                    self.say(&text, &id, &ts, cx);
                }
                if self.mode.phase == Phase::Thinking {
                    self.mode.phase = Phase::Listening;
                }
            }
            EventPayload::AssistantText { item_id, text, summary, narration, final_reply, .. } => {
                if *summary || *narration {
                    return;
                }
                if *final_reply {
                    self.mode.last_message = None;
                    self.say(text, item_id, &event.ts, cx);
                } else {
                    self.mode.last_message = Some((item_id.clone(), text.clone(), event.ts.clone()));
                }
            }
            EventPayload::ApprovalRequested(request) => {
                self.mode.approvals.requested(request.clone());
                if !self.before_voice(&event.ts) && self.mode.spoken.insert(format!("approval-{}", event.seq)) {
                    self.read_out(request, cx);
                }
            }
            EventPayload::ApprovalResolved(resolution) => self.mode.approvals.resolved(&resolution.approval_id),
            EventPayload::QuestionRequested(_) => self.say("I've got a question for you on screen.", &format!("question-{}", event.seq), &event.ts, cx),
            _ => {}
        }
        cx.notify();
    }

    /// A card's request, whole, then what to say to it. It can be answered
    /// from when all that has played to its end. A card the words can't
    /// cover is named, and left for the screen.
    fn read_out(&mut self, card: &ApprovalRequest, cx: &mut Context<Self>) {
        let Some(reading) = SpokenApprovals::reading(card).map(str::to_owned) else {
            self.speak(rules::sentences(&format!("I need your OK on screen: {}.", card.title)), None, cx);
            return;
        };
        let ask = if card.options.contains(&ApprovalOption::AllowAlways) { "Say yes, no, or always allow." } else { "Say yes or no." };
        self.mode.next_reading += 1;
        let id = format!("card-{}", self.mode.next_reading);
        self.mode.reading.insert(id.clone(), card.clone());
        self.speak(vec!["I need your OK.".to_owned(), reading, ask.to_owned()], Some(id), cx);
    }

    fn speak_now(&mut self, line: &str, cx: &mut Context<Self>) {
        self.speak(vec![line.to_owned()], None, cx);
    }

    fn say(&mut self, markdown: &str, id: &str, ts: &str, cx: &mut Context<Self>) {
        if self.before_voice(ts) || !self.mode.spoken.insert(id.to_owned()) {
            return;
        }
        self.speak(rules::sentences(markdown), None, cx);
    }

    /// Only what came after Voice was turned on is said.
    fn before_voice(&self, ts: &str) -> bool {
        let Some(started) = self.mode.started_at else { return false };
        threadbear_core::chrono::DateTime::parse_from_rfc3339(ts).is_ok_and(|t| t < started)
    }

    fn speak(&mut self, sentences: Vec<String>, played: Option<String>, cx: &mut Context<Self>) {
        if sentences.is_empty() || !self.mode.is_on() {
            return;
        }
        let voice = self.prefs.voice();
        let command = json!({ "cmd": "speak", "sentences": sentences, "voice": voice.engine_voice, "speed": self.prefs.speed(), "language": self.prefs.language().id, "id": played });
        if let Err(e) = self.tell(command, cx) {
            self.mode_failed(&e, cx);
        }
    }
}
