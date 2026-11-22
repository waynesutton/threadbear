//! Where Tasks' sections, columns, project groups and dropped tasks go. The
//! web page has the same rules in web/src/tasks-order.ts; keep them agreeing.

pub const BOARD: [&str; 4] = ["backlog", "todo", "doing", "done"];
/// The list runs most active first; the board runs left to right.
pub const LIST: [&str; 4] = ["doing", "todo", "backlog", "done"];

/// A saved order with any status it lacks put back (Doing after Todo), so an
/// order saved before Doing existed still shows every status.
pub fn full_order(saved: &[String], fallback: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let source: Vec<String> = if saved.is_empty() {
        fallback.iter().map(|s| s.to_string()).collect()
    } else {
        saved.to_vec()
    };
    for s in source {
        if BOARD.contains(&s.as_str()) && !out.contains(&s) {
            out.push(s);
        }
    }
    for s in BOARD {
        if !out.iter().any(|o| o == s) {
            let at = if s == "doing" {
                out.iter().position(|o| o == "todo").map_or(0, |i| i + 1)
            } else {
                out.len()
            };
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
    let mut out: Vec<String> = saved
        .iter()
        .filter(|k| present.contains(k))
        .cloned()
        .collect();
    let mut rest: Vec<String> = present.into_iter().filter(|k| !out.contains(k)).collect();
    rest.sort_by_key(|k| k == "none");
    out.extend(rest);
    out
}

/// A position that sorts just before `before` among `peers` (in order), or after the last.
pub fn position_at(peers: &[(String, f64)], before: Option<&str>, now: f64) -> f64 {
    let at = before
        .and_then(|b| peers.iter().position(|(id, _)| id == b))
        .unwrap_or(peers.len());
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
    let (Some(from), Some(to)) = (
        list.iter().position(|x| x == item),
        list.iter().position(|x| x == target),
    ) else {
        return list.to_vec();
    };
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
    let Some(first) = lines.iter().position(|l| !l.trim().is_empty()) else {
        return (String::new(), String::new());
    };
    let title: String = lines[first].trim().chars().take(500).collect();
    (title, lines[first + 1..].join("\n").trim().to_string())
}

/// One line of a pasted Markdown list, as a task to make.
#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    pub title: String,
    pub notes: String,
    /// Ticked (`- [x]`), so it goes straight to Done.
    pub done: bool,
    /// The index of the item it's indented under: tasks have one level of sub-tasks,
    /// so anything deeper hangs off its top-level item too.
    pub parent: Option<usize>,
}

/// A pasted Markdown list (`-`, `*`, `+`, `1.`, `1)`, with or without `[ ]`) as one task per
/// item, or None when the text isn't a list of two or more, so it stays one task with a note.
/// Headings are skipped and indented plain lines join the note of the item above them.
pub fn split_list(text: &str) -> Option<Vec<ListItem>> {
    let text = text.replace("\r\n", "\n").replace('\t', "    ");
    let mut items: Vec<ListItem> = Vec::new();
    let mut top: Option<(usize, usize)> = None;
    for line in text.split('\n') {
        let body = line.trim_start();
        if body.trim().is_empty() || body.starts_with('#') {
            continue;
        }
        let indent = line.len() - body.len();
        let Some(rest) = list_marker(body) else {
            let last = items.last_mut().filter(|_| indent > 0)?;
            if !last.notes.is_empty() {
                last.notes.push('\n');
            }
            last.notes.push_str(body.trim_end());
            continue;
        };
        let (done, rest) = match rest.get(..3).map(|b| b.to_ascii_lowercase()) {
            Some(b)
                if (b == "[ ]" || b == "[x]")
                    && rest[3..].chars().next().is_none_or(char::is_whitespace) =>
            {
                (b == "[x]", &rest[3..])
            }
            _ => (false, rest),
        };
        let title: String = rest.trim().chars().take(500).collect();
        if title.is_empty() {
            continue;
        }
        let parent = match top {
            Some((at, top_indent)) if indent > top_indent => Some(at),
            _ => {
                top = Some((items.len(), indent));
                None
            }
        };
        items.push(ListItem {
            title,
            notes: String::new(),
            done,
            parent,
        });
    }
    (items.len() >= 2).then_some(items)
}

/// What follows a list marker and its space, or None for a line that isn't a list item.
fn list_marker(line: &str) -> Option<&str> {
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    let after =
        if digits > 0 && digits <= 9 && matches!(line.as_bytes().get(digits), Some(b'.' | b')')) {
            &line[digits + 1..]
        } else if matches!(line.as_bytes().first(), Some(b'-' | b'*' | b'+')) {
            &line[1..]
        } else {
            return None;
        };
    if after.is_empty() {
        Some(after)
    } else {
        after.strip_prefix(' ')
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn an_order_saved_before_doing_gets_it_back_after_todo() {
        assert_eq!(
            full_order(&v(&["done", "todo", "backlog"]), &BOARD),
            v(&["done", "todo", "doing", "backlog"])
        );
        assert_eq!(full_order(&[], &BOARD), v(&BOARD));
        assert_eq!(full_order(&[], &LIST), v(&LIST));
    }

    #[test]
    fn project_groups_keep_their_saved_order_with_no_project_last() {
        assert_eq!(
            group_order(&v(&["b", "a"]), &v(&["a", "none", "c", "b"])),
            v(&["b", "a", "c", "none"])
        );
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
        assert_eq!(
            move_to(&cols, "backlog", "doing"),
            v(&["todo", "doing", "backlog", "done"])
        );
        assert_eq!(
            move_to(&cols, "done", "todo"),
            v(&["backlog", "done", "todo", "doing"])
        );
    }

    #[test]
    fn the_first_typed_line_is_the_title() {
        assert_eq!(
            split_draft("Call Sam\nabout the beta\n"),
            ("Call Sam".into(), "about the beta".into())
        );
        assert_eq!(split_draft("  \n "), (String::new(), String::new()));
    }
    #[test]
    fn a_pasted_list_is_one_task_per_item() {
        let items = split_list("## Launch\n- [ ] Write the post\n  - [x] Pick a title\n    - Draft the hook\n  with a picture\n* Ship it\n\n1. Tell Sam").unwrap();
        let got: Vec<(&str, &str, bool, Option<usize>)> = items
            .iter()
            .map(|i| (i.title.as_str(), i.notes.as_str(), i.done, i.parent))
            .collect();
        assert_eq!(
            got,
            vec![
                ("Write the post", "", false, None),
                ("Pick a title", "", true, Some(0)),
                ("Draft the hook", "with a picture", false, Some(0)),
                ("Ship it", "", false, None),
                ("Tell Sam", "", false, None),
            ]
        );
    }

    #[test]
    fn text_that_isnt_a_list_stays_one_task() {
        assert_eq!(split_list("Plan the launch\n- post\n- tweet"), None);
        assert_eq!(split_list("- just one"), None);
        assert_eq!(split_list("**bold**\n---"), None);
        assert_eq!(split_list("Call Sam"), None);
    }
}
