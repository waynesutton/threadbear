// Talking into a note (the Swift app's NoteDictation): Voice's listening half
// on its own. What's said is turned into text on this Mac by macOS and typed
// into the open note at its cursor each time there's a pause, through the
// note editor's own edit, so it saves, colours and undoes as typing does. The
// words go into the note itself, wherever the keyboard is, and a note's words
// still never leave the Mac. Nothing is spoken back, so pocket-tts isn't
// started. One microphone user at a time: this ends Voice on a thread and Fn
// dictation, and either of those starting ends this.
use super::{log, now, Listener, Voice};
use crate::notes_page::NotesPage;
use gpui_kit::*;
use serde_json::Value;
use std::time::Duration;
use threadbear_core::voice::{Heard, UtteranceTracker};

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum NotePhase {
    #[default]
    Off,
    Starting(String),
    Listening,
    /// The words so far, not typed in yet.
    Hearing(String),
    Failed(String),
}

pub(crate) struct NoteDictation {
    pub(crate) phase: NotePhase,
    pub(crate) note: Option<String>,
    pub(crate) level: f32,
    page: WeakEntity<NotesPage>,
    tracker: UtteranceTracker,
    start: u64,
}

impl Default for NoteDictation {
    fn default() -> NoteDictation {
        NoteDictation { phase: NotePhase::Off, note: None, level: 0.0, page: WeakEntity::new_invalid(), tracker: UtteranceTracker::new(0.8), start: 0 }
    }
}

impl NoteDictation {
    pub(crate) fn is_on(&self) -> bool {
        !matches!(self.phase, NotePhase::Off | NotePhase::Failed(_))
    }

    pub(crate) fn is_on_for(&self, note: &str) -> bool {
        self.is_on() && self.note.as_deref() == Some(note)
    }
}

impl Voice {
    /// The microphone beside the note's bin shows: Voice is on, and so is
    /// Use with Notes.
    pub(crate) fn notes_available(&self, cx: &App) -> bool {
        self.available(cx) && self.prefs.in_notes()
    }

    pub(crate) fn toggle_note(&mut self, note: &str, page: WeakEntity<NotesPage>, cx: &mut Context<Self>) {
        if self.note.is_on_for(note) {
            self.stop_note(true, cx);
        } else {
            self.start_note(note, page, cx);
        }
    }

    fn start_note(&mut self, note: &str, page: WeakEntity<NotesPage>, cx: &mut Context<Self>) {
        if !self.notes_available(cx) {
            return;
        }
        self.stop_note(true, cx);
        // The microphone is one thing at a time.
        self.stop_mode(cx);
        self.cancel_dictation(cx);
        let start = self.note.start + 1;
        self.note = NoteDictation { start, note: Some(note.to_owned()), page, phase: NotePhase::Starting("Asking for the microphone…".to_owned()), ..NoteDictation::default() };
        if let Err(e) = self.listen(Listener::Note, false, cx) {
            self.note_failed(&e, cx);
        }
        cx.notify();
    }

    /// Whatever was being said goes in first, unless the note it was for
    /// isn't the one showing any more.
    pub(crate) fn stop_note(&mut self, typing: bool, cx: &mut Context<Self>) {
        if matches!(self.note.phase, NotePhase::Failed(_)) {
            self.note.phase = NotePhase::Off;
            self.note.note = None;
            cx.notify();
        }
        if !self.note.is_on() {
            return;
        }
        if let (true, Some(text)) = (typing, self.note.tracker.flush()) {
            self.type_into_note(text, cx);
        }
        self.stop_listening(Listener::Note, cx);
        let start = self.note.start + 1;
        self.note = NoteDictation { start, ..NoteDictation::default() };
        log("stopped dictating into a note");
        cx.notify();
    }

    pub(super) fn note_failed(&mut self, message: &str, cx: &mut Context<Self>) {
        if !self.note.is_on() {
            return;
        }
        log(&format!("note dictation failed: {message}"));
        self.stop_listening(Listener::Note, cx);
        let (start, note) = (self.note.start + 1, self.note.note.clone());
        self.note = NoteDictation { start, note, phase: NotePhase::Failed(message.to_owned()), ..NoteDictation::default() };
        cx.notify();
    }

    /// The Notes page showed another note, or went away: dictation for the
    /// one it was showing stops, with nothing more typed into it.
    pub(super) fn notes_changed(&mut self, showing: Option<String>, cx: &mut Context<Self>) {
        if self.note.is_on() && self.note.note != showing {
            self.stop_note(false, cx);
        }
    }

    pub(super) fn note_listening(&mut self, event: &Value, cx: &mut Context<Self>) {
        match event["event"].as_str() {
            Some("listening") => {
                self.note.phase = NotePhase::Listening;
                let start = self.note.start;
                self.every(Duration::from_millis(200), cx, move |v, cx| {
                    if v.note.start != start || !v.note.is_on() {
                        return false;
                    }
                    if let Some(text) = v.note.tracker.take(now()) {
                        v.type_into_note(text, cx);
                        v.note.phase = NotePhase::Listening;
                        cx.notify();
                    }
                    true
                });
                log("dictating into a note");
                cx.notify();
            }
            Some("listenFailed") => self.note_failed(event["message"].as_str().unwrap_or("The microphone wouldn't start."), cx),
            Some("heard") => {
                if !self.note.is_on() {
                    return;
                }
                let heard = Heard::new(event["text"].as_str().unwrap_or(""), event["start"].as_f64().unwrap_or(0.0), event["end"].as_f64().unwrap_or(0.0), event["final"].as_bool().unwrap_or(false));
                if self.note.tracker.add(heard, now()) {
                    self.note.phase = NotePhase::Hearing(self.note.tracker.pending());
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    pub(super) fn note_level(&mut self, level: f32, cx: &mut Context<Self>) {
        let rounded = (level * 10.0).round() / 10.0;
        if rounded != self.note.level {
            self.note.level = rounded;
            cx.notify();
        }
    }

    /// Into the note through its editor, after this update: the page may be
    /// what's being updated now.
    fn type_into_note(&mut self, words: String, cx: &mut Context<Self>) {
        let (Some(page), Some(note)) = (self.note.page.upgrade(), self.note.note.clone()) else { return };
        let Some(app) = self.app.upgrade() else { return };
        let window = app.read(cx).main_window;
        cx.defer(move |cx| {
            window.update(cx, |_, window, cx| page.update(cx, |p, cx| p.insert_dictated(&note, &words, window, cx))).ok();
        });
    }
}
