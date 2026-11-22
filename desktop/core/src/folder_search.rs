// Searching the text inside a folder's files, for the search box over a folder
// in the side pane, a port of the Swift app's FolderSearch
// (ThreadbearCore/Threads/FolderSearch.swift, Wayne's 3708b1c7): every line
// that has the words, with its file and line number, so a click opens the file
// there. Folders nobody means to search (`.git`, `node_modules`, build output)
// are skipped, and so are big files and anything that isn't text.
//
// It reads this machine's disk, as the Swift one does: only a folder on this
// runner's Mac gets the box, as another Mac's folder has no copy here. It runs
// off the main thread; the caller stops it by setting `stop`.
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// One line with the words in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    /// The file's full path.
    pub path: String,
    /// From 1, as editors count them.
    pub line: usize,
    /// The line, trimmed and cut short around the match.
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Found {
    pub hits: Vec<Hit>,
    /// More matched than the limit: the list stops there.
    pub cut: bool,
    /// Files whose text was read.
    pub searched: usize,
}

/// Folders skipped wherever they are: version control, dependencies, build
/// output and caches.
pub const SKIPPED_FOLDERS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    ".build",
    "build",
    "dist",
    "out",
    ".next",
    ".nuxt",
    ".turbo",
    ".cache",
    ".parcel-cache",
    "DerivedData",
    "Pods",
    ".venv",
    "venv",
    "__pycache__",
    "target",
    ".gradle",
    ".idea",
    "coverage",
    ".expo",
    ".svelte-kit",
    ".vercel",
    ".convex",
];

/// Folders macOS shows as one file (`isPackage`): an app, a bundle. Never
/// searched inside, as the Finder never opens them.
const PACKAGES: &[&str] = &[
    "app",
    "bundle",
    "framework",
    "plugin",
    "kext",
    "xcassets",
    "xcodeproj",
    "xcworkspace",
    "photoslibrary",
    "pages",
    "numbers",
    "key",
];

/// Bigger files aren't read: they're data or generated, and slow.
pub const MAX_FILE_SIZE: u64 = 1_000_000;
pub const DEFAULT_LIMIT: usize = 500;
/// The line shown for a hit is cut to about this many characters.
pub const SHOWN_LENGTH: usize = 160;

pub fn skips_folder(name: &str) -> bool {
    SKIPPED_FOLDERS.contains(&name)
        || name.rsplit_once('.').is_some_and(|(stem, ext)| {
            !stem.is_empty() && PACKAGES.contains(&ext.to_ascii_lowercase().as_str())
        })
}

/// The lines of `text` with `query` in them, whatever their case, as (line
/// number from 1, the line as shown).
pub fn matches(text: &str, query: &str, limit: usize) -> Vec<(usize, String)> {
    let query = query.trim();
    if query.is_empty() || limit == 0 {
        return vec![];
    }
    let needle = query.to_lowercase();
    let mut found = vec![];
    for (i, line) in text.lines().enumerate() {
        if let Some(at) = find_lower(line, &needle) {
            found.push((i + 1, shown(line, at)));
            if found.len() >= limit {
                break;
            }
        }
    }
    found
}

/// Where `needle` (lowercase) starts in `line`, whatever its case, in
/// characters.
fn find_lower(line: &str, needle: &str) -> Option<usize> {
    let lower: String = line.chars().flat_map(char::to_lowercase).collect();
    // Lowercasing can change a line's length (İ), so the place is counted in
    // the lowered line's characters and used as roughly the same in the line.
    let byte = lower.find(needle)?;
    Some(lower[..byte].chars().count())
}

/// A line for the results list: leading space gone, and a long one cut to a
/// window around the match (`at`, in characters) with … at the cut ends.
pub fn shown(line: &str, at: usize) -> String {
    let chars: Vec<char> = line.chars().collect();
    let leading = chars
        .iter()
        .take_while(|c| **c == ' ' || **c == '\t')
        .count();
    let trimmed = &chars[leading.min(chars.len())..];
    if at < leading {
        return chars.iter().take(SHOWN_LENGTH).collect();
    }
    if trimmed.len() <= SHOWN_LENGTH {
        return trimmed.iter().collect::<String>().trim().to_owned();
    }
    let before = at - leading;
    let from = before.saturating_sub(40);
    let to = (from + SHOWN_LENGTH).min(trimmed.len());
    let mut out = String::new();
    if from > 0 {
        out.push('…');
    }
    out.extend(&trimmed[from..to]);
    if to < trimmed.len() {
        out.push('…');
    }
    out
}

/// Searches every text file under `root`. Hidden files and folders are
/// searched (a `.env.example`, `.github`), except the skipped ones; links are
/// never followed, so nothing outside the folder is read.
pub fn search(root: &str, query: &str, limit: usize, stop: &AtomicBool) -> Found {
    let mut found = Found::default();
    let query = query.trim();
    if query.is_empty() {
        return found;
    }
    let mut files = vec![];
    let mut folders = vec![std::path::PathBuf::from(root)];
    while let Some(folder) = folders.pop() {
        if stop.load(Ordering::Relaxed) {
            return found;
        }
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            // The entry's own kind: a link is a link, not what it points at.
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            if kind.is_dir() {
                if !skips_folder(&name) {
                    folders.push(entry.path());
                }
            } else if kind.is_file() {
                let size = entry.metadata().map_or(0, |m| m.len());
                if size > 0 && size <= MAX_FILE_SIZE {
                    files.push(entry.path().to_string_lossy().into_owned());
                }
            }
        }
    }
    // In path order, so the list reads folder by folder.
    files.sort_by_key(|p| p.to_lowercase());
    for path in files {
        if stop.load(Ordering::Relaxed) {
            return found;
        }
        let Some(text) = read_text(Path::new(&path)) else {
            continue;
        };
        found.searched += 1;
        let room = limit - found.hits.len();
        for (line, text) in matches(&text, query, room + 1) {
            if found.hits.len() >= limit {
                found.cut = true;
                return found;
            }
            found.hits.push(Hit {
                path: path.clone(),
                line,
                text,
            });
        }
    }
    found
}

/// A file's text, if it reads as text: UTF-8, else Latin-1, as the Swift
/// search falls back to.
fn read_text(path: &Path) -> Option<String> {
    let data = std::fs::read(path).ok()?;
    if !crate::highlights::looks_like_text(&data[..data.len().min(8192)]) {
        return None;
    }
    Some(match String::from_utf8(data) {
        Ok(text) => text,
        Err(e) => e.into_bytes().iter().map(|b| *b as char).collect(),
    })
}

/// The path under `root`, for the results list: `src/app.ts`.
pub fn relative<'a>(path: &'a str, root: &str) -> &'a str {
    // Either separator, as Windows' paths use `\\` (the fork's Windows CI,
    // 16 Nov): the root, then the separator after it.
    let root = root.trim_end_matches(['/', '\\']);
    match path.strip_prefix(root) {
        Some(rest) if rest.starts_with(['/', '\\']) => &rest[1..],
        _ => path,
    }
}

/// The hits a file at a time, in the order found: (the file, its hits).
pub fn by_file(hits: &[Hit]) -> Vec<(&str, Vec<&Hit>)> {
    let mut out: Vec<(&str, Vec<&Hit>)> = vec![];
    for hit in hits {
        match out.last_mut() {
            Some((path, list)) if *path == hit.path => list.push(hit),
            _ => out.push((&hit.path, vec![hit])),
        }
    }
    out
}

/// What the results say above the list: "12 matches in 3 files", with
/// "(the first 500)" when it was cut.
pub fn summary(found: &Found) -> String {
    let files = by_file(&found.hits).len();
    match found.hits.len() {
        0 => "No matches".to_owned(),
        n => {
            let matches = if n == 1 {
                "1 match".to_owned()
            } else {
                format!("{n} matches")
            };
            let files = if files == 1 {
                "1 file".to_owned()
            } else {
                format!("{files} files")
            };
            let cut = if found.cut {
                format!(" (the first {n})")
            } else {
                String::new()
            };
            format!("{matches} in {files}{cut}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct Folder(std::path::PathBuf);

    impl Folder {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "tb-folder-search-{name}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Folder(path)
        }
        fn write(&self, rel: &str, bytes: &[u8]) {
            let path = self.0.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        fn root(&self) -> String {
            self.0.to_string_lossy().into_owned()
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn matches_lines_whatever_their_case() {
        let found = matches(
            "one\n  Hello World\nnothing\nhello again",
            "hello",
            usize::MAX,
        );
        assert_eq!(found.iter().map(|f| f.0).collect::<Vec<_>>(), vec![2, 4]);
        assert_eq!(
            found.iter().map(|f| f.1.as_str()).collect::<Vec<_>>(),
            vec!["Hello World", "hello again"]
        );
        assert!(matches("abc", "  ", usize::MAX).is_empty());
    }

    #[test]
    fn a_long_line_is_cut_round_the_match() {
        let line = format!("{}NEEDLE{}", "a".repeat(300), "b".repeat(300));
        let shown = &matches(&line, "needle", usize::MAX)[0].1;
        assert!(shown.contains("NEEDLE"));
        assert!(shown.starts_with('…') && shown.ends_with('…'));
        assert!(shown.chars().count() <= SHOWN_LENGTH + 2);
    }

    #[test]
    fn dependencies_build_output_and_binaries_are_skipped() {
        let dir = Folder::new("skips");
        dir.write("src/app.ts", b"const port = 5173\n// TODO: port");
        dir.write("README.md", b"Run on port 5173");
        dir.write(".github/workflows/ci.yml", b"port: 5173");
        dir.write("node_modules/vite/index.js", b"port 5173");
        dir.write(".git/config", b"port 5173");
        dir.write("build/out.js", b"port 5173");
        dir.write("Tool.app/Contents/x.txt", b"port 5173");
        dir.write("logo.png", &[0, 1, 2, b'p', b'o', b'r', b't']);
        let root = dir.root();
        let found = search(&root, "port", DEFAULT_LIMIT, &AtomicBool::new(false));
        // As `/` everywhere, so the list reads the same on Windows.
        let mut files: Vec<String> = found
            .hits
            .iter()
            .map(|h| relative(&h.path, &root).replace('\\', "/"))
            .collect();
        files.dedup();
        assert_eq!(
            files,
            vec![".github/workflows/ci.yml", "README.md", "src/app.ts"]
        );
        assert_eq!(
            found
                .hits
                .iter()
                .filter(|h| h.path.ends_with("app.ts"))
                .map(|h| h.line)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert!(!found.cut);
        assert_eq!(summary(&found), "4 matches in 3 files");
    }

    #[test]
    fn the_limit_cuts_the_list() {
        let dir = Folder::new("limit");
        let text: Vec<String> = (1..=20).map(|i| format!("match {i}")).collect();
        dir.write("a.txt", text.join("\n").as_bytes());
        let found = search(&dir.root(), "match", 5, &AtomicBool::new(false));
        assert_eq!(found.hits.len(), 5);
        assert!(found.cut);
        assert_eq!(summary(&found), "5 matches in 1 file (the first 5)");
    }

    #[test]
    fn stop_ends_the_search() {
        let dir = Folder::new("stop");
        dir.write("a.txt", b"match");
        assert!(
            search(&dir.root(), "match", DEFAULT_LIMIT, &AtomicBool::new(true))
                .hits
                .is_empty()
        );
    }

    #[test]
    fn a_link_out_of_the_folder_is_never_followed() {
        let dir = Folder::new("link");
        let outside = Folder::new("link-outside");
        outside.write("secret.txt", b"match");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside.0, dir.0.join("out-there")).unwrap();
        assert!(
            search(&dir.root(), "match", DEFAULT_LIMIT, &AtomicBool::new(false))
                .hits
                .is_empty()
        );
    }

    #[test]
    fn paths_under_the_root() {
        assert_eq!(relative("/x/y/src/a.ts", "/x/y"), "src/a.ts");
        assert_eq!(relative("/x/y/src/a.ts", "/x/y/"), "src/a.ts");
        assert_eq!(relative("/z/a.ts", "/x/y"), "/z/a.ts");
        assert_eq!(relative("/x/yz/a.ts", "/x/y"), "/x/yz/a.ts");
        assert_eq!(relative(r"C:\x\y\src\a.ts", r"C:\x\y"), r"src\a.ts");
        assert_eq!(relative(r"C:\x\y\src\a.ts", r"C:\x\y\"), r"src\a.ts");
    }
}
