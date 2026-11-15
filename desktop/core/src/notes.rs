//! Notes (the bar's Notes page, ⌘6): your own notes, kept on this Mac only, a
//! port of the Swift app's `NotesStore`, `NoteCrypto`, `NoteNames`,
//! `NoteHighlight` and `NoteEditing`. Both apps read and write the same files,
//! so the notes are the same in either:
//!
//! - `<data dir>/notes/<id>.tbnote`: a version byte (1), then AES-256-GCM's
//!   12-byte nonce, the sealed JSON and the 16-byte tag, as CryptoKit's
//!   `SealedBox.combined` lays them out. The JSON is the Swift `Note`, with
//!   its dates as seconds since 1 Jan 2001 (JSONEncoder's default).
//! - `<data dir>/secrets/notes-key`: the 32-byte key, this user's only.
//!
//! Nothing here goes to the runner, an agent or the backend. The Swift app
//! moved the key out of the Keychain on 7 Nov; with notes on disk and no key
//! file, that move hasn't happened on this Mac, and only the Swift app can
//! make it, so this says the notes can't be unlocked and replaces nothing.
//!
//! Text positions here are byte offsets into the note's UTF-8 text, as the
//! GPUI editor counts them (the Swift app's are UTF-16, and never cross over).

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom as _, SystemRandom};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use std::path::{Path, PathBuf};

/// One note, as both apps keep it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub title: String,
    pub body: String,
    /// Seconds since 1 Jan 2001, as Foundation's `Date` encodes.
    pub created_at: f64,
    pub updated_at: f64,
}

/// Seconds between 1 Jan 1970 and 1 Jan 2001.
const REFERENCE_DATE: f64 = 978_307_200.;

/// Now, as a note's dates count.
pub fn now() -> f64 {
    let unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.);
    unix - REFERENCE_DATE
}

/// A note's date as the page shows it: "Nov 15, 2026 at 9:41 PM".
pub fn date_label(at: f64, with_time: bool) -> String {
    use chrono::TimeZone as _;
    let unix = at + REFERENCE_DATE;
    let Some(local) = chrono::Local.timestamp_opt(unix.floor() as i64, 0).single() else { return String::new() };
    if with_time {
        local.format("%b %-d, %Y at %-I:%M %p").to_string()
    } else {
        local.format("%b %-d, %Y").to_string()
    }
}

impl Note {
    /// A new, empty note called `title`.
    pub fn new(title: String) -> Self {
        let at = now();
        Note { id: new_id(), title, body: String::new(), created_at: at, updated_at: at }
    }

    /// The list's second line: the first line with words in it, or the date.
    pub fn preview(&self) -> String {
        self.body.lines().find(|l| !l.trim().is_empty()).map(str::to_owned).unwrap_or_else(|| date_label(self.updated_at, false))
    }
}

/// An id as Foundation's `UUID().uuidString` writes one: version 4, upper case.
pub fn new_id() -> String {
    let mut b = [0u8; 16];
    // SystemRandom only fails where there's no OS random source at all.
    let _ = SystemRandom::new().fill(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02X}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}

// Sealing.

const VERSION: u8 = 1;
const TAG_LEN: usize = 16;

#[derive(Debug, PartialEq, Eq)]
pub struct BadData;

fn aead_key(key: &[u8; 32]) -> LessSafeKey {
    // AES-256 takes any 32 bytes, so this can't fail.
    LessSafeKey::new(UnboundKey::new(&AES_256_GCM, key).expect("a 32-byte key"))
}

/// The note as stored (`NoteCrypto.seal`).
pub fn seal(note: &Note, key: &[u8; 32]) -> Result<Vec<u8>, BadData> {
    let mut nonce = [0u8; NONCE_LEN];
    SystemRandom::new().fill(&mut nonce).map_err(|_| BadData)?;
    let mut sealed = serde_json::to_vec(note).map_err(|_| BadData)?;
    aead_key(key).seal_in_place_append_tag(Nonce::assume_unique_for_key(nonce), Aad::empty(), &mut sealed).map_err(|_| BadData)?;
    let mut out = Vec::with_capacity(1 + NONCE_LEN + sealed.len());
    out.push(VERSION);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&sealed);
    Ok(out)
}

/// A stored note, opened (`NoteCrypto.open`): an error for another key, a
/// file that's been changed, or anything that isn't one.
pub fn open(data: &[u8], key: &[u8; 32]) -> Result<Note, BadData> {
    if data.first() != Some(&VERSION) || data.len() < 1 + NONCE_LEN + TAG_LEN {
        return Err(BadData);
    }
    let nonce: [u8; NONCE_LEN] = data[1..1 + NONCE_LEN].try_into().map_err(|_| BadData)?;
    let mut sealed = data[1 + NONCE_LEN..].to_vec();
    let plain = aead_key(key).open_in_place(Nonce::assume_unique_for_key(nonce), Aad::empty(), &mut sealed).map_err(|_| BadData)?;
    serde_json::from_slice(plain).map_err(|_| BadData)
}

// Names.

pub const BASE_NAME: &str = "Untitled note";

/// "Untitled note", or the first "Untitled note N" not taken (`NoteNames.next`).
pub fn next_name<'a>(titles: impl IntoIterator<Item = &'a str>) -> String {
    let taken: std::collections::HashSet<String> = titles.into_iter().map(|t| t.trim().to_lowercase()).collect();
    if !taken.contains(&BASE_NAME.to_lowercase()) {
        return BASE_NAME.into();
    }
    (2..).map(|n| format!("{BASE_NAME} {n}")).find(|name| !taken.contains(&name.to_lowercase())).unwrap_or_default()
}

/// A title as kept: one line, trimmed, at most 200 characters, never empty
/// (`NoteNames.clean`).
pub fn clean_name(title: &str, fallback: &str) -> String {
    let one = title.replace('\n', " ");
    let one = one.trim_matches(|c: char| c == ' ' || c == '\t');
    if one.is_empty() {
        fallback.into()
    } else {
        one.chars().take(200).collect()
    }
}

// Colouring (`NoteHighlight`).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Heading,
    Bold,
    Italic,
    Code,
    CodeBlock,
    ListMarker,
    Link,
    Quote,
    /// A mark that stays in the text, dimmed: a heading's #s, a quote's >.
    Mark,
    /// A task's box, `[ ]` or `[x]`, that a click ticks.
    Checkbox,
    /// The words of a ticked task.
    Done,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub kind: Kind,
    /// A heading's level, 1 for #.
    pub level: usize,
}

fn span(range: Range<usize>, kind: Kind) -> Span {
    Span { range, kind, level: 0 }
}

/// What the editor colours, line by line, in the order the Swift rules give
/// them (a later span's style goes over an earlier one's).
pub fn spans(text: &str) -> Vec<Span> {
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut at = 0;
    for line in text.split('\n') {
        let range = at..at + line.len();
        let trimmed = line.trim_matches(|c: char| c == ' ' || c == '\t');
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            out.push(span(range.clone(), Kind::CodeBlock));
            in_fence = !in_fence;
        } else if in_fence {
            out.push(span(range.clone(), Kind::CodeBlock));
        } else if let Some(level) = heading_level(line) {
            out.push(Span { range: range.clone(), kind: Kind::Heading, level });
            out.push(span(at..at + level, Kind::Mark));
        } else if let Some(mark) = quote_mark(line) {
            out.push(span(range.clone(), Kind::Quote));
            out.push(span(at + mark.start..at + mark.end, Kind::Mark));
        } else {
            if let Some(item) = list_item(line) {
                out.push(span(at + item.bullet.start..at + item.bullet.end, Kind::ListMarker));
                if let Some((box_at, ticked)) = item.task {
                    out.push(span(at + box_at..at + box_at + 3, Kind::Checkbox));
                    if ticked && item.end < line.len() {
                        out.push(span(at + item.end..range.end, Kind::Done));
                    }
                }
            }
            for (kind, found) in [(Kind::Bold, bold(line)), (Kind::Italic, italic(line)), (Kind::Code, code(line)), (Kind::Link, links(line))] {
                out.extend(found.into_iter().map(|r| span(at + r.start..at + r.end, kind)));
            }
        }
        at = range.end + 1;
    }
    out
}

/// `^(#{1,6})\s`: the number of #s.
fn heading_level(line: &str) -> Option<usize> {
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    let next = line[hashes..].chars().next();
    ((1..=6).contains(&hashes) && next.is_some_and(char::is_whitespace)).then_some(hashes)
}

/// `^\s*>\s?`: where the mark is.
fn quote_mark(line: &str) -> Option<Range<usize>> {
    let start = line.len() - line.trim_start().len();
    if !line[start..].starts_with('>') {
        return None;
    }
    let mut end = start + 1;
    if let Some(c) = line[end..].chars().next().filter(|c| c.is_whitespace()) {
        end += c.len_utf8();
    }
    Some(start..end)
}

struct ListItem {
    bullet: Range<usize>,
    /// Where the box's `[` is, and whether it's ticked.
    task: Option<(usize, bool)>,
    /// The end of the bullet, the space and the box (with one space after).
    end: usize,
}

/// `^\s*([-*+]|\d{1,9}[.)])\s(\[([ xX])\](?:\s|$))?`
fn list_item(line: &str) -> Option<ListItem> {
    let start = line.len() - line.trim_start().len();
    let rest = &line[start..];
    let bullet_len = match rest.as_bytes().first()? {
        b'-' | b'*' | b'+' => 1,
        _ => {
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            if !(1..=9).contains(&digits) || !matches!(rest.as_bytes().get(digits), Some(b'.' | b')')) {
                return None;
            }
            digits + 1
        }
    };
    let bullet = start..start + bullet_len;
    let space = line[bullet.end..].chars().next().filter(|c| c.is_whitespace())?;
    let mut end = bullet.end + space.len_utf8();
    let mut task = None;
    let b = line.as_bytes();
    if b.get(end) == Some(&b'[') && matches!(b.get(end + 1), Some(b' ' | b'x' | b'X')) && b.get(end + 2) == Some(&b']') {
        let after = end + 3;
        let next = line[after..].chars().next();
        if next.is_none_or(char::is_whitespace) {
            task = Some((end, b[end + 1] != b' '));
            end = after + next.map_or(0, char::len_utf8);
        }
    }
    Some(ListItem { bullet, task, end })
}

/// A \w character, as NSRegularExpression counts them.
fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The chars of a line with where each starts, and where the line ends.
fn chars(line: &str) -> Vec<(usize, char)> {
    line.char_indices().collect()
}

/// Leftmost, non-overlapping matches of `at_each`, which tries a match
/// starting at char `i` and gives the char index just past it.
fn scan(line: &str, at_each: impl Fn(&[(usize, char)], usize) -> Option<usize>) -> Vec<Range<usize>> {
    let cs = chars(line);
    let byte = |i: usize| cs.get(i).map_or(line.len(), |(b, _)| *b);
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        match at_each(&cs, i) {
            Some(end) if end > i => {
                out.push(byte(i)..byte(end));
                i = end;
            }
            _ => i += 1,
        }
    }
    out
}

/// `\*\*[^*\n]+\*\*|__[^_\n]+__`
fn bold(line: &str) -> Vec<Range<usize>> {
    scan(line, |cs, i| {
        let mark = cs[i].1;
        if !(mark == '*' || mark == '_') || cs.get(i + 1).map(|c| c.1) != Some(mark) {
            return None;
        }
        let mut k = i + 2;
        while k < cs.len() && cs[k].1 != mark && cs[k].1 != '\n' {
            k += 1;
        }
        (k > i + 2 && cs.get(k).map(|c| c.1) == Some(mark) && cs.get(k + 1).map(|c| c.1) == Some(mark)).then_some(k + 2)
    })
}

/// `(?<![*\w])\*[^*\s][^*\n]*\*(?!\*)|(?<![_\w])_[^_\s][^_\n]*_(?!\w)`
fn italic(line: &str) -> Vec<Range<usize>> {
    scan(line, |cs, i| {
        let mark = cs[i].1;
        if mark != '*' && mark != '_' {
            return None;
        }
        let before = i.checked_sub(1).map(|p| cs[p].1);
        if before.is_some_and(|b| b == mark || word(b)) {
            return None;
        }
        let first = cs.get(i + 1)?.1;
        if first == mark || first.is_whitespace() {
            return None;
        }
        let mut k = i + 2;
        while k < cs.len() && cs[k].1 != mark && cs[k].1 != '\n' {
            k += 1;
        }
        if cs.get(k).map(|c| c.1) != Some(mark) {
            return None;
        }
        let after = cs.get(k + 1).map(|c| c.1);
        let ok = if mark == '*' { after != Some('*') } else { !after.is_some_and(word) };
        ok.then_some(k + 1)
    })
}

/// `` `[^`\n]+` ``
fn code(line: &str) -> Vec<Range<usize>> {
    scan(line, |cs, i| {
        if cs[i].1 != '`' {
            return None;
        }
        let mut k = i + 1;
        while k < cs.len() && cs[k].1 != '`' {
            k += 1;
        }
        (k > i + 1 && k < cs.len()).then_some(k + 1)
    })
}

/// `\[[^\]\n]+\]\([^)\s]+\)|https?://[^\s)]+`
fn links(line: &str) -> Vec<Range<usize>> {
    scan(line, |cs, i| {
        if cs[i].1 == '[' {
            let mut k = i + 1;
            while k < cs.len() && cs[k].1 != ']' {
                k += 1;
            }
            if k > i + 1 && cs.get(k + 1).map(|c| c.1) == Some('(') {
                let mut m = k + 2;
                while m < cs.len() && cs[m].1 != ')' && !cs[m].1.is_whitespace() {
                    m += 1;
                }
                if m > k + 2 && cs.get(m).map(|c| c.1) == Some(')') {
                    return Some(m + 1);
                }
            }
        }
        let rest: String = cs[i..].iter().take(8).map(|c| c.1).collect();
        let scheme = if rest.starts_with("https://") { 8 } else if rest.starts_with("http://") { 7 } else { return None };
        let mut k = i + scheme;
        while k < cs.len() && !cs[k].1.is_whitespace() && cs[k].1 != ')' {
            k += 1;
        }
        (k > i + scheme).then_some(k)
    })
}

// Typing help (`NoteEditing`).

/// Ticks or unticks the box whose `[` is at `at`: the range inside it and
/// what goes there, or None if there's no box there.
pub fn toggled_box(text: &str, at: usize) -> Option<(Range<usize>, &'static str)> {
    let b = text.as_bytes();
    if b.get(at) != Some(&b'[') || b.get(at + 2) != Some(&b']') {
        return None;
    }
    match b.get(at + 1)? {
        b' ' => Some((at + 1..at + 2, "x")),
        b'x' | b'X' => Some((at + 1..at + 2, " ")),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Newline {
    /// An ordinary new line.
    Plain,
    /// A new line that starts with this, the list's next item.
    ContinueList(String),
    /// Return on an empty item: the list ends, and its marker goes.
    EndList,
}

/// What Return does with the caret after `before` (the line up to the
/// caret) and before `after` (the rest of the line).
pub fn newline(before: &str, after: &str) -> Newline {
    let indent_len = before.len() - before.trim_start().len();
    let rest = &before[indent_len..];
    let (bullet, bullet_len) = match rest.as_bytes().first() {
        Some(b'-' | b'*' | b'+') => (rest[..1].to_owned(), 1),
        Some(_) => {
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            let Some(sep @ (b'.' | b')')) = rest.as_bytes().get(digits).copied().filter(|_| (1..=9).contains(&digits)) else { return Newline::Plain };
            let n: u64 = rest[..digits].parse().unwrap_or(0);
            (format!("{}{}", n + 1, sep as char), digits + 1)
        }
        None => return Newline::Plain,
    };
    let Some(space) = rest[bullet_len..].chars().next().filter(|c| c.is_whitespace()) else { return Newline::Plain };
    let mut content = &rest[bullet_len + space.len_utf8()..];
    let mut task = false;
    let cb = content.as_bytes();
    if cb.first() == Some(&b'[') && matches!(cb.get(1), Some(b' ' | b'x' | b'X')) && cb.get(2) == Some(&b']') {
        task = true;
        content = &content[3..];
        if let Some(c) = content.chars().next().filter(|c| c.is_whitespace()) {
            content = &content[c.len_utf8()..];
        }
    }
    if content.trim().is_empty() {
        return if after.trim().is_empty() { Newline::EndList } else { Newline::Plain };
    }
    let indent = &before[..indent_len];
    Newline::ContinueList(format!("{indent}{bullet} {}", if task { "[ ] " } else { "" }))
}

// The files (`NotesStore`).

/// The notes on this Mac, as loaded from `<data dir>/notes`.
pub struct Store {
    folder: PathBuf,
    key_file: PathBuf,
    key: Option<[u8; 32]>,
    pub notes: Vec<Note>,
    pub loaded: bool,
    /// Why the notes can't be opened, if they can't.
    pub problem: Option<String>,
}

pub const NO_KEY: &str = "Your notes couldn't be unlocked: Threadbear couldn't read their key.";

impl Store {
    /// The notes of the runner whose data folder is `home`.
    pub fn new(home: &Path) -> Self {
        Self::at(home.join("notes"), home.join("secrets").join("notes-key"))
    }

    pub fn at(folder: PathBuf, key_file: PathBuf) -> Self {
        Store { folder, key_file, key: None, notes: Vec::new(), loaded: false, problem: None }
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// Reads them all, once. A file that won't open is left where it is.
    pub fn load(&mut self) {
        if self.loaded {
            return;
        }
        let key = match self.key.map(Ok).unwrap_or_else(|| self.load_key()) {
            Ok(key) => key,
            Err(e) => {
                eprintln!("threadbear: notes: {e}");
                self.problem = Some(NO_KEY.into());
                return;
            }
        };
        self.key = Some(key);
        if let Err(e) = crate::private_file::dir(&self.folder) {
            eprintln!("threadbear: notes: couldn't make {}: {e}", self.folder.display());
        }
        let mut found = Vec::new();
        for path in self.note_files() {
            match std::fs::read(&path).map_err(|_| BadData).and_then(|data| open(&data, &key)) {
                Ok(note) => found.push(note),
                Err(_) => eprintln!("threadbear: notes: couldn't open {}", path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()),
            }
        }
        self.notes = found;
        self.loaded = true;
        self.problem = None;
    }

    fn note_files(&self) -> Vec<PathBuf> {
        std::fs::read_dir(&self.folder).map(|dir| dir.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "tbnote")).collect()).unwrap_or_default()
    }

    /// The key from its file. With none, a new one, but only while there are
    /// no notes it should have opened.
    fn load_key(&self) -> Result<[u8; 32], String> {
        match std::fs::read(&self.key_file) {
            Ok(data) if data.len() == 32 => return data.try_into().map_err(|_| "bad key".to_owned()),
            Ok(_) => return Err(format!("{} isn't a notes key", self.key_file.display())),
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(format!("couldn't read {}: {e}", self.key_file.display())),
            Err(_) => {}
        }
        if !self.note_files().is_empty() {
            return Err("notes with no key file: open Notes in the Swift app once to move its key".into());
        }
        let mut key = [0u8; 32];
        SystemRandom::new().fill(&mut key).map_err(|_| "no random source".to_owned())?;
        crate::private_file::write(&self.key_file, &key).map_err(|e| format!("couldn't save {}: {e}", self.key_file.display()))?;
        Ok(key)
    }

    /// Newest first.
    pub fn sorted(&self) -> Vec<&Note> {
        let mut list: Vec<&Note> = self.notes.iter().collect();
        list.sort_by(|a, b| b.updated_at.total_cmp(&a.updated_at));
        list
    }

    pub fn get(&self, id: &str) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    /// A new, empty note with the next free name, saved: its id.
    pub fn create(&mut self) -> Option<String> {
        self.load();
        self.key?;
        let note = Note::new(next_name(self.notes.iter().map(|n| n.title.as_str())));
        let id = note.id.clone();
        self.notes.push(note);
        self.save(&id);
        Some(id)
    }

    /// Renamed and saved, if the cleaned name is new.
    pub fn rename(&mut self, id: &str, title: &str) -> bool {
        let Some(note) = self.notes.iter_mut().find(|n| n.id == id) else { return false };
        let clean = clean_name(title, &note.title);
        if clean == note.title {
            return false;
        }
        note.title = clean;
        note.updated_at = now();
        self.save(id);
        true
    }

    /// The text as typed, kept here: whoever calls this saves it (a moment
    /// after the typing stops). False when nothing changed.
    pub fn set_body(&mut self, id: &str, body: &str) -> bool {
        let Some(note) = self.notes.iter_mut().find(|n| n.id == id) else { return false };
        if note.body == body {
            return false;
        }
        note.body = body.to_owned();
        note.updated_at = now();
        true
    }

    /// Writes the note whole, sealed, this user's only.
    pub fn save(&self, id: &str) {
        let (Some(key), Some(note)) = (self.key, self.get(id)) else { return };
        let result = seal(note, &key).map_err(|_| "couldn't seal it".to_owned()).and_then(|data| crate::private_file::write(&self.file(id), &data).map_err(|e| e.to_string()));
        if let Err(e) = result {
            eprintln!("threadbear: notes: couldn't save a note: {e}");
        }
    }

    pub fn delete(&mut self, id: &str) {
        self.notes.retain(|n| n.id != id);
        let _ = std::fs::remove_file(self.file(id));
    }

    fn file(&self, id: &str) -> PathBuf {
        // An id is a UUID: nothing in it can leave the folder.
        let safe: String = id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
        self.folder.join(format!("{safe}.tbnote"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 32] = {
        let mut k = [0u8; 32];
        let mut i = 0;
        while i < 32 {
            k[i] = i as u8;
            i += 1;
        }
        k
    };

    /// Sealed by CryptoKit with KEY, as the Swift app writes a note.
    const FROM_SWIFT: &str = "013c86053848bf758dc13c53871242fc2287d33a68c52f18943c40d316dcbdd7df8c7a16d9674648f8e20c31f2c0f08582156a060f0a79862eb1d047609b74cd89a719eaf1f6d4acd94b201010e4ec9cae7e16e74bac52c0b8f4391b8db0a1ff2d07e16aaa88538a55e1d149707874c37eb2e3e69b4cc3106e2d719ee269b31d7412e2d00095e192a381dcc2d9eca3365c6a5d2aa2f6f003fafaafd36e0b87ddfafdb449024caa95d1d4908e34fb5cc29c";

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    fn texts<'a>(text: &'a str, kind: Kind) -> Vec<&'a str> {
        spans(text).into_iter().filter(|s| s.kind == kind).map(|s| &text[s.range]).collect()
    }

    #[test]
    fn opens_what_the_swift_app_sealed() {
        let note = open(&unhex(FROM_SWIFT), &KEY).unwrap();
        assert_eq!(note.id, "6F1C2B9E-3A4D-4E5F-8A7B-1C2D3E4F5A6B");
        assert_eq!(note.title, "From Swift");
        assert_eq!(note.body, "- [ ] milk\n**bold** é");
        assert_eq!(note.created_at, 784_000_000.25);
        assert_eq!(note.updated_at, 784_000_100.5);
    }

    #[test]
    fn sealed_notes_open_only_with_their_key() {
        let note = Note { body: "- secret thing".into(), ..Note::new("Plans".into()) };
        let sealed = seal(&note, &KEY).unwrap();
        assert!(!sealed.windows(6).any(|w| w == b"secret"), "nothing readable on disk");
        assert_eq!(open(&sealed, &KEY).unwrap(), note);
        assert_eq!(open(&sealed, &[7u8; 32]), Err(BadData));
        let mut tampered = sealed.clone();
        *tampered.last_mut().unwrap() ^= 0xff;
        assert_eq!(open(&tampered, &KEY), Err(BadData));
        assert_eq!(open(&sealed[..10], &KEY), Err(BadData));
        // The JSON inside is the Swift Note's: its keys and plain-number dates.
        let mut inner = sealed[1 + NONCE_LEN..].to_vec();
        let plain = aead_key(&KEY).open_in_place(Nonce::assume_unique_for_key(sealed[1..13].try_into().unwrap()), Aad::empty(), &mut inner).unwrap();
        let json: serde_json::Value = serde_json::from_slice(plain).unwrap();
        assert!(json["createdAt"].is_f64() && json["updatedAt"].is_f64() && json["id"].is_string());
    }

    #[test]
    fn ids_look_like_foundations() {
        let id = new_id();
        assert_eq!(id.len(), 36);
        assert_eq!(&id[14..15], "4");
        assert_eq!(id, id.to_uppercase());
        assert_ne!(new_id(), id);
    }

    #[test]
    fn default_names() {
        assert_eq!(next_name([]), "Untitled note");
        assert_eq!(next_name(["Untitled note"]), "Untitled note 2");
        assert_eq!(next_name(["untitled note", "Untitled note 2", "Shopping"]), "Untitled note 3");
        assert_eq!(clean_name("  Groceries \n list ", "x"), "Groceries   list");
        assert_eq!(clean_name("   ", "Untitled note"), "Untitled note");
        assert_eq!(clean_name(&"é".repeat(300), "x").chars().count(), 200);
    }

    #[test]
    fn highlighting() {
        let text = "# Title\n- **milk** and `eggs`\n> quoted\n```\ncode *not italic*\n```\nsee https://example.com";
        let kinds: Vec<Kind> = spans(text).into_iter().map(|s| s.kind).collect();
        for kind in [Kind::Heading, Kind::ListMarker, Kind::Bold, Kind::Code, Kind::Quote, Kind::Link] {
            assert!(kinds.contains(&kind), "{kind:?}");
        }
        assert!(!kinds.contains(&Kind::Italic), "inside a code block it's code");
        assert_eq!(texts(text, Kind::Bold), ["**milk**"]);
        assert_eq!(texts(text, Kind::Code), ["`eggs`"]);
        assert_eq!(texts(text, Kind::Link), ["https://example.com"]);
        assert_eq!(texts(text, Kind::Mark), ["#", "> "]);
    }

    #[test]
    fn inline_marks_as_the_swift_rules_read_them() {
        assert_eq!(texts("an *em* word and _this_ one", Kind::Italic), ["*em*", "_this_"]);
        assert!(texts("snake_case_name and 2*3*4", Kind::Italic).is_empty());
        assert!(texts("**bold**", Kind::Italic).is_empty());
        assert_eq!(texts("__strong__ text", Kind::Bold), ["__strong__"]);
        assert_eq!(texts("a [link](https://x.dev) here", Kind::Link), ["[link](https://x.dev)"]);
        assert_eq!(texts("(see http://a.b/c) then", Kind::Link), ["http://a.b/c"]);
        assert!(texts("an `unclosed tick", Kind::Code).is_empty());
        assert_eq!(texts("é *ü* ö", Kind::Italic), ["*ü*"]);
    }

    #[test]
    fn heading_levels_and_tasks() {
        let text = "# One\n### Three\n- [ ] milk\n- [x] eggs\n* [ ]";
        let all = spans(text);
        assert_eq!(all.iter().filter(|s| s.kind == Kind::Heading).map(|s| s.level).collect::<Vec<_>>(), [1, 3]);
        assert_eq!(texts(text, Kind::Checkbox), ["[ ]", "[x]", "[ ]"]);
        assert_eq!(texts(text, Kind::Done), ["eggs"], "only the ticked task is struck through");
        assert_eq!(texts(text, Kind::ListMarker)[0], "-");
        assert!(texts("####### seven", Kind::Heading).is_empty());
        assert!(texts("#nospace", Kind::Heading).is_empty());
        assert_eq!(texts("12. [X] done it", Kind::Done), ["done it"]);
    }

    #[test]
    fn ticking_a_box() {
        let text = "- [ ] milk\n- [x] eggs";
        assert_eq!(toggled_box(text, 2), Some((3..4, "x")));
        assert_eq!(toggled_box(text, 13), Some((14..15, " ")));
        assert_eq!(toggled_box(text, 0), None);
        assert_eq!(toggled_box(text, 99), None);
    }

    #[test]
    fn return_carries_lists_on() {
        use Newline::*;
        assert_eq!(newline("- milk", ""), ContinueList("- ".into()));
        assert_eq!(newline("  * [x] eggs", ""), ContinueList("  * [ ] ".into()));
        assert_eq!(newline("9. bread", ""), ContinueList("10. ".into()));
        assert_eq!(newline("1) a", ""), ContinueList("2) ".into()));
        assert_eq!(newline("- [ ] ", ""), EndList);
        assert_eq!(newline("- ", ""), EndList);
        assert_eq!(newline("- ", "rest"), Plain);
        assert_eq!(newline("just text", ""), Plain);
        assert_eq!(newline("-dash", ""), Plain);
        assert_eq!(newline("1234567890. ten digits", ""), Plain);
        assert_eq!(newline("", ""), Plain);
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tb-notes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_store_makes_a_key_and_keeps_notes_sealed() {
        let home = temp("store");
        let mut store = Store::new(&home);
        store.load();
        assert!(store.problem.is_none());
        let first = store.create().unwrap();
        let second = store.create().unwrap();
        assert_eq!(store.get(&second).unwrap().title, "Untitled note 2");
        assert!(store.set_body(&first, "Buy milk"));
        assert!(!store.set_body(&first, "Buy milk"));
        store.save(&first);
        assert!(store.rename(&second, " Groceries "));
        assert!(!store.rename(&second, "Groceries"));
        let on_disk = std::fs::read(home.join("notes").join(format!("{first}.tbnote"))).unwrap();
        assert!(!on_disk.windows(4).any(|w| w == b"milk"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode(&home.join("secrets").join("notes-key")), 0o600);
            assert_eq!(mode(&home.join("notes")), 0o700);
            assert_eq!(mode(&home.join("notes").join(format!("{first}.tbnote"))), 0o600);
        }
        let mut again = Store::new(&home);
        again.load();
        assert_eq!(again.sorted().iter().map(|n| n.title.as_str()).collect::<Vec<_>>(), ["Groceries", "Untitled note"]);
        assert_eq!(again.get(&first).unwrap().body, "Buy milk");
        again.delete(&first);
        let mut third = Store::new(&home);
        third.load();
        assert_eq!(third.notes.len(), 1);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn notes_with_no_key_are_left_alone() {
        let home = temp("nokey");
        crate::private_file::write(&home.join("notes").join("A.tbnote"), &unhex(FROM_SWIFT)).unwrap();
        let mut store = Store::new(&home);
        store.load();
        assert_eq!(store.problem.as_deref(), Some(NO_KEY));
        assert!(!home.join("secrets").join("notes-key").exists(), "no new key over notes it can't open");
        assert!(store.create().is_none());
        // With the Swift app's key moved into its file, they open.
        crate::private_file::write(&home.join("secrets").join("notes-key"), &KEY).unwrap();
        let mut store = Store::new(&home);
        store.load();
        assert_eq!(store.notes[0].title, "From Swift");
        let _ = std::fs::remove_dir_all(&home);
    }
}
