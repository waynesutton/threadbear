//! Where Tasks' sections, columns, project groups and dropped tasks go. The
//! web page has the same rules in web/src/tasks-order.ts; keep them agreeing.

pub const BOARD: [&str; 4] = ["backlog", "todo", "doing", "done"];
/// The list runs most active first; the board runs left to right.
pub const LIST: [&str; 4] = ["doing", "todo", "backlog", "done"];

/// A saved order with any status it lacks put back (Doing after Todo), so an
/// order saved before Doing existed still shows every status.
pub fn full_order(saved: &[String], fallback: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let source: Vec<String> = if saved.is_empty() { fallback.iter().map(|s| s.to_string()).collect() } else { saved.to_vec() };
    for s in source {
        if BOARD.contains(&s.as_str()) && !out.contains(&s) {
            out.push(s);
        }
    }
    for s in BOARD {
        if !out.iter().any(|o| o == s) {
            let at = if s == "doing" { out.iter().position(|o| o == "todo").map_or(0, |i| i + 1) } else { out.len() };
            out.insert(at, s.to_string());
        }
    }
    out
}

/// Project groups to draw: the saved order first, then new ones, No project last.
pub fn group_order(saved: &[String], keys: &[String]) -> Vec<String> {
    let mut present: Vec<String> = Vec::new();
    for k in keys {
        if !present.contains(k) {
            present.push(k.clone());
        }
    }
    let mut out: Vec<String> = saved.iter().filter(|k| present.contains(k)).cloned().collect();
    let mut rest: Vec<String> = present.into_iter().filter(|k| !out.contains(k)).collect();
    rest.sort_by_key(|k| k == "none");
    out.extend(rest);
    out
}

/// A position that sorts just before `before` among `peers` (in order), or after the last.
pub fn position_at(peers: &[(String, f64)], before: Option<&str>, now: f64) -> f64 {
    let at = before.and_then(|b| peers.iter().position(|(id, _)| id == b)).unwrap_or(peers.len());
    let high = peers.get(at).map(|p| p.1);
    let low = at.checked_sub(1).and_then(|i| peers.get(i)).map(|p| p.1);
    match (low, high) {
        (low, None) => low.unwrap_or(now) + 1000.,
        (None, Some(high)) => high - 1000.,
        (Some(low), Some(high)) => (low + high) / 2.,
    }
}

/// Moves `item` to where `target` is: dragged right or down it lands after, left or up before.
pub fn move_to(list: &[String], item: &str, target: &str) -> Vec<String> {
    let (Some(from), Some(to)) = (list.iter().position(|x| x == item), list.iter().position(|x| x == target)) else { return list.to_vec() };
    if from == to {
        return list.to_vec();
    }
    let mut next: Vec<String> = list.iter().filter(|x| *x != item).cloned().collect();
    next.insert(to, item.to_string());
    next
}

/// What's typed in the quick-add box: the first line is the title, the rest the note.
pub fn split_draft(text: &str) -> (String, String) {
    let text = text.replace("\r\n", "\n");
    let lines: Vec<&str> = text.split('\n').collect();
    let Some(first) = lines.iter().position(|l| !l.trim().is_empty()) else { return (String::new(), String::new()) };
    let title: String = lines[first].trim().chars().take(500).collect();
    (title, lines[first + 1..].join("\n").trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn an_order_saved_before_doing_gets_it_back_after_todo() {
        assert_eq!(full_order(&v(&["done", "todo", "backlog"]), &BOARD), v(&["done", "todo", "doing", "backlog"]));
        assert_eq!(full_order(&[], &BOARD), v(&BOARD));
        assert_eq!(full_order(&[], &LIST), v(&LIST));
    }

    #[test]
    fn project_groups_keep_their_saved_order_with_no_project_last() {
        assert_eq!(group_order(&v(&["b", "a"]), &v(&["a", "none", "c", "b"])), v(&["b", "a", "c", "none"]));
        assert_eq!(group_order(&v(&["gone", "a"]), &v(&["a"])), v(&["a"]));
    }

    #[test]
    fn a_dropped_task_lands_between_its_neighbours() {
        let peers = vec![("a".to_string(), 100.), ("b".to_string(), 200.)];
        assert_eq!(position_at(&peers, Some("b"), 0.), 150.);
        assert_eq!(position_at(&peers, Some("a"), 0.), -900.);
        assert_eq!(position_at(&peers, None, 0.), 1200.);
    }

    #[test]
    fn a_dragged_column_lands_where_it_was_dropped() {
        let cols = v(&BOARD);
        assert_eq!(move_to(&cols, "backlog", "doing"), v(&["todo", "doing", "backlog", "done"]));
        assert_eq!(move_to(&cols, "done", "todo"), v(&["backlog", "done", "todo", "doing"]));
    }

    #[test]
    fn the_first_typed_line_is_the_title() {
        assert_eq!(split_draft("Call Sam\nabout the beta\n"), ("Call Sam".into(), "about the beta".into()));
        assert_eq!(split_draft("  \n "), (String::new(), String::new()));
    }
}
