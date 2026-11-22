// Bears (docs/protocol.md, "Bears" and "Tables"): what the app shows of
// them with no window. A thread in a bear's folder carries the bear on its
// summary, and the sidebar draws its icon on its colour in place of the
// folder (Swift `BearTile`); the composer's @ list offers the bears
// `bears.list` gives (Swift `BearPicker`); a table's project page lists
// who sits at it (Swift `TableBearsSection`).
use serde::Deserialize;
use serde_json::Value;
use std::ops::Range;

/// The bear whose folder a thread is in (`ThreadSummary.bear`), as the
/// runner sends it while the bears experiment is on.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct ThreadBear {
    #[serde(deserialize_with = "crate::wire::id")]
    pub id: String,
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub name: String,
    /// An SF Symbol, from the bear's bear.json `icon`.
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub icon: Option<String>,
    /// "#rrggbb" or "#rgb", from the bear's bear.json `color`.
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub color: Option<String>,
    /// The head bear, which runs the others.
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub head: Option<bool>,
}

/// The bear's own fur, for a tile with no colour of its own (Swift
/// `BearTile.fur`: 0.84, 0.58, 0.33).
pub const FUR: (u8, u8, u8) = (214, 148, 84);

impl ThreadBear {
    pub fn is_head(&self) -> bool {
        self.head == Some(true)
    }

    /// The tile's colour: its own, or the fur for one that isn't "#rgb" or
    /// "#rrggbb".
    pub fn rgb(&self) -> (u8, u8, u8) {
        self.color.as_deref().and_then(rgb).unwrap_or(FUR)
    }

    /// The symbol to draw when the bear's own icon can't be (it has none,
    /// or the app has no drawing of it): a crown for the head bear, a paw
    /// for any other.
    pub fn fallback_symbol(&self) -> &'static str {
        if self.is_head() {
            "crown.fill"
        } else {
            "pawprint.fill"
        }
    }

    /// The tile's tooltip: "Writer", or "Head Bear, the head bear".
    pub fn tip(&self) -> String {
        if self.is_head() {
            format!("{}, the head bear", self.name)
        } else {
            self.name.clone()
        }
    }
}

/// Red, green and blue from "#rgb" or "#rrggbb", or none for anything else.
pub fn rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let digits = hex.strip_prefix('#')?;
    let full: String = match digits.len() {
        3 => digits.chars().flat_map(|c| [c, c]).collect(),
        6 => digits.to_owned(),
        _ => return None,
    };
    let value = u32::from_str_radix(&full, 16).ok()?;
    Some(((value >> 16) as u8, (value >> 8) as u8, value as u8))
}

/// Bear Colour's choices (Swift `BearColorMenu.colors`): the bear's own
/// fur (no colour set), then a few that sit well on both looks.
pub const COLOURS: [(&str, Option<&str>); 8] = [
    ("Fur", None),
    ("Honey", Some("#E0A33A")),
    ("Berry", Some("#C2414B")),
    ("Rose", Some("#D9668C")),
    ("Plum", Some("#8A5CC2")),
    ("Sky", Some("#3E8ED0")),
    ("Moss", Some("#5E9B4F")),
    ("Slate", Some("#6B7280")),
];

/// Whether a Bear Colour choice is the bear's colour now (its tick).
pub fn is_current(bear: &ThreadBear, hex: Option<&str>) -> bool {
    bear.color.as_deref().map(str::to_lowercase) == hex.map(str::to_lowercase)
}

/// A Bear Colour line's tooltip.
pub fn colour_tip(bear: &ThreadBear, name: &str, hex: Option<&str>) -> String {
    match hex {
        None => format!("{}'s own fur colour", bear.name),
        Some(_) => format!("Colour {}'s tile {}", bear.name, name.to_lowercase()),
    }
}

/// A bear as `bears.list` gives it, for the @ list and a table's page.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BearEntry {
    pub id: String,
    pub name: String,
    pub does: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub head: bool,
}

impl BearEntry {
    /// The same bear as a thread's tile draws it.
    pub fn tile(&self) -> ThreadBear {
        ThreadBear {
            id: self.id.clone(),
            name: self.name.clone(),
            icon: self.icon.clone(),
            color: self.color.clone(),
            head: self.head.then_some(true),
        }
    }
}

/// `bears.list`'s answer: none while the experiment is off, and none of the
/// archived ones. One that won't read is left out, not the rest.
pub fn decode_list(v: &Value) -> Vec<BearEntry> {
    if v["on"].as_bool() != Some(true) {
        return Vec::new();
    }
    let text = |b: &Value, key: &str| b[key].as_str().filter(|s| !s.is_empty()).map(str::to_owned);
    v["bears"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|b| b["archived"].as_bool() != Some(true))
        .filter_map(|b| {
            Some(BearEntry {
                id: text(b, "id")?,
                name: text(b, "name")?,
                does: text(b, "does"),
                icon: text(b, "icon"),
                color: text(b, "color"),
                head: b["head"].as_bool() == Some(true),
            })
        })
        .collect()
}

/// The @ list's bears for what's typed after the @ (Swift
/// `BearPicker.matches`): names starting with it first (the head bear
/// first among them), then names with a later word starting with it; eight
/// at most.
pub fn matches(query: &str, bears: &[BearEntry]) -> Vec<BearEntry> {
    let q = query.to_lowercase();
    let mut sorted: Vec<&BearEntry> = bears.iter().collect();
    sorted.sort_by(|a, b| (!a.head, &a.name).cmp(&(!b.head, &b.name)));
    let starts = sorted
        .iter()
        .filter(|b| b.name.to_lowercase().starts_with(&q));
    let words = sorted.iter().filter(|b| {
        let name = b.name.to_lowercase();
        !name.starts_with(&q) && name.split(' ').any(|w| w.starts_with(&q))
    });
    starts.chain(words).take(8).map(|b| (*b).clone()).collect()
}

/// The @word the cursor is at the end of, as a byte range from the @ (Swift
/// `bearToken`): names can have spaces ("@Head Bear"), so up to 40 letters,
/// digits, - ' and spaces back to an @ at the start of a word (the text's
/// start, or after a space, a new line, a tab or a "(").
pub fn token(text: &str, cursor: usize) -> Option<Range<usize>> {
    if cursor > text.len() || !text.is_char_boundary(cursor) {
        return None;
    }
    let before = &text[..cursor];
    let mut start = cursor;
    for (i, c) in before.char_indices().rev() {
        if cursor - i > 40 || !(c.is_ascii_alphanumeric() || c == '-' || c == ' ' || c == '\'') {
            break;
        }
        start = i;
    }
    let at = before[..start]
        .char_indices()
        .next_back()
        .filter(|(_, c)| *c == '@')
        .map(|(i, _)| i)?;
    if let Some(prev) = text[..at].chars().next_back() {
        if !matches!(prev, ' ' | '\n' | '\t' | '(') {
            return None;
        }
    }
    Some(at..cursor)
}

/// What the list offers for this @word: nothing to offer, or the words
/// already make one bear's whole name, and it stays shut.
pub fn offer(text: &str, range: &Range<usize>, bears: &[BearEntry]) -> Vec<BearEntry> {
    let query = &text[range.start + 1..range.end];
    let found = matches(query, bears);
    if found.len() == 1 && found[0].name.to_lowercase() == query.to_lowercase() {
        return Vec::new();
    }
    found
}

/// What picking a bear writes in place of the @word: "@Name ".
pub fn mention(bear: &BearEntry) -> String {
    format!("@{} ", bear.name)
}

/// When the list may show (Swift `BearPicker.Gate`): the message goes to
/// this Mac's runner (its bears are this runner's, and another runner hands
/// a mention to its own), the box has the cursor, and this @word wasn't put
/// away with Esc.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Gate {
    /// Where the @ of the word Esc put away is, until the cursor leaves it.
    pub dismissed_at: Option<usize>,
}

impl Gate {
    pub fn may_offer(&self, token_at: usize, focused: bool, bears_here: bool) -> bool {
        bears_here && focused && self.dismissed_at != Some(token_at)
    }

    /// The cursor is on another @word, or none: Esc's no longer holds.
    pub fn moved(&mut self, token_at: Option<usize>) {
        if token_at != self.dismissed_at {
            self.dismissed_at = None;
        }
    }
}

/// Which line of the list is picked after an arrow, going round the ends.
pub fn step(selected: usize, by: i32, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    (selected as i64 + by as i64).rem_euclid(count as i64) as usize
}

/// A project that's a table of bears (protocol "Tables"): the ids of the
/// bears at it, and how many bear replies each message allows (none: the
/// runner's default, 6).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    pub bears: Vec<String>,
    pub max_turns: Option<u32>,
}

impl Table {
    /// A project's `table`, or none for any other project or one that
    /// won't read.
    pub fn decode(v: &Value) -> Option<Table> {
        let o = v.as_object()?;
        let bears = o
            .get("bears")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|b| b.as_str().map(str::to_owned))
            .collect();
        let max_turns = o
            .get("maxTurns")
            .and_then(|n| n.as_u64().or_else(|| n.as_f64().map(|f| f as u64)))
            .map(|n| n as u32);
        Some(Table { bears, max_turns })
    }
}

/// One line of "Bears at this table": the bear as listed, or its id where
/// this Mac's runner doesn't list it now.
#[derive(Clone, Debug, PartialEq)]
pub struct Seat {
    pub tile: ThreadBear,
    pub name: String,
    pub does: Option<String>,
    pub tip: String,
}

/// The table's seats in its order (Swift `TableBearsSection`).
pub fn seats(table: &Table, bears: &[BearEntry]) -> Vec<Seat> {
    table
        .bears
        .iter()
        .map(|id| match bears.iter().find(|b| &b.id == id) {
            Some(b) => Seat {
                tile: b.tile(),
                name: b.name.clone(),
                does: b.does.clone(),
                tip: format!("{} sits at this table", b.name),
            },
            None => Seat {
                tile: ThreadBear {
                    id: id.clone(),
                    name: id.clone(),
                    ..Default::default()
                },
                name: id.clone(),
                does: None,
                tip: "A bear this Mac doesn't list now".to_owned(),
            },
        })
        .collect()
}

/// What a table with nobody at it says.
pub const NO_SEATS: &str = "No bears yet. Put a bear's thread in this project to seat it.";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn bear(id: &str, name: &str, head: bool) -> BearEntry {
        BearEntry {
            id: id.into(),
            name: name.into(),
            head,
            ..Default::default()
        }
    }

    #[test]
    fn colours_read_short_and_long_hex() {
        assert_eq!(rgb("#3E8ED0"), Some((0x3E, 0x8E, 0xD0)));
        assert_eq!(rgb("#fff"), Some((255, 255, 255)));
        assert_eq!(rgb("3E8ED0"), None);
        assert_eq!(rgb("#12345"), None);
        assert_eq!(rgb("#zzzzzz"), None);
        let b = ThreadBear {
            id: "w".into(),
            name: "Writer".into(),
            color: Some("blue".into()),
            ..Default::default()
        };
        assert_eq!(b.rgb(), FUR);
    }

    #[test]
    fn the_head_bear_gets_a_crown_and_says_so() {
        let head = ThreadBear {
            id: "h".into(),
            name: "Head Bear".into(),
            head: Some(true),
            ..Default::default()
        };
        assert_eq!(head.fallback_symbol(), "crown.fill");
        assert_eq!(head.tip(), "Head Bear, the head bear");
        let writer = ThreadBear {
            id: "w".into(),
            name: "Writer".into(),
            ..Default::default()
        };
        assert_eq!(writer.fallback_symbol(), "pawprint.fill");
        assert_eq!(writer.tip(), "Writer");
    }

    #[test]
    fn a_summary_reads_its_bear_and_a_bad_one_reads_as_none() {
        let t: crate::inbox::ThreadSummary = serde_json::from_value(json!({ "id": "t1", "bear": { "id": "w", "name": "Writer", "icon": "pencil", "color": "#3E8ED0" } })).unwrap();
        assert_eq!(t.bear.as_ref().map(|b| b.name.as_str()), Some("Writer"));
        let t: crate::inbox::ThreadSummary =
            serde_json::from_value(json!({ "id": "t1", "bear": "nonsense" })).unwrap();
        assert_eq!(t.bear, None);
    }

    #[test]
    fn the_current_colour_is_ticked_whatever_its_case() {
        let mut b = ThreadBear {
            id: "w".into(),
            name: "Writer".into(),
            color: Some("#3e8ed0".into()),
            ..Default::default()
        };
        assert!(is_current(&b, Some("#3E8ED0")));
        assert!(!is_current(&b, None));
        b.color = None;
        assert!(is_current(&b, None));
        assert_eq!(
            colour_tip(&b, "Moss", Some("#5E9B4F")),
            "Colour Writer's tile moss"
        );
        assert_eq!(colour_tip(&b, "Fur", None), "Writer's own fur colour");
    }

    #[test]
    fn the_list_leaves_out_archived_bears_and_is_empty_while_off() {
        let v = json!({ "on": true, "bears": [{ "id": "w", "name": "Writer", "does": "Drafts" }, { "id": "o", "name": "Old", "archived": true }, { "name": "No id" }] });
        let list = decode_list(&v);
        assert_eq!(
            list.iter().map(|b| b.name.as_str()).collect::<Vec<_>>(),
            ["Writer"]
        );
        assert_eq!(list[0].does.as_deref(), Some("Drafts"));
        assert!(
            decode_list(&json!({ "on": false, "bears": [{ "id": "w", "name": "Writer" }] }))
                .is_empty()
        );
    }

    #[test]
    fn names_starting_with_it_come_first_the_head_bear_leading() {
        let bears = [
            bear("w", "Writer", false),
            bear("h", "Head Bear", true),
            bear("b", "Bear Wrangler", false),
            bear("hw", "Hardware", false),
        ];
        let names = |q: &str| {
            matches(q, &bears)
                .into_iter()
                .map(|b| b.name)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names(""),
            ["Head Bear", "Bear Wrangler", "Hardware", "Writer"]
        );
        assert_eq!(names("h"), ["Head Bear", "Hardware"]);
        assert_eq!(names("wr"), ["Writer", "Bear Wrangler"]);
        assert_eq!(names("bear"), ["Bear Wrangler", "Head Bear"]);
        assert!(names("zz").is_empty());
    }

    #[test]
    fn the_token_is_an_at_word_ending_at_the_cursor() {
        assert_eq!(token("Hey @wr", 7), Some(4..7));
        assert_eq!(token("@", 1), Some(0..1));
        assert_eq!(token("ask @Head Bear", 14), Some(4..14));
        assert_eq!(token("(@wr", 4), Some(1..4));
        // An email address isn't one.
        assert_eq!(token("mail me@home", 12), None);
        // Nor once the cursor's past a stop.
        assert_eq!(token("@wr, then", 9), None);
        assert_eq!(token("Hey @wr", 3), None);
        // Past 40 back, it's not a name any more.
        let long = format!("@{}", "a".repeat(45));
        assert_eq!(token(&long, long.len()), None);
        // Never splits a letter.
        assert_eq!(token("é@", 1), None);
    }

    #[test]
    fn a_whole_name_typed_closes_the_list() {
        let bears = [bear("w", "Writer", false)];
        assert_eq!(offer("Hey @writer", &(4..11), &bears), vec![]);
        assert_eq!(offer("Hey @wr", &(4..7), &bears).len(), 1);
        assert_eq!(mention(&bears[0]), "@Writer ");
    }

    #[test]
    fn esc_keeps_its_word_shut_until_the_cursor_leaves_it() {
        let mut gate = Gate::default();
        assert!(gate.may_offer(4, true, true));
        assert!(!gate.may_offer(4, false, true));
        assert!(!gate.may_offer(4, true, false));
        gate.dismissed_at = Some(4);
        gate.moved(Some(4));
        assert!(!gate.may_offer(4, true, true));
        gate.moved(None);
        assert!(gate.may_offer(4, true, true));
    }

    #[test]
    fn the_arrows_go_round() {
        assert_eq!(step(0, -1, 3), 2);
        assert_eq!(step(2, 1, 3), 0);
        assert_eq!(step(0, 1, 0), 0);
    }

    #[test]
    fn a_tables_seats_follow_its_order_and_name_unknown_bears() {
        let table = Table::decode(&json!({ "bears": ["h", "gone"], "maxTurns": 4 })).unwrap();
        assert_eq!(table.max_turns, Some(4));
        assert!(Table::decode(&json!("nonsense")).is_none());
        let seats = seats(
            &table,
            &[bear("w", "Writer", false), bear("h", "Head Bear", true)],
        );
        assert_eq!(
            seats.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            ["Head Bear", "gone"]
        );
        assert_eq!(seats[0].tip, "Head Bear sits at this table");
        assert_eq!(seats[1].tip, "A bear this Mac doesn't list now");
    }
}
