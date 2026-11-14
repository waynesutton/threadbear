// Searching the text inside a folder's files, for the search field over a
// folder in the side pane (`FolderSearch.swift`): every line that has the
// words, with its file and line number, so a click opens the file there.
// Folders nobody means to search (`.git`, `node_modules`, build output) are
// skipped, and so are big files and anything that isn't text. Runs off the
// main thread; the caller stops it by returning true from `stop`.
use std::path::Path;

/// One matching line.
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
    ".git", ".hg", ".svn", "node_modules", ".build", "build", "dist", "out", ".next", ".nuxt", ".turbo", ".cache", ".parcel-cache", "DerivedData", "Pods", ".venv", "venv", "__pycache__", "target", ".gradle", ".idea", "coverage", ".expo",
    ".svelte-kit", ".vercel", ".convex",
];

/// Bigger files aren't read: they're data or generated, and slow.
pub const MAX_FILE_SIZE: u64 = 1_000_000;
pub const DEFAULT_LIMIT: usize = 500;
/// The line shown for a hit is cut to about this many characters.
pub const SHOWN_LENGTH: usize = 160;

pub fn skips(folder: &str) -> bool {
    SKIPPED_FOLDERS.contains(&folder)
}

/// The lines of `text` with `query` in them, without case, as (line number
/// from 1, the line as shown).
pub fn matches(text: &str, query: &str, limit: usize) -> Vec<(usize, String)> {
    let query = query.trim().to_lowercase();
    if query.is_empty() || limit == 0 {
        return vec![];
    }
    let mut found = vec![];
    for (i, line) in text.lines().enumerate() {
        if let Some(at) = line.to_lowercase().find(&query) {
            found.push((i + 1, shown(line, at)));
            if found.len() >= limit {
                break;
            }
        }
    }
    found
}

/// A line for the results list: leading space gone, and a long one cut to a
/// window around the match (`at`, a byte offset into the lowercased line,
/// which only differs in length for a few scripts) with … at the cut ends.
pub fn shown(line: &str, at: usize) -> String {
    let chars: Vec<char> = line.chars().collect();
    let leading = chars.iter().take_while(|c| **c == ' ' || **c == '\t').count();
    let trimmed = &chars[leading..];
    if trimmed.len() <= SHOWN_LENGTH {
        return trimmed.iter().collect::<String>().trim_end().to_owned();
    }
    let before = line.get(..at.min(line.len())).map_or(0, |s| s.chars().count()).saturating_sub(leading);
    let from = before.saturating_sub(40);
    let to = (from + SHOWN_LENGTH).min(trimmed.len());
    format!("{}{}{}", if from > 0 { "…" } else { "" }, trimmed[from..to].iter().collect::<String>(), if to < trimmed.len() { "…" } else { "" })
}

/// Searches every text file under `root`. Hidden files and folders are
/// searched (a `.env.example`, `.github`), except the skipped ones.
pub fn search(root: &str, query: &str, limit: usize, stop: &dyn Fn() -> bool) -> Found {
    let mut found = Found::default();
    let query = query.trim();
    if query.is_empty() {
        return found;
    }
    let mut files = vec![];
    let mut folders = vec![Path::new(root).to_path_buf()];
    while let Some(folder) = folders.pop() {
        if stop() {
            return found;
        }
        let Ok(entries) = std::fs::read_dir(&folder) else { continue };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else { continue };
            let path = entry.path();
            if kind.is_dir() {
                let name = entry.file_name();
                if !skips(&name.to_string_lossy()) {
                    folders.push(path);
                }
            } else if kind.is_file() {
                let size = entry.metadata().map_or(0, |m| m.len());
                if size > 0 && size <= MAX_FILE_SIZE {
                    files.push(path);
                }
            }
        }
    }
    // In path order, so the list reads folder by folder.
    files.sort();
    for file in files {
        if stop() {
            return found;
        }
        let Ok(data) = std::fs::read(&file) else { continue };
        if !crate::highlights::looks_like_text(&data[..data.len().min(8192)]) {
            continue;
        }
        let text = String::from_utf8_lossy(&data);
        found.searched += 1;
        let room = limit - found.hits.len();
        for (line, shown) in matches(&text, query, room + 1) {
            if found.hits.len() >= limit {
                found.cut = true;
                return found;
            }
            found.hits.push(Hit { path: file.to_string_lossy().into_owned(), line, text: shown });
        }
    }
    found
}

/// The path under `root`, for the results list: `src/app.ts`.
pub fn relative<'a>(path: &'a str, root: &str) -> &'a str {
    let base = if root.ends_with('/') { root.to_owned() } else { format!("{root}/") };
    path.strip_prefix(base.as_str()).unwrap_or(path)
}

/// The line over the results: "3 lines in 2 files", or the first so many.
pub fn summary(found: &Found) -> String {
    let lines = found.hits.len();
    let mut files: Vec<&str> = found.hits.iter().map(|h| h.path.as_str()).collect();
    files.dedup();
    let base = format!("{lines} {} in {} {}", if lines == 1 { "line" } else { "lines" }, files.len(), if files.len() == 1 { "file" } else { "files" });
    if found.cut {
        format!("The first {base}. Add more words to narrow it.")
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_with_the_words_whatever_their_case() {
        let text = "one\n  Two words\nthree\nTWO again";
        assert_eq!(matches(text, "two", 10), vec![(2, "Two words".to_owned()), (4, "TWO again".to_owned())]);
        assert_eq!(matches(text, "two", 1).len(), 1);
        assert!(matches(text, "  ", 10).is_empty());
    }

    #[test]
    fn a_long_line_is_cut_around_the_match() {
        let line = format!("{}needle{}", "a".repeat(300), "b".repeat(300));
        let at = line.find("needle").unwrap();
        let shown = shown(&line, at);
        assert!(shown.starts_with('…') && shown.ends_with('…') && shown.contains("needle"), "{shown}");
    }

    #[test]
    fn it_skips_dependencies_and_reads_text_only() {
        let root = std::env::temp_dir().join(format!("tb-folder-search-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("node_modules/x")).unwrap();
        std::fs::write(root.join("src/app.ts"), "const port = 3000;\n// the port\n").unwrap();
        std::fs::write(root.join("node_modules/x/index.js"), "port\n").unwrap();
        std::fs::write(root.join("logo.png"), [0u8, 159, 146, 150, 0, 0, 1]).unwrap();
        let root_s = root.to_string_lossy().into_owned();
        let found = search(&root_s, "PORT", DEFAULT_LIMIT, &|| false);
        assert_eq!(found.hits.iter().map(|h| (relative(&h.path, &root_s), h.line)).collect::<Vec<_>>(), vec![("src/app.ts", 1), ("src/app.ts", 2)]);
        assert_eq!(summary(&found), "2 lines in 1 file");
        let cut = search(&root_s, "port", 1, &|| false);
        assert!(cut.cut);
        assert_eq!(summary(&cut), "The first 1 line in 1 file. Add more words to narrow it.");
        let _ = std::fs::remove_dir_all(&root);
    }
}
