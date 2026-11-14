//! The Notes page's notes (gpui/src/notes_page.rs), kept as the Swift app
//! keeps them so the same notes open in either app: one file per note in
//! `<data folder>/notes/<id>.tbnote`, each a version byte (1) and then
//! AES-256-GCM's nonce, sealed JSON and tag (CryptoKit's `combined`), under
//! the 32-byte key in `<data folder>/secrets/notes-key`. The JSON is Swift's
//! `Note` as JSONEncoder writes it: `id`, `title`, `body`, and `createdAt` and
//! `updatedAt` as seconds since 1 Jan 2001 (Foundation's reference date).
//! Folders 0700, files 0600 (private_file). Nothing here is sent anywhere.
//!
//! The Swift app kept the key in the Keychain until 7 Nov, and moves it into
//! the file the first time Notes opens there. This app never reads the
//! Keychain: macOS would ask for the Mac password, as the item belongs to
//! the Swift app's signature. With notes on disk but no key file, it says so
//! and leaves the notes as they are, for the Swift app to move the key.

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom as _, SystemRandom};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Seconds from 1 Jan 1970 to 1 Jan 2001, Foundation's reference date.
const REFERENCE_DATE: f64 = 978_307_200.;

/// The note's file's first byte.
const VERSION: u8 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub title: String,
    pub body: String,
    /// Seconds since 1 Jan 2001, as Swift's Date encodes.
    pub created_at: f64,
    pub updated_at: f64,
}

impl Note {
    pub fn new(title: &str, now: f64) -> Self {
        Note { id: new_id(), title: title.to_owned(), body: String::new(), created_at: now, updated_at: now }
    }

    /// When it last changed, in milliseconds since 1970.
    pub fn updated_ms(&self) -> i64 {
        ((self.updated_at + REFERENCE_DATE) * 1000.) as i64
    }

    /// The list's second line: the first line with words in it.
    pub fn preview(&self) -> Option<&str> {
        self.body.lines().map(str::trim).find(|l| !l.is_empty())
    }
}

/// Now, as a note's dates are kept.
pub fn now() -> f64 {
    let since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    since.as_secs_f64() - REFERENCE_DATE
}

/// An id as Swift's `UUID().uuidString` makes them: random (version 4),
/// upper case.
pub fn new_id() -> String {
    let mut b = [0u8; 16];
    SystemRandom::new().fill(&mut b).expect("system randomness for note id");
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02X}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}

#[derive(Debug, PartialEq)]
pub enum Failure {
    BadData,
}

fn cipher(key: &[u8; 32]) -> LessSafeKey {
    LessSafeKey::new(UnboundKey::new(&AES_256_GCM, key).expect("a 32-byte key"))
}

/// The note as stored (`NoteCrypto.seal`).
pub fn seal(note: &Note, key: &[u8; 32]) -> Vec<u8> {
    let mut nonce = [0u8; NONCE_LEN];
    SystemRandom::new().fill(&mut nonce).expect("system randomness for encryption nonce");
    let mut data = serde_json::to_vec(note).expect("a note serializes");
    cipher(key).seal_in_place_append_tag(Nonce::assume_unique_for_key(nonce), Aad::empty(), &mut data).expect("sealing fits");
    let mut out = Vec::with_capacity(1 + NONCE_LEN + data.len());
    out.push(VERSION);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&data);
    out
}

/// A stored note opened (`NoteCrypto.open`).
pub fn open(data: &[u8], key: &[u8; 32]) -> Result<Note, Failure> {
    if data.first() != Some(&VERSION) || data.len() < 1 + NONCE_LEN + 16 {
        return Err(Failure::BadData);
    }
    let nonce = Nonce::try_assume_unique_for_key(&data[1..1 + NONCE_LEN]).map_err(|_| Failure::BadData)?;
    let mut sealed = data[1 + NONCE_LEN..].to_vec();
    let plain = cipher(key).open_in_place(nonce, Aad::empty(), &mut sealed).map_err(|_| Failure::BadData)?;
    serde_json::from_slice(plain).map_err(|_| Failure::BadData)
}

/// `NoteNames`: the default names, and a title as kept.
pub mod names {
    pub const BASE: &str = "Untitled note";

    /// "Untitled note", or the first "Untitled note N" not taken.
    pub fn next<'a>(titles: impl IntoIterator<Item = &'a str>) -> String {
        let taken: std::collections::HashSet<String> = titles.into_iter().map(|t| t.trim().to_lowercase()).collect();
        if !taken.contains(&BASE.to_lowercase()) {
            return BASE.to_owned();
        }
        (2..).map(|n| format!("{BASE} {n}")).find(|t| !taken.contains(&t.to_lowercase())).unwrap_or_default()
    }

    /// Trimmed, one line, never empty, 200 characters at most.
    pub fn clean(title: &str, fallback: &str) -> String {
        let one = title.replace('\n', " ");
        let one = one.trim();
        if one.is_empty() {
            fallback.to_owned()
        } else {
            one.chars().take(200).collect()
        }
    }
}

/// Why the notes can't be opened.
#[derive(Debug, PartialEq)]
pub enum Locked {
    /// Notes on disk, but their key is still in the Keychain, where only the
    /// Swift app can read it without a password.
    KeyInKeychain,
    /// The key file couldn't be read or made.
    NoKey,
}

impl Locked {
    pub fn text(&self) -> &'static str {
        match self {
            Locked::KeyInKeychain => "Your notes' key is still in the Keychain from an older Threadbear. Open Notes once in the older app to move it, and they'll open here too.",
            Locked::NoKey => "Your notes couldn't be unlocked: Threadbear couldn't read their key.",
        }
    }
}

/// The notes in a data folder (`NotesStore`): read whole when Notes first
/// opens, and each note written whole when it changes.
pub struct Store {
    folder: PathBuf,
    key_file: PathBuf,
    key: Option<[u8; 32]>,
    pub notes: Vec<Note>,
    /// Notes whose file couldn't be opened (a different key, damage): left alone.
    pub unreadable: usize,
}

impl Store {
    /// The notes of the data folder `home` (the runner's, as the app's settings are).
    pub fn in_home(home: &Path) -> Self {
        Self::new(home.join("notes"), home.join("secrets").join("notes-key"))
    }

    pub fn new(folder: PathBuf, key_file: PathBuf) -> Self {
        Store { folder, key_file, key: None, notes: vec![], unreadable: 0 }
    }

    pub fn is_open(&self) -> bool {
        self.key.is_some()
    }

    fn has_note_files(&self) -> bool {
        std::fs::read_dir(&self.folder).map(|d| d.flatten().any(|e| e.path().extension().is_some_and(|x| x == "tbnote"))).unwrap_or(false)
    }

    /// The key from its file, or a new one when there are no notes yet.
    fn load_key(&self) -> Result<[u8; 32], Locked> {
        if let Ok(data) = std::fs::read(&self.key_file) {
            if let Ok(key) = <[u8; 32]>::try_from(data.as_slice()) {
                return Ok(key);
            }
        }
        if self.key_file.exists() { return Err(Locked::NoKey); }
        if self.has_note_files() {
            return Err(Locked::KeyInKeychain);
        }
        let mut key = [0u8; 32];
        SystemRandom::new().fill(&mut key).map_err(|_| Locked::NoKey)?;
        crate::private_file::write(&self.key_file, &key).map_err(|_| Locked::NoKey)?;
        Ok(key)
    }

    /// Reads the notes, once their key's there. Nothing is written but a new
    /// key, and only into an empty notes folder's data folder.
    pub fn load(&mut self) -> Result<(), Locked> {
        if self.key.is_some() {
            return Ok(());
        }
        let key = self.load_key()?;
        crate::private_file::dir(&self.folder).map_err(|_| Locked::NoKey)?;
        let mut notes = vec![];
        let mut unreadable = 0;
        for entry in std::fs::read_dir(&self.folder).map_err(|_| Locked::NoKey)?.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|x| x != "tbnote") {
                continue;
            }
            match std::fs::read(&path).ok().and_then(|d| open(&d, &key).ok()) {
                Some(note) => notes.push(note),
                None => unreadable += 1,
            }
        }
        self.notes = notes;
        self.unreadable = unreadable;
        self.key = Some(key);
        Ok(())
    }

    /// Newest first.
    pub fn sorted(&self) -> Vec<&Note> {
        let mut out: Vec<&Note> = self.notes.iter().collect();
        out.sort_by(|a, b| b.updated_at.total_cmp(&a.updated_at));
        out
    }

    pub fn get(&self, id: &str) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    fn file(&self, id: &str) -> PathBuf {
        self.folder.join(format!("{id}.tbnote"))
    }

    /// Writes one note whole, sealed, 0600 (no file protection: macOS refuses it).
    pub fn write(&self, id: &str) -> std::io::Result<()> {
        let (Some(key), Some(note)) = (self.key.as_ref(), self.get(id)) else { return Ok(()) };
        crate::private_file::write(&self.file(id), &seal(note, key))
    }

    /// A new, empty note with the next free default name, written at once.
    pub fn create(&mut self) -> Option<String> {
        self.key?;
        let note = Note::new(&names::next(self.notes.iter().map(|n| n.title.as_str())), now());
        let id = note.id.clone();
        self.notes.push(note);
        let _ = self.write(&id);
        Some(id)
    }

    /// Renames it (cleaned), and writes it if that changed anything.
    pub fn rename(&mut self, id: &str, title: &str) -> bool {
        let Some(note) = self.notes.iter_mut().find(|n| n.id == id) else { return false };
        let clean = names::clean(title, &note.title);
        if clean == note.title {
            return false;
        }
        note.title = clean;
        note.updated_at = now();
        let _ = self.write(id);
        true
    }

    /// The body as typed; the caller writes it a moment after the typing stops.
    pub fn set_body(&mut self, id: &str, body: &str) -> bool {
        let Some(note) = self.notes.iter_mut().find(|n| n.id == id) else { return false };
        if note.body == body {
            return false;
        }
        note.body = body.to_owned();
        note.updated_at = now();
        true
    }

    pub fn delete(&mut self, id: &str) -> std::io::Result<()> {
        std::fs::remove_file(self.file(id))?;
        self.notes.retain(|n| n.id != id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tb-notes-{}", new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Made by the Swift app's `NoteCrypto.seal` (CryptoKit), with this key.
    const SWIFT_KEY: [u8; 32] = [7; 32];
    const SWIFT_NOTE: &str = include_str!("../tests/fixtures/notes/swift-note.hex");

    fn unhex(s: &str) -> Vec<u8> {
        let s = s.trim();
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn a_note_the_swift_app_sealed_opens() {
        let note = open(&unhex(SWIFT_NOTE), &SWIFT_KEY).unwrap();
        assert_eq!(note.title, "From Swift");
        assert_eq!(note.body, "# Groceries\n- [ ] milk");
        assert_eq!(note.id, "6D1C4F0A-3B2E-4C55-9A10-2F6B7C8D9E01");
        assert_eq!(note.created_at, 784_000_000.5);
    }

    #[test]
    fn sealed_here_opens_here_and_not_with_another_key() {
        let mut note = Note::new("Hello", 784_000_000.25);
        note.body = "the body".into();
        let data = seal(&note, &SWIFT_KEY);
        assert_eq!(data[0], 1);
        assert_eq!(open(&data, &SWIFT_KEY).unwrap(), note);
        assert_eq!(open(&data, &[8; 32]), Err(Failure::BadData));
    }

    #[test]
    fn ids_look_like_swifts() {
        let id = new_id();
        assert_eq!(id.len(), 36);
        assert_eq!(id, id.to_uppercase());
        assert_eq!(&id[14..15], "4");
    }

    #[test]
    fn names_count_on() {
        assert_eq!(names::next([]), "Untitled note");
        assert_eq!(names::next(["untitled note ", "Untitled note 2"]), "Untitled note 3");
        assert_eq!(names::clean("  a\nb ", "x"), "a b");
        assert_eq!(names::clean("   ", "x"), "x");
    }

    #[test]
    fn a_new_key_goes_in_a_file_only_you_can_read_and_notes_open_again() {
        let home = temp();
        let mut store = Store::in_home(&home);
        store.load().unwrap();
        let key = std::fs::read(home.join("secrets/notes-key")).unwrap();
        assert_eq!(key.len(), 32);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(std::fs::metadata(home.join("secrets/notes-key")).unwrap().permissions().mode() & 0o777, 0o600);
            assert_eq!(std::fs::metadata(home.join("notes")).unwrap().permissions().mode() & 0o777, 0o700);
        }
        let id = store.create().unwrap();
        assert!(store.set_body(&id, "remember the milk"));
        store.write(&id).unwrap();
        assert!(store.rename(&id, "Shopping"));
        let mut again = Store::in_home(&home);
        again.load().unwrap();
        assert_eq!(again.notes.len(), 1);
        assert_eq!(again.notes[0].body, "remember the milk");
        assert_eq!(again.notes[0].title, "Shopping");
        again.delete(&id).unwrap();
        assert!(!home.join(format!("notes/{id}.tbnote")).exists());
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn notes_without_their_key_file_are_left_alone() {
        let home = temp();
        std::fs::create_dir_all(home.join("notes")).unwrap();
        std::fs::write(home.join("notes/A.tbnote"), [1, 2, 3]).unwrap();
        let mut store = Store::in_home(&home);
        assert_eq!(store.load(), Err(Locked::KeyInKeychain));
        assert!(!home.join("secrets/notes-key").exists());
        assert_eq!(std::fs::read(home.join("notes/A.tbnote")).unwrap(), vec![1, 2, 3]);
        let _ = std::fs::remove_dir_all(home);
    }
    #[test]
    fn damaged_notes_and_keys_are_preserved() {
        let home = temp();
        let mut store = Store::in_home(&home);
        store.load().unwrap();
        std::fs::write(home.join("notes/damaged.tbnote"), b"damaged ciphertext").unwrap();
        let mut again = Store::in_home(&home);
        again.load().unwrap();
        assert_eq!(again.unreadable, 1);
        assert_eq!(std::fs::read(home.join("notes/damaged.tbnote")).unwrap(), b"damaged ciphertext");
        std::fs::remove_file(home.join("notes/damaged.tbnote")).unwrap();
        std::fs::write(home.join("secrets/notes-key"), b"damaged key").unwrap();
        let mut again = Store::in_home(&home);
        assert_eq!(again.load(), Err(Locked::NoKey));
        assert_eq!(std::fs::read(home.join("secrets/notes-key")).unwrap(), b"damaged key");
        let _ = std::fs::remove_dir_all(home);
    }

}
