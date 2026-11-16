// Voice's rules (src/voice.rs, src/voice_keys.rs), ported with the Swift
// app's VoiceTests.swift and VoiceKeysTests.swift so both apps agree.
use serde_json::json;
use threadbear_core::reducer::event::{ApprovalDecision as D, ApprovalOption as O, ApprovalRequest, StartRequest};
use threadbear_core::voice::*;
use threadbear_core::voice_keys::*;

// ---- SpokenText ----

#[test]
fn markdown_is_read_as_words() {
    let reply = "## Dinner tonight\n\nTry **lemon pasta**. It takes [20 minutes](https://example.com/recipe).\n\n- Boil the `pasta`\n- Zest a lemon";
    assert_eq!(plain(reply), "Dinner tonight. Try lemon pasta. It takes 20 minutes. Boil the pasta. Zest a lemon.");
}

#[test]
fn code_and_tables_are_left_for_the_screen() {
    let reply = "Here it is:\n\n```swift\nlet x = 1\nlet y = 2\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nDone.";
    assert_eq!(sentences(reply), vec![format!("Here it is: {CODE_LINE}"), TABLE_LINE.to_owned(), "Done.".to_owned()]);
}

#[test]
fn bare_links_are_not_spelled_out() {
    assert_eq!(plain("See https://threadbear.mikecann.app/docs for more"), "See a link for more.");
}

#[test]
fn sentences_split_at_stops() {
    assert_eq!(sentences("One. Two? Three! Version 1.5 is out."), vec!["One.", "Two?", "Three!", "Version 1.5 is out."]);
}

#[test]
fn long_sentences_are_cut() {
    let long = vec!["word"; 80].join(" ");
    let parts = sentences_within(&long, 60);
    assert!(parts.len() > 1);
    assert!(parts.iter().all(|p| p.chars().count() <= 60));
    assert_eq!(parts.join(" "), format!("{long}."));
}

#[test]
fn nothing_to_say() {
    assert!(sentences("  \n---\n").is_empty());
}

#[test]
fn pictures_go_and_emphasis_keeps_its_words() {
    assert_eq!(plain("![shot](a.png) A _quiet_ ~~old~~ **new** one"), "A quiet old new one.");
    assert_eq!(plain("> 1. [x] done it"), "done it.");
}

// ---- VoiceCatalog ----

#[test]
fn emily_and_mike_come_first() {
    let names: Vec<String> = voices().into_iter().take(2).map(|v| v.name).collect();
    assert_eq!(names, vec!["Emily", "Mike"]);
    assert_eq!(voice(Some("mike")).engine_voice, "michael");
    assert_eq!(voice(Some("nobody")), voices()[0]);
    let ids: std::collections::HashSet<_> = voices().iter().map(|v| v.id).collect();
    assert_eq!(ids.len(), voices().len());
}

#[test]
fn pause_defaults_to_one_and_a_half_and_snaps_to_an_offer() {
    assert_eq!(VoicePrefs::default().pause(), 1.5);
    let prefs: VoicePrefs = serde_json::from_value(json!({ "voicePause": 2.2, "voiceInNotes": true })).unwrap();
    assert_eq!(prefs.pause(), 2.0);
    assert!(prefs.in_notes());
    assert!(!VoicePrefs::default().in_notes());
    assert_eq!(pause_title(1.0), "1 second");
    assert_eq!(pause_title(1.5), "1.5 seconds");
}

#[test]
fn speeds() {
    assert_eq!(speed_title(1.0), "1x");
    assert_eq!(speed_title(1.25), "1.25x");
    assert_eq!(speed(None), 1.0);
    assert_eq!(speed(Some(1.3)), 1.25);
    assert_eq!(speed(Some(9.0)), 2.0);
}

#[test]
fn language_falls_back_to_english() {
    assert_eq!(language(Some("klingon")).id, "english");
    assert_eq!(language(Some("french")).locale, "fr-FR");
}

#[test]
fn prefs_use_the_swift_apps_keys() {
    let prefs: VoicePrefs = serde_json::from_value(json!({ "voiceEnabled": false, "voiceIcon": "waveform", "voiceKeyTrigger": "rightOption", "voiceKeyShift": false, "voiceCommandMode": true, "voiceKeyCommand": "shift" })).unwrap();
    assert!(!prefs.enabled());
    assert_eq!(prefs.icon(), VoiceIcon::Waveform);
    let keys = prefs.keys();
    assert_eq!(keys.trigger, VoiceTriggerKey::RightOption);
    assert!(!keys.shift_toggle);
    assert!(!keys.double_tap);
    assert!(keys.hold_to_dictate);
    assert!(keys.command_mode);
    assert_eq!(keys.command_display(), "Right ⌥ ⇧");
    assert_eq!(VoicePrefs::default().keys(), VoiceKeys::default());
    assert!(VoicePrefs::default().enabled());
    assert_eq!(VoiceIcon::saved(Some("nonsense")), VoiceIcon::Microphone);
    assert_eq!(serde_json::to_value(VoicePrefs::default()).unwrap(), json!({}));
}

// ---- UtteranceTracker ----

fn r(text: &str, start: f64, end: f64, is_final: bool) -> Heard {
    Heard::new(text, start, end, is_final)
}

#[test]
fn sends_after_a_pause() {
    let mut t = UtteranceTracker::new(1.0);
    assert!(t.add(r("what should", 0.0, 1.0, false), 10.0));
    t.add(r("what should I cook", 0.0, 1.6, false), 10.5);
    assert_eq!(t.pending(), "what should I cook");
    assert_eq!(t.take(11.2), None);
    assert_eq!(t.take(11.6).as_deref(), Some("what should I cook"));
    assert_eq!(t.take(20.0), None);
}

#[test]
fn final_replaces_the_guess_it_covers() {
    let mut t = UtteranceTracker::new(1.0);
    t.add(r("what should I cook", 0.0, 1.6, false), 0.0);
    t.add(r("What should I cook?", 0.0, 1.6, true), 0.2);
    t.add(r("tonight", 1.8, 2.3, false), 0.4);
    assert_eq!(t.pending(), "What should I cook? tonight");
}

#[test]
fn late_final_for_sent_audio_is_dropped() {
    let mut t = UtteranceTracker::new(1.0);
    t.add(r("hello there", 0.0, 1.0, false), 0.0);
    assert_eq!(t.take(1.5).as_deref(), Some("hello there"));
    assert!(!t.add(r("Hello there.", 0.0, 1.0, true), 1.7));
    assert!(!t.has_words());
    assert!(t.add(r("and more", 1.2, 2.0, false), 1.8));
    assert_eq!(t.pending(), "and more");
}

#[test]
fn revisions_of_what_went_are_not_sent_again() {
    // Fn dictation, 8 Nov: "Is this working? Is this working?"
    let mut t = UtteranceTracker::new(0.8);
    t.add(r("is this working", 0.0, 1.0, false), 0.0);
    assert_eq!(t.take(1.0).as_deref(), Some("is this working"));
    assert!(!t.add(r("is this working", 0.0, 1.3, false), 1.1));
    assert!(!t.add(r("Is this working?", 0.0, 1.5, true), 1.2));
    assert_eq!(t.take(5.0), None);
    assert_eq!(t.flush(), None);
}

#[test]
fn new_words_on_a_revision_still_count() {
    let mut t = UtteranceTracker::new(0.8);
    t.add(r("hello do you", 0.0, 1.0, false), 0.0);
    assert_eq!(t.take(1.0).as_deref(), Some("hello do you"));
    assert!(t.add(r("Hello, do you hear me?", 0.0, 1.8, true), 1.2));
    assert_eq!(t.flush().as_deref(), Some("hear me?"));
}

#[test]
fn silence_alone_sends_nothing() {
    let mut t = UtteranceTracker::new(1.0);
    assert!(!t.add(r(" ", 0.0, 1.0, true), 0.0));
    assert_eq!(t.take(5.0), None);
    assert_eq!(t.flush(), None);
}

#[test]
fn the_tracker_says_where_whats_waiting_starts() {
    let mut t = UtteranceTracker::new(1.0);
    assert_eq!(t.pending_start(), None);
    t.add(r("yes", 4.5, 5.0, true), 0.0);
    t.add(r("go ahead", 5.2, 6.0, false), 0.0);
    assert_eq!(t.pending_start(), Some(4.5));
    t.flush();
    assert_eq!(t.pending_start(), None);
}

// ---- SpokenAnswer ----

const ALL: [O; 3] = [O::Allow, O::AllowAlways, O::Deny];

#[test]
fn yes() {
    for said in ["Yes.", "yeah go ahead", "Go ahead, please.", "Sure", "OK, do it!", "yes thank you", "Um, yes."] {
        assert_eq!(spoken_decision(said, &ALL), Some(D::Allow), "{said}");
    }
}

#[test]
fn no() {
    for said in ["No.", "Nope", "Don't do that.", "no thanks", "Cancel"] {
        assert_eq!(spoken_decision(said, &ALL), Some(D::Deny), "{said}");
    }
}

#[test]
fn always() {
    assert_eq!(spoken_decision("Always allow it", &ALL), Some(D::AllowAlways));
    assert_eq!(spoken_decision("always allow", &[O::Allow, O::Deny]), None);
}

#[test]
fn anything_more_is_a_message() {
    for said in ["No, use the other folder instead", "yes but only for the tests folder", "what does that command do", "thank you", ""] {
        assert_eq!(spoken_decision(said, &ALL), None, "{said}");
    }
}

#[test]
fn questions_are_not_answers() {
    for said in ["Do you always do that?", "Is that always on?", "Is that it?", "Do it for that?", "Go for it on that", "Not that way", "It's fine", "good", "go", "always", "do", "You always allow it?", "You approve it", "Okay?", "you always allow it", "do you"] {
        assert_eq!(spoken_decision(said, &ALL), None, "{said}");
    }
}

#[test]
fn whole_answers() {
    assert_eq!(spoken_decision("Yes, go ahead and do it.", &ALL), Some(D::Allow));
    assert_eq!(spoken_decision("Yes, always allow.", &ALL), Some(D::AllowAlways));
    assert_eq!(spoken_decision("No, don't do that.", &ALL), Some(D::Deny));
}

// ---- SpokenApprovals ----

fn card(id: &str, command: &str) -> ApprovalRequest {
    ApprovalRequest::new(id, "Bash", command, json!({ "command": command }))
}

fn card_with(id: &str, options: Vec<O>) -> ApprovalRequest {
    let mut c = card(id, "npm test");
    c.options = options;
    c
}

/// A read-out finishing at 10, and him speaking at 11 with the app quiet since 10.
fn answer(a: &mut SpokenApprovals, said: &str) -> SpokenOutcome {
    a.take(said, 11.0, 10.0)
}

#[test]
fn nothing_waiting_is_a_message() {
    let mut a = SpokenApprovals::default();
    assert_eq!(answer(&mut a, "yes"), SpokenOutcome::Message);
}

#[test]
fn a_yes_answers_the_card_read_out() {
    let mut a = SpokenApprovals::default();
    let first = card("a", "npm test");
    a.requested(first.clone());
    a.read_out(&first, 10.0);
    assert_eq!(answer(&mut a, "yes"), SpokenOutcome::Answer(first, D::Allow));
    assert!(a.open().is_empty());
    assert_eq!(answer(&mut a, "yes"), SpokenOutcome::Message, "answered once, and that's it");
}

#[test]
fn before_it_is_read_out_a_yes_answers_nothing() {
    let mut a = SpokenApprovals::default();
    let first = card("a", "npm test");
    a.requested(first.clone());
    assert_eq!(answer(&mut a, "yes"), SpokenOutcome::ReadAgain(first.clone()));
    assert_eq!(a.open(), &[first]);
}

#[test]
fn a_new_card_just_before_he_speaks_does_not_take_his_yes() {
    let mut a = SpokenApprovals::default();
    let (first, second) = (card("a", "npm test"), card("b", "rm -rf build"));
    a.requested(first.clone());
    a.read_out(&first, 10.0);
    a.requested(second.clone());
    assert_eq!(answer(&mut a, "yes"), SpokenOutcome::ReadAgain(second.clone()));
    assert_eq!(a.open(), &[first, second]);
}

#[test]
fn a_yes_said_before_the_card_was_read_out_does_not_answer_it() {
    let mut a = SpokenApprovals::default();
    let (first, second) = (card("a", "npm test"), card("b", "npm test"));
    a.requested(first.clone());
    a.read_out(&first, 10.0);
    a.requested(second.clone());
    a.read_out(&second, 20.0);
    assert_eq!(a.take("yes", 11.0, 20.0), SpokenOutcome::ReadAgain(second.clone()));
    assert_eq!(a.open(), &[first, second]);
}

#[test]
fn words_said_while_it_talks_do_not_answer() {
    let mut a = SpokenApprovals::default();
    let first = card("a", "npm test");
    a.requested(first.clone());
    a.read_out(&first, 10.0);
    assert_eq!(a.take("yes", 11.0, f64::INFINITY), SpokenOutcome::ReadAgain(first.clone()), "still talking");
    a.read_out(&first, 10.0);
    assert_eq!(a.take("yes", 11.0, 12.0), SpokenOutcome::ReadAgain(first.clone()), "stopped after he started");
    a.read_out(&first, 10.0);
    assert_eq!(a.take("yes", f64::NEG_INFINITY, 10.0), SpokenOutcome::ReadAgain(first.clone()), "no idea when it was said");
    a.read_out(&first, 10.0);
    assert_eq!(a.take("yes", 12.0, 12.0), SpokenOutcome::Answer(first, D::Allow));
}

#[test]
fn an_older_card_finishing_its_read_out_does_not_bind_over_a_newer_one() {
    let mut a = SpokenApprovals::default();
    let (first, second) = (card("a", "npm test"), card("b", "npm test"));
    a.requested(first.clone());
    a.requested(second.clone());
    a.read_out(&first, 10.0);
    assert_eq!(a.heard(), None);
    assert_eq!(answer(&mut a, "yes"), SpokenOutcome::ReadAgain(second.clone()));
    a.read_out(&second, 10.0);
    assert_eq!(answer(&mut a, "yes"), SpokenOutcome::Answer(second, D::Allow));
    assert_eq!(answer(&mut a, "yes"), SpokenOutcome::ReadAgain(first.clone()));
    a.read_out(&first, 10.0);
    assert_eq!(answer(&mut a, "no"), SpokenOutcome::Answer(first, D::Deny));
}

#[test]
fn a_gone_card_is_not_answered() {
    let mut a = SpokenApprovals::default();
    let first = card("a", "npm test");
    a.requested(first.clone());
    a.read_out(&first, 10.0);
    a.resolved("a");
    assert_eq!(answer(&mut a, "yes"), SpokenOutcome::Message);
}

#[test]
fn a_changed_card_must_be_heard_again() {
    let mut a = SpokenApprovals::default();
    let (first, changed) = (card("a", "npm test"), card("a", "rm -rf build"));
    a.requested(first.clone());
    a.read_out(&first, 10.0);
    a.requested(changed.clone());
    assert_eq!(answer(&mut a, "yes"), SpokenOutcome::ReadAgain(changed.clone()));
    a.read_out(&first, 10.0);
    assert_eq!(answer(&mut a, "yes"), SpokenOutcome::ReadAgain(changed.clone()));
    a.read_out(&changed, 10.0);
    assert_eq!(answer(&mut a, "yes"), SpokenOutcome::Answer(changed, D::Allow));
}

#[test]
fn always_only_where_the_card_offers_it() {
    let mut a = SpokenApprovals::default();
    let first = card_with("a", vec![O::Allow, O::Deny]);
    a.requested(first.clone());
    a.read_out(&first, 10.0);
    assert_eq!(answer(&mut a, "always allow"), SpokenOutcome::ReadAgain(first.clone()));
    a.read_out(&first, 10.0);
    assert_eq!(answer(&mut a, "yes"), SpokenOutcome::Answer(first, D::Allow));
}

#[test]
fn longer_words_are_a_message() {
    let mut a = SpokenApprovals::default();
    let first = card("a", "npm test");
    a.requested(first.clone());
    a.read_out(&first, 10.0);
    assert_eq!(answer(&mut a, "no, use the other folder"), SpokenOutcome::Message);
    assert_eq!(a.heard(), Some(&first), "still the card he heard");
}

#[test]
fn a_card_the_words_cannot_cover_is_left_to_the_screen() {
    let hidden = "echo checking\nrm -rf important";
    let cards = vec![
        ApprovalRequest::new("a", "Bash", "echo checking", json!({ "command": hidden })),
        ApprovalRequest::new("b", "Bash", "echo checking", json!({})),
        ApprovalRequest::new("c", "Edit", "Edit a.ts", json!({ "file_path": "a.ts" })),
        ApprovalRequest::new("d", "mcp__x__do", "x: do", json!({})),
        ApprovalRequest::new("e", StartRequest::CARD_NAME, "Start a thread", json!({})),
        ApprovalRequest::new("f", "ExitPlanMode", "Finish the plan", json!({})),
        card("g", &"x".repeat(161)),
        card("h", "echo checking; rm -rf important"),
        card("i", "cat $(echo secret)"),
        card("j", "./safe\u{200B}"),
        card("k", "c\u{0430}t notes"),
        ApprovalRequest::new("l", "Read", "Read .env", json!({ "file_path": "/production/.env" })),
    ];
    for each in cards {
        let mut a = SpokenApprovals::default();
        a.requested(each.clone());
        assert_eq!(SpokenApprovals::reading(&each), None, "{}", each.approval_id);
        a.read_out(&each, 10.0);
        assert_eq!(a.heard(), None, "{}", each.approval_id);
        assert_eq!(answer(&mut a, "yes"), SpokenOutcome::OnScreen(each.clone()), "{}", each.approval_id);
        assert_eq!(a.open(), &[each]);
    }
}

#[test]
fn a_one_line_command_is_read_as_it_stands() {
    let command = "npm run test:unit -- --filter=a.b/c,d";
    assert_eq!(SpokenApprovals::reading(&card("a", command)), Some(command));
}

#[test]
fn clear_forgets_all() {
    let mut a = SpokenApprovals::default();
    let first = card("a", "npm test");
    a.requested(first.clone());
    a.read_out(&first, 10.0);
    a.clear();
    assert_eq!(answer(&mut a, "yes"), SpokenOutcome::Message);
}

// ---- DictatedText ----

#[test]
fn words_are_spaced_from_the_text_around_them() {
    assert_eq!(insertion("Buy milk.", None, None).as_deref(), Some("Buy milk."));
    assert_eq!(insertion("and eggs", Some('k'), None).as_deref(), Some(" and eggs"));
    assert_eq!(insertion("and eggs", Some(' '), None).as_deref(), Some("and eggs"));
    assert_eq!(insertion("fresh", Some(' '), Some('m')).as_deref(), Some("fresh "));
    assert_eq!(insertion("today", Some(' '), Some('.')).as_deref(), Some("today"));
}

#[test]
fn a_line_starts_with_a_capital() {
    assert_eq!(insertion("call mum", Some('\n'), None).as_deref(), Some("Call mum"));
    assert_eq!(insertion("call mum", None, None).as_deref(), Some("Call mum"));
    assert_eq!(insertion("call mum", Some(' '), None).as_deref(), Some("call mum"));
}

#[test]
fn new_line_on_its_own_is_a_line_break() {
    assert_eq!(insertion("New line.", Some('k'), None).as_deref(), Some("\n"));
    assert_eq!(insertion("new paragraph", Some('k'), None).as_deref(), Some("\n\n"));
    assert_eq!(insertion("Start a new line here", None, None).as_deref(), Some("Start a new line here"));
}

#[test]
fn nothing_said_is_nothing_inserted() {
    assert_eq!(insertion("  ", Some('a'), None), None);
}

// ---- VoiceKeyMachine ----

fn machine(edit: impl FnOnce(&mut VoiceKeys)) -> VoiceKeyMachine {
    let mut keys = VoiceKeys::default();
    edit(&mut keys);
    VoiceKeyMachine::new(keys)
}

fn plain_machine() -> VoiceKeyMachine {
    machine(|_| {})
}

use KeyAction as A;
use KeyEvent as E;

#[test]
fn hold_dictates_and_release_types_it_in() {
    let mut m = plain_machine();
    assert_eq!(m.handle(E::TriggerDown(0.0)).actions, vec![A::WaitToHold(0.0)]);
    assert_eq!(m.handle(E::HoldDelayPassed(0.0)).actions, vec![A::Start(KeyMode::Dictating)]);
    assert_eq!(m.mode(), KeyMode::Dictating);
    assert_eq!(m.handle(E::TriggerUp(2.0)).actions, vec![A::Finish]);
    assert_eq!(m.mode(), KeyMode::Idle);
}

#[test]
fn a_tap_never_listens() {
    let mut m = plain_machine();
    m.handle(E::TriggerDown(0.0));
    assert!(m.handle(E::TriggerUp(0.1)).actions.is_empty());
    assert!(m.handle(E::HoldDelayPassed(0.0)).actions.is_empty());
    assert_eq!(m.mode(), KeyMode::Idle);
}

#[test]
fn double_tap_starts_hands_free_and_a_tap_stops_it() {
    let mut m = machine(|k| k.double_tap = true);
    m.handle(E::TriggerDown(0.0));
    m.handle(E::TriggerUp(0.1));
    assert_eq!(m.handle(E::TriggerDown(0.3)).actions, vec![A::Start(KeyMode::HandsFree)]);
    assert!(m.handle(E::TriggerUp(0.4)).actions.is_empty());
    assert_eq!(m.mode(), KeyMode::HandsFree);
    assert_eq!(m.handle(E::KeyDown(Key::Other)), KeyOutcome::default());
    m.handle(E::TriggerDown(5.0));
    assert_eq!(m.handle(E::TriggerUp(5.1)).actions, vec![A::Finish]);
    assert_eq!(m.mode(), KeyMode::Idle);
}

#[test]
fn taps_too_far_apart_are_not_a_double_tap() {
    let mut m = plain_machine();
    m.handle(E::TriggerDown(0.0));
    m.handle(E::TriggerUp(0.1));
    assert_eq!(m.handle(E::TriggerDown(1.0)).actions, vec![A::WaitToHold(1.0)]);
}

#[test]
fn double_tap_is_off_by_default() {
    let mut m = plain_machine();
    m.handle(E::TriggerDown(0.0));
    m.handle(E::TriggerUp(0.1));
    assert_eq!(m.handle(E::TriggerDown(0.3)).actions, vec![A::WaitToHold(0.3)]);
}

#[test]
fn trigger_space_toggles_hands_free_and_swallows_the_space() {
    let mut m = plain_machine();
    m.handle(E::TriggerDown(0.0));
    assert_eq!(m.handle(E::KeyDown(Key::Space)), KeyOutcome { actions: vec![A::Start(KeyMode::HandsFree)], swallow: true });
    assert!(m.handle(E::TriggerUp(0.5)).actions.is_empty());
    assert_eq!(m.mode(), KeyMode::HandsFree);
    m.handle(E::TriggerDown(3.0));
    assert_eq!(m.handle(E::KeyDown(Key::Space)), KeyOutcome { actions: vec![A::Finish], swallow: true });
    assert!(m.handle(E::TriggerUp(3.2)).actions.is_empty());
    assert_eq!(m.mode(), KeyMode::Idle);
}

#[test]
fn shift_trigger_toggles_hands_free() {
    let mut m = plain_machine();
    m.handle(E::TriggerDown(0.0));
    assert_eq!(m.handle(E::ShiftDown), KeyOutcome { actions: vec![A::Start(KeyMode::HandsFree)], swallow: false });
    assert!(m.handle(E::TriggerUp(0.3)).actions.is_empty());
    assert_eq!(m.mode(), KeyMode::HandsFree);
    m.handle(E::TriggerDown(4.0));
    assert_eq!(m.handle(E::ShiftDown).actions, vec![A::Finish]);
    assert!(m.handle(E::TriggerUp(4.2)).actions.is_empty());
    assert_eq!(m.mode(), KeyMode::Idle);
}

#[test]
fn shift_can_be_turned_off() {
    let mut m = machine(|k| k.shift_toggle = false);
    m.handle(E::TriggerDown(0.0));
    assert!(m.handle(E::ShiftDown).actions.is_empty());
    assert!(m.handle(E::HoldDelayPassed(0.0)).actions.is_empty());
}

#[test]
fn space_during_a_hold_carries_on_hands_free() {
    let mut m = plain_machine();
    m.handle(E::TriggerDown(0.0));
    m.handle(E::HoldDelayPassed(0.0));
    assert_eq!(m.handle(E::KeyDown(Key::Space)).actions, vec![A::Start(KeyMode::HandsFree)]);
    assert!(m.handle(E::TriggerUp(1.0)).actions.is_empty());
    assert_eq!(m.mode(), KeyMode::HandsFree);
}

#[test]
fn space_without_the_trigger_is_just_a_space() {
    let mut m = plain_machine();
    assert_eq!(m.handle(E::KeyDown(Key::Space)), KeyOutcome::default());
}

#[test]
fn fn_as_a_modifier_never_listens() {
    let mut m = plain_machine();
    m.handle(E::TriggerDown(0.0));
    assert_eq!(m.handle(E::KeyDown(Key::Other)), KeyOutcome::default());
    assert!(m.handle(E::HoldDelayPassed(0.0)).actions.is_empty());
    assert!(m.handle(E::TriggerUp(1.0)).actions.is_empty());
    m.handle(E::TriggerDown(1.1));
    assert_eq!(m.mode(), KeyMode::Idle);
}

#[test]
fn a_key_during_a_hold_cancels_it() {
    let mut m = plain_machine();
    m.handle(E::TriggerDown(0.0));
    m.handle(E::HoldDelayPassed(0.0));
    assert_eq!(m.handle(E::KeyDown(Key::Other)).actions, vec![A::Cancel]);
    assert!(m.handle(E::TriggerUp(1.0)).actions.is_empty());
}

#[test]
fn esc_cancels_and_is_swallowed_only_while_listening() {
    let mut m = plain_machine();
    assert_eq!(m.handle(E::KeyDown(Key::Escape)), KeyOutcome::default());
    m.handle(E::TriggerDown(0.0));
    m.handle(E::HoldDelayPassed(0.0));
    assert_eq!(m.handle(E::KeyDown(Key::Escape)), KeyOutcome { actions: vec![A::Cancel], swallow: true });
    assert!(m.handle(E::TriggerUp(1.0)).actions.is_empty());
    m.handle(E::TriggerDown(5.0));
    m.handle(E::KeyDown(Key::Space));
    m.handle(E::TriggerUp(5.2));
    assert_eq!(m.handle(E::KeyDown(Key::Escape)), KeyOutcome { actions: vec![A::Cancel], swallow: true });
    assert_eq!(m.mode(), KeyMode::Idle);
}

#[test]
fn command_mode_only_when_turned_on() {
    let mut off = plain_machine();
    off.handle(E::TriggerDown(0.0));
    assert!(off.handle(E::CommandKeyDown).actions.is_empty());
    assert!(off.handle(E::HoldDelayPassed(0.0)).actions.is_empty());

    let mut on = machine(|k| k.command_mode = true);
    on.handle(E::TriggerDown(0.0));
    assert_eq!(on.handle(E::CommandKeyDown).actions, vec![A::Start(KeyMode::Command)]);
    assert!(on.handle(E::HoldDelayPassed(0.0)).actions.is_empty());
    assert_eq!(on.handle(E::TriggerUp(2.0)).actions, vec![A::Finish]);
}

#[test]
fn command_key_during_a_hold_switches_it() {
    let mut m = machine(|k| k.command_mode = true);
    m.handle(E::TriggerDown(0.0));
    m.handle(E::HoldDelayPassed(0.0));
    assert_eq!(m.handle(E::CommandKeyDown).actions, vec![A::Start(KeyMode::Command)]);
    assert_eq!(m.mode(), KeyMode::Command);
}

#[test]
fn hold_can_be_turned_off() {
    let mut m = machine(|k| k.hold_to_dictate = false);
    assert!(m.handle(E::TriggerDown(0.0)).actions.is_empty());
    assert!(m.handle(E::HoldDelayPassed(0.0)).actions.is_empty());
    assert_eq!(m.handle(E::KeyDown(Key::Space)).actions, vec![A::Start(KeyMode::HandsFree)]);
}

#[test]
fn ended_elsewhere_leaves_the_held_key_doing_nothing() {
    let mut m = plain_machine();
    m.handle(E::TriggerDown(0.0));
    m.handle(E::HoldDelayPassed(0.0));
    m.ended();
    assert_eq!(m.mode(), KeyMode::Idle);
    assert!(m.handle(E::TriggerUp(1.0)).actions.is_empty());
}

#[test]
fn key_caps() {
    let keys = VoiceKeys::default();
    assert_eq!(keys.shift_display(), "⇧ Fn");
    assert_eq!(keys.hold_display(), "Hold Fn");
    assert_eq!(keys.hands_free_display(), "Fn Space");
    assert_eq!(keys.listening_line(KeyMode::HandsFree), "Hands-free: ⇧ Fn to stop, Esc to cancel");
    assert_eq!(keys.listening_line(KeyMode::Dictating), "Listening: let go of Fn to type it in. Esc cancels.");
}
