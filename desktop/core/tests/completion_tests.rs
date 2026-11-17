mod support;

use serde_json::json;
use support::*;
use threadbear_core::reducer::*;

fn completion(needs: &str, tasks: serde_json::Value) -> Event {
    event_turn(3, "turn.completed", json!({
        "turnId": "t1", "outcome": "completed", "needs": needs,
        "asks": if needs == "you" { json!([{"action":"Try the installed app", "options":["It works", "Needs changes"]}]) } else { json!([]) },
        "completion": {"tasks": tasks}
    }), Some("t1"))
}

fn open_card(model: &TranscriptModel) -> &AsksCard {
    let RowContent::Asks(card) = &model.open_card().expect("an open card").content else { panic!("asks card") };
    card
}

#[test]
fn incomplete_tasks_keep_the_needs_you_actions() {
    let model = from_events(vec![completion("you", json!([
        {"id":"build","title":"Build the card","status":"completed"},
        {"id":"try","title":"Try the installed app","status":"pending"},
        {"id":"deploy","title":"Deploy the app","status":"blocked"}
    ]))]);
    let card = open_card(&model);
    let summary = card.completion.as_ref().unwrap();
    assert_eq!(summary.heading(), "1 of 3 tasks completed");
    assert_eq!(summary.tasks[2].status.label(), "Blocked");
    assert_eq!(card.asks[0].options, ["It works", "Needs changes"]);
    assert!(!card.is_done());
    assert!(card.loose_ends.is_none(), "task completion makes no repository-check claim");
}

#[test]
fn fully_completed_tasks_have_a_clear_count_on_the_done_card() {
    let model = from_events(vec![completion("nothing", json!([
        {"id":"build","title":"Build the card","status":"completed"},
        {"id":"try","title":"Try the installed app","status":"completed"}
    ]))]);
    let card = open_card(&model);
    assert!(card.is_done());
    assert_eq!(card.completion.as_ref().unwrap().heading(), "All 2 tasks completed");
    assert!(card.loose_ends.is_none());
}

#[test]
fn missing_empty_or_unknown_completion_keeps_the_original_card() {
    for tasks in [json!([]), json!([{"id":"new","title":"Future status","status":"future"}]), json!([{"title":"Missing id","status":"completed"}])] {
        let model = from_events(vec![completion("nothing", tasks)]);
        assert!(open_card(&model).completion.is_none());
    }
    let model = from_events(vec![event_turn(3, "turn.completed", json!({"turnId":"t1","outcome":"completed","needs":"nothing"}), Some("t1"))]);
    assert!(open_card(&model).completion.is_none());
}

#[test]
fn new_turns_show_the_latest_request_summary() {
    let mut model = from_events(vec![completion("nothing", json!([
        {"id":"build","title":"Build the card","status":"completed"}
    ]))]);
    model.apply(vec![event_turn(4, "turn.completed", json!({
        "turnId":"t2","outcome":"completed","needs":"nothing",
        "completion":{"tasks":[
            {"id":"build","title":"Build the card","status":"completed"},
            {"id":"docs","title":"Write the docs","status":"pending"}
        ]}
    }), Some("t2"))]);
    assert_eq!(open_card(&model).completion.as_ref().unwrap().heading(), "1 of 2 tasks completed");
}

#[test]
fn completed_task_status_does_not_substitute_for_a_runner_check() {
    let mut model = from_events(vec![completion("nothing", json!([
        {"id":"build","title":"Build the card","status":"completed"}
    ]))]);
    assert_eq!(open_card(&model).completion.as_ref().unwrap().heading(), "1 task completed");
    assert!(open_card(&model).loose_ends.is_none());
    model.set_loose_ends(Some(LooseEnds { seq: 3, ends: vec![LooseEnd::new("unlanded", "The branch still needs landing")] }));
    assert_eq!(open_card(&model).loose_ends.as_ref().unwrap().len(), 1);
    model.set_loose_ends(Some(LooseEnds { seq: 3, ends: vec![] }));
    assert!(open_card(&model).loose_ends.as_ref().unwrap().is_empty());
}
