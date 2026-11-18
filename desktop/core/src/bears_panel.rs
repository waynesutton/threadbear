// The Bears & Tables panel on the right rail (gpui's bears_panel.rs): its
// bears as `bears.list { archived: true }` gives them, which section each
// sits in, the rows picked with ⌘-click and Shift-click, and the words of
// its Delete sheet. Tables are projects with a `table` (protocol "Tables"),
// from the projects list the app already keeps.
use serde_json::Value;

/// A bear as the panel draws it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PanelBear {
    pub id: String,
    pub name: String,
    pub does: Option<String>,
    pub icon: Option<String>,
    pub folder: String,
    pub head: bool,
    pub pinned: bool,
    pub archived: bool,
    /// While it's away (hibernating), until when.
    pub away_until: Option<String>,
    /// Its routines that aren't paused.
    pub routines: usize,
}

pub const PINNED: &str = "Pinned";
pub const BEARS: &str = "Bears";
pub const TABLES: &str = "Tables";
pub const ARCHIVED: &str = "Archived";

impl PanelBear {
    /// Archived ones last, the head bear and pinned ones first, as the patch
    /// panel had them.
    pub fn section(&self) -> &'static str {
        if self.archived {
            ARCHIVED
        } else if self.head || self.pinned {
            PINNED
        } else {
            BEARS
        }
    }

    pub fn title(&self) -> String {
        if self.head { format!("{} (head bear)", self.name) } else { self.name.clone() }
    }
}

/// `bears.list`'s answer, in its order, or none while bears are off. One
/// that won't read is left out, not the rest.
pub fn decode(v: &Value) -> Option<Vec<PanelBear>> {
    if v["on"].as_bool() != Some(true) {
        return None;
    }
    let text = |b: &Value, key: &str| b[key].as_str().filter(|s| !s.is_empty()).map(str::to_owned);
    Some(
        v["bears"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|b| {
                Some(PanelBear {
                    id: text(b, "id")?,
                    name: text(b, "name")?,
                    does: text(b, "does"),
                    icon: text(b, "icon"),
                    folder: text(b, "folder").unwrap_or_default(),
                    head: b["head"].as_bool() == Some(true),
                    pinned: b["state"]["pinned"].is_number(),
                    archived: b["archived"].as_bool() == Some(true),
                    away_until: (b["away"].as_bool() == Some(true)).then(|| text(&b["state"], "awayUntil")).flatten(),
                    routines: b["routines"].as_array().map_or(0, |r| r.iter().filter(|r| r["paused"].as_bool() != Some(true)).count()),
                })
            })
            .collect(),
    )
}

/// The order of the sections down the panel.
pub fn section_rank(section: &str) -> usize {
    [PINNED, BEARS, TABLES, ARCHIVED].iter().position(|s| *s == section).unwrap_or(4)
}

/// A row's key in the selection: "bear:<id>" or "table:<project id>".
pub fn bear_key(id: &str) -> String {
    format!("bear:{id}")
}
pub fn table_key(id: &str) -> String {
    format!("table:{id}")
}

/// How a row was clicked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    /// Opens it, and drops what was picked.
    Plain,
    /// ⌘-click: in or out of what's picked.
    Toggle,
    /// Shift-click: everything from the last row clicked to this one.
    Range,
}

/// The rows picked, in the order they were picked, and the row the next
/// Shift-click counts from.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Selection {
    pub keys: Vec<String>,
    pub anchor: Option<String>,
}

impl Selection {
    /// A click on `key`, with the rows as they're drawn. True when the row
    /// is to open (a plain click).
    pub fn click(&mut self, key: &str, click: Click, order: &[String]) -> bool {
        match click {
            Click::Plain => {
                self.keys.clear();
                self.anchor = Some(key.to_owned());
                true
            }
            Click::Toggle => {
                if let Some(i) = self.keys.iter().position(|k| k == key) {
                    self.keys.remove(i);
                } else {
                    self.keys.push(key.to_owned());
                }
                self.anchor = Some(key.to_owned());
                false
            }
            Click::Range => {
                let to = order.iter().position(|k| k == key);
                let from = self.anchor.as_ref().and_then(|a| order.iter().position(|k| k == a)).or(to);
                if let (Some(from), Some(to)) = (from, to) {
                    let (a, b) = if from <= to { (from, to) } else { (to, from) };
                    self.keys = order[a..=b].to_vec();
                }
                false
            }
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        self.keys.iter().any(|k| k == key)
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.anchor = None;
    }

    /// Drops rows that have gone (deleted here or elsewhere).
    pub fn retain(&mut self, order: &[String]) {
        self.keys.retain(|k| order.contains(k));
        if self.anchor.as_ref().is_some_and(|a| !order.contains(a)) {
            self.anchor = None;
        }
    }
}

/// What the Delete sheet says: its title, and what goes.
pub fn delete_words(bears: &[String], tables: &[String]) -> (String, String) {
    const BEAR: &str = "A bear's folder moves to the runner's trash, its routines are removed and its threads are archived.";
    const TABLE: &str = "A table stops being a project. Its chat threads and its folder stay where they are.";
    let quote = |s: &str| format!("\u{201C}{s}\u{201D}");
    match (bears, tables) {
        ([one], []) => (format!("Delete {}?", quote(one)), "Its folder moves to the runner's trash, its routines are removed and its threads are archived.".to_owned()),
        ([], [one]) => (format!("Delete {}?", quote(one)), "It stops being a project. Its chat threads and its folder stay where they are.".to_owned()),
        _ => {
            let mut lines = vec![];
            if !bears.is_empty() {
                lines.push(format!("Bears: {}", bears.join(", ")));
            }
            if !tables.is_empty() {
                lines.push(format!("Tables: {}", tables.join(", ")));
            }
            if !bears.is_empty() {
                lines.push(BEAR.to_owned());
            }
            if !tables.is_empty() {
                lines.push(TABLE.to_owned());
            }
            (format!("Delete {} items?", bears.len() + tables.len()), lines.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keys(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn decodes_sections_and_skips_bad_rows() {
        let v = json!({ "on": true, "bears": [
            { "id": "head", "name": "Head Bear", "head": true, "folder": "/b/head", "state": {} },
            { "id": "w", "name": "Writer", "does": "Writes", "state": { "pinned": 0 }, "routines": [{ "paused": false }, { "paused": true }] },
            { "id": "r", "name": "Researcher", "away": true, "state": { "awayUntil": "2026-11-20T09:00:00Z" } },
            { "id": "old", "name": "Old", "archived": true, "state": { "archivedAt": "x" } },
            { "name": "No id" }
        ]});
        let bears = decode(&v).unwrap();
        assert_eq!(bears.len(), 4);
        assert_eq!(bears.iter().map(PanelBear::section).collect::<Vec<_>>(), vec![PINNED, PINNED, BEARS, ARCHIVED]);
        assert_eq!(bears[0].title(), "Head Bear (head bear)");
        assert_eq!(bears[1].routines, 1);
        assert_eq!(bears[2].away_until.as_deref(), Some("2026-11-20T09:00:00Z"));
        assert_eq!(decode(&json!({ "on": false, "bears": [] })), None);
    }

    #[test]
    fn plain_click_opens_and_clears() {
        let order = keys(&["a", "b", "c"]);
        let mut s = Selection { keys: keys(&["a", "b"]), anchor: None };
        assert!(s.click("c", Click::Plain, &order));
        assert!(s.is_empty());
        assert_eq!(s.anchor.as_deref(), Some("c"));
    }

    #[test]
    fn command_click_toggles() {
        let order = keys(&["a", "b", "c"]);
        let mut s = Selection::default();
        assert!(!s.click("a", Click::Toggle, &order));
        s.click("c", Click::Toggle, &order);
        assert_eq!(s.keys, keys(&["a", "c"]));
        s.click("a", Click::Toggle, &order);
        assert_eq!(s.keys, keys(&["c"]));
    }

    #[test]
    fn shift_click_takes_the_range_either_way() {
        let order = keys(&["a", "b", "c", "d"]);
        let mut s = Selection::default();
        s.click("b", Click::Plain, &order);
        s.click("d", Click::Range, &order);
        assert_eq!(s.keys, keys(&["b", "c", "d"]));
        s.click("a", Click::Range, &order);
        assert_eq!(s.keys, keys(&["a", "b"]));
        // With nothing clicked before, just the one.
        let mut fresh = Selection::default();
        fresh.click("c", Click::Range, &order);
        assert_eq!(fresh.keys, keys(&["c"]));
    }

    #[test]
    fn retain_drops_gone_rows() {
        let mut s = Selection { keys: keys(&["a", "b"]), anchor: Some("b".into()) };
        s.retain(&keys(&["a"]));
        assert_eq!(s.keys, keys(&["a"]));
        assert_eq!(s.anchor, None);
    }

    #[test]
    fn delete_words_name_what_goes() {
        let (title, _) = delete_words(&keys(&["Writer"]), &[]);
        assert_eq!(title, "Delete \u{201C}Writer\u{201D}?");
        let (title, text) = delete_words(&keys(&["Writer", "Researcher"]), &keys(&["Launch crew"]));
        assert_eq!(title, "Delete 3 items?");
        assert!(text.starts_with("Bears: Writer, Researcher\nTables: Launch crew"));
        assert!(!text.contains('\u{2014}'));
    }
}
