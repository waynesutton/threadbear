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
    /// The projects its new threads join (`state.projects`).
    pub projects: Vec<String>,
    /// A picture of it: a `photo*` file in its folder (`find_photo`),
    /// looked for on this Mac after the list comes.
    pub photo: Option<std::path::PathBuf>,
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
                    projects: b["state"]["projects"].as_array().into_iter().flatten().filter_map(|p| p.as_str().map(str::to_owned)).collect(),
                    photo: None,
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

/// The kinds of picture a bear's photo may be.
pub const PHOTO_KINDS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];

/// The biggest picture taken as a photo, so a stray RAW or video isn't
/// copied into a bear's folder.
pub const PHOTO_LIMIT: u64 = 20 * 1024 * 1024;

/// A bear's photo lives in its own folder as `photo-<ms>.<kind>`: the bear's
/// folder travels with it (exports, another Mac), and a new name each time
/// means a replaced photo is never drawn from the old one's cache. The
/// newest wins; `photo.png` dropped in by hand counts too.
pub fn is_photo_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let Some((stem, ext)) = lower.rsplit_once('.') else { return false };
    PHOTO_KINDS.contains(&ext) && (stem == "photo" || stem.strip_prefix("photo-").is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())))
}

/// The photo's file name for a picture `source` at `ms`, or none for a kind
/// that isn't a picture.
pub fn photo_name(source: &std::path::Path, ms: i64) -> Option<String> {
    let ext = source.extension()?.to_str()?.to_ascii_lowercase();
    PHOTO_KINDS.contains(&ext.as_str()).then(|| format!("photo-{ms}.{ext}"))
}

/// The bear's photo in `folder`, the newest by name.
pub fn find_photo(folder: &str) -> Option<std::path::PathBuf> {
    if folder.is_empty() {
        return None;
    }
    let mut names: Vec<String> = std::fs::read_dir(folder).ok()?.flatten().filter_map(|e| e.file_name().to_str().map(str::to_owned)).filter(|n| is_photo_name(n)).collect();
    names.sort_by_key(|n| photo_rank(n));
    names.pop().map(|n| std::path::Path::new(folder).join(n))
}

/// `photo-<ms>` sorts by its time; a bare `photo` is oldest.
fn photo_rank(name: &str) -> u64 {
    name.split_once('-').and_then(|(_, rest)| rest.split('.').next()?.parse().ok()).unwrap_or(0)
}

/// The first message of a thread that changes a bear or a table for Wayne,
/// through the bears skill (never by hand-editing a bear's memory).
pub fn change_prompt(key: &str, name: &str, folder: &str, ask: &str) -> String {
    let ask = ask.trim();
    let what = if ask.is_empty() { "Ask me what to change. Start by showing what it's set to now.".to_owned() } else { format!("What to change: {ask}") };
    if let Some(id) = key.strip_prefix("table:") {
        format!(
            "Use the bears skill to change the table \u{201C}{name}\u{201D} (project id {id}, folder {folder}). Seat or unseat bears with `tb tables seat`, and change its title, goals or parent project with the project tools.\n\n{what}\n\nSay what you changed when you're done."
        )
    } else {
        let id = key.strip_prefix("bear:").unwrap_or(key);
        format!(
            "Use the bears skill to change the bear \u{201C}{name}\u{201D} (id {id}, folder {folder}). Change its settings with `tb bears`; edit its persona (CLAUDE.md and AGENTS.md) only where it asks, and never its memory.\n\n{what}\n\nSay what you changed when you're done."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn photo_names() {
        assert!(is_photo_name("photo.png"));
        assert!(is_photo_name("photo-1763548800000.JPG"));
        assert!(!is_photo_name("photo-x.png"));
        assert!(!is_photo_name("photo-.png"));
        assert!(!is_photo_name("photo.txt"));
        assert!(!is_photo_name("myphoto.png"));
        assert_eq!(photo_name(std::path::Path::new("/a/Me.JPEG"), 5).as_deref(), Some("photo-5.jpeg"));
        assert_eq!(photo_name(std::path::Path::new("/a/notes.md"), 5), None);
    }

    #[test]
    fn newest_photo_wins() {
        let dir = std::env::temp_dir().join(format!("tb-photo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for n in ["photo.png", "photo-20.webp", "photo-100.png", "notes.md"] {
            std::fs::write(dir.join(n), b"x").unwrap();
        }
        let found = find_photo(&dir.to_string_lossy()).unwrap();
        assert_eq!(found.file_name().unwrap(), "photo-100.png");
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(find_photo(""), None);
    }

    #[test]
    fn change_prompts_name_the_row() {
        let bear = change_prompt("bear:writer-1a2b", "Writer", "/b/writer", "  shorter replies ");
        assert!(bear.contains("bear \u{201C}Writer\u{201D} (id writer-1a2b, folder /b/writer)"));
        assert!(bear.contains("What to change: shorter replies"));
        let table = change_prompt("table:p_1", "Launch crew", "/t/launch", "");
        assert!(table.contains("project id p_1"));
        assert!(table.contains("Ask me what to change"));
        assert!(!bear.contains('\u{2014}') && !table.contains('\u{2014}'));
    }

    fn keys(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn decodes_sections_and_skips_bad_rows() {
        let v = json!({ "on": true, "bears": [
            { "id": "head", "name": "Head Bear", "head": true, "folder": "/b/head", "state": {} },
            { "id": "w", "name": "Writer", "does": "Writes", "state": { "pinned": 0, "projects": ["p_1"] }, "routines": [{ "paused": false }, { "paused": true }] },
            { "id": "r", "name": "Researcher", "away": true, "state": { "awayUntil": "2026-11-20T09:00:00Z" } },
            { "id": "old", "name": "Old", "archived": true, "state": { "archivedAt": "x" } },
            { "name": "No id" }
        ]});
        let bears = decode(&v).unwrap();
        assert_eq!(bears.len(), 4);
        assert_eq!(bears.iter().map(PanelBear::section).collect::<Vec<_>>(), vec![PINNED, PINNED, BEARS, ARCHIVED]);
        assert_eq!(bears[0].title(), "Head Bear (head bear)");
        assert_eq!(bears[1].routines, 1);
        assert_eq!(bears[1].projects, vec!["p_1".to_owned()]);
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
