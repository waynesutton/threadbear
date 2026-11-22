// Prints (docs/protocol.md, "Prints"; "Pages" until 10 Nov 2026, and the
// protocol still says page): the named, versioned pages agents make, kept by
// the runner, in zero or more projects. A port of the Swift app's Page.swift
// (ThreadbearCore/Protocol) and PageFence (RichBlock.swift): what a Print
// is, what its rows and cards say, the ```print fence in a reply, where its
// files are, and the links a todo list or board carries. The pages
// experiment; with it off the runner sends none.
use crate::inbox::{ago, parse_ms};
use serde_json::Value;

/// The experiment's id, as in hello's `experiments`.
pub const EXPERIMENT: &str = "pages";

/// How long Refresh or Edit with Agent marks a card as under way, unless a
/// newer version lands first (`PageCards.refreshFor`).
pub const REFRESH_FOR_MS: i64 = 20 * 60 * 1000;

/// A project a Print is in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectRef {
    pub id: String,
    pub pinned: bool,
}

/// A Print, as `pages.list`, `pages.get` and `pages.updated` give it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub owner_thread: String,
    pub projects: Vec<ProjectRef>,
    pub version: u32,
    pub bytes: u64,
    /// Scripts allowed: they run only after Run them.
    pub live: bool,
    /// page, chart, dashboard, report, doc, diagram, bundle, todo or board.
    pub kind: String,
    pub recipe: Option<String>,
    /// The newest version was written in the app's editor, or by a click.
    pub edited_by_user: bool,
    /// Its link's slug while published (the website's /p/<slug>).
    pub published_slug: Option<String>,
    /// While deleted (kept 7 days for Restore), in ms since 1970.
    pub deleted_at: Option<i64>,
    pub updated_at: i64,
}

impl Page {
    /// One Print, or None for anything without an id and a title. Every
    /// other field has a default, so a newer runner's extras don't matter.
    pub fn decode(v: &Value) -> Option<Page> {
        let id = v["id"].as_str()?.to_owned();
        let title = v["title"].as_str()?.to_owned();
        let projects = v["projects"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| {
                Some(ProjectRef {
                    id: p["id"].as_str()?.to_owned(),
                    pinned: p["pinned"].as_bool().unwrap_or(false),
                })
            })
            .collect();
        let kind = v["kind"]
            .as_str()
            .filter(|k| !k.is_empty())
            .unwrap_or("page")
            .to_owned();
        Some(Page {
            id,
            title,
            summary: v["summary"].as_str().unwrap_or("").to_owned(),
            owner_thread: v["ownerThread"].as_str().unwrap_or("").to_owned(),
            projects,
            version: v["version"]
                .as_u64()
                .map(|n| n.clamp(1, u32::MAX as u64) as u32)
                .unwrap_or(1),
            bytes: v["bytes"].as_u64().unwrap_or(0),
            live: v["live"].as_bool().unwrap_or(false),
            kind,
            recipe: v["recipe"].as_str().map(str::to_owned),
            edited_by_user: v["editedBy"].as_str() == Some("user"),
            published_slug: v["published"]["slug"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
            deleted_at: v["deletedAt"].as_str().and_then(parse_ms),
            updated_at: v["updatedAt"].as_str().and_then(parse_ms).unwrap_or(0),
        })
    }

    /// `{ pages: [...] }`'s list, skipping what can't be read.
    pub fn list(v: &Value) -> Vec<Page> {
        v.as_array()
            .into_iter()
            .flatten()
            .filter_map(Page::decode)
            .collect()
    }

    pub fn is_in(&self, project: &str) -> bool {
        self.projects.iter().any(|p| p.id == project)
    }

    pub fn is_doc(&self) -> bool {
        self.kind == "doc"
    }

    pub fn is_diagram(&self) -> bool {
        self.kind == "diagram"
    }

    /// A checklist or a kanban board: markdown the user changes by clicking.
    pub fn is_list(&self) -> bool {
        self.kind == "todo" || self.kind == "board"
    }

    /// Edit Text is on its menu: a doc, a diagram, a todo list or a board.
    pub fn has_editable_source(&self) -> bool {
        self.is_doc() || self.is_diagram() || self.is_list()
    }

    /// What its kind is called: a plain one is a Print.
    pub fn kind_label(&self) -> String {
        match self.kind.as_str() {
            "page" => "Print".to_owned(),
            "todo" => "Todo list".to_owned(),
            other => {
                let mut c = other.chars();
                c.next()
                    .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            }
        }
    }

    /// The versions the runner still keeps, newest first: the newest ten.
    pub fn kept_versions(&self) -> Vec<u32> {
        (self.version.saturating_sub(9).max(1)..=self.version)
            .rev()
            .collect()
    }

    /// An SF Symbol name for its kind (the app maps it to its icon).
    pub fn symbol(&self) -> &'static str {
        match self.kind.as_str() {
            "chart" => "chart.bar",
            "dashboard" => "rectangle.3.group",
            "report" => "doc.text.magnifyingglass",
            "doc" => "doc.plaintext",
            "diagram" => "point.3.connected.trianglepath.dotted",
            "bundle" => "shippingbox",
            "todo" => "checklist",
            "board" => "rectangle.split.3x1",
            _ => "doc.richtext",
        }
    }

    /// A project page row's words after its title: "Dashboard · v3 · 2h",
    /// with "edited by you" and "published" when they are.
    pub fn row_detail(&self, now_ms: i64) -> String {
        let mut parts = vec![
            self.kind_label(),
            format!("v{}", self.version),
            ago(self.updated_at, now_ms),
        ];
        if self.edited_by_user {
            parts.push("edited by you".into());
        }
        if self.published_slug.is_some() {
            parts.push("published".into());
        }
        parts.join(" · ")
    }

    /// A card's line under its title: "Dashboard · v3 · 2h ago", or what's
    /// under way.
    pub fn card_meta(&self, refreshing: bool, now_ms: i64) -> String {
        if refreshing {
            return format!(
                "{} · v{} · refreshing in its thread…",
                self.kind_label(),
                self.version
            );
        }
        let age = if now_ms - self.updated_at < 90_000 {
            "just now".to_owned()
        } else {
            format!("{} ago", ago(self.updated_at, now_ms))
        };
        format!(
            "{} · v{} · {}{}",
            self.kind_label(),
            self.version,
            age,
            if self.published_slug.is_some() {
                " · published"
            } else {
                ""
            }
        )
    }

    /// A version newer than the first, made in the last 15 minutes: its
    /// card says "New version".
    pub fn is_new_version(&self, refreshing: bool, now_ms: i64) -> bool {
        !refreshing && self.version > 1 && now_ms - self.updated_at < 15 * 60 * 1000
    }

    /// A folder or file name for it: no slashes or colons.
    pub fn file_name(&self) -> String {
        let name: String = self
            .title
            .chars()
            .map(|c| if c == '/' || c == ':' { '-' } else { c })
            .collect();
        let name = name.trim().trim_start_matches('.').to_owned();
        if name.is_empty() {
            self.id.clone()
        } else {
            name
        }
    }
}

/// What a card shows when its Print isn't here.
pub const NOT_HERE: &str = "Not on this Mac. It may be deleted, or on another Mac's runner.";

/// Prints as rows: in a thread's Highlights (the ones it made) and in the
/// rail's Prints list (all of them), each saying which projects it's in.
pub mod rows {
    use super::Page;
    use std::collections::HashMap;

    /// Every Print, newest first, deleted ones left out.
    pub fn all(pages: &[Page]) -> Vec<Page> {
        let mut out: Vec<Page> = pages
            .iter()
            .filter(|p| p.deleted_at.is_none())
            .cloned()
            .collect();
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        out
    }

    /// The Prints a thread made, newest first.
    pub fn made_by(thread: &str, pages: &[Page]) -> Vec<Page> {
        all(pages)
            .into_iter()
            .filter(|p| p.owner_thread == thread)
            .collect()
    }

    /// The Prints in a project, newest first.
    pub fn in_project(project: &str, pages: &[Page]) -> Vec<Page> {
        all(pages)
            .into_iter()
            .filter(|p| p.is_in(project))
            .collect()
    }

    /// "Threadbear, Poddle", or "No project". A project the app doesn't
    /// know (archived, or not loaded yet) is left out.
    pub fn projects(page: &Page, titles: &HashMap<String, String>) -> String {
        let names: Vec<&str> = page
            .projects
            .iter()
            .filter_map(|p| titles.get(&p.id).map(String::as_str))
            .collect();
        if names.is_empty() {
            "No project".to_owned()
        } else {
            names.join(", ")
        }
    }

    /// "Chart · v3", for a row's right-hand side.
    pub fn kind_and_version(page: &Page) -> String {
        format!("{} · v{}", page.kind_label(), page.version)
    }
}

/// The Print id a reply's fence names: ```print <id> (```page <id> before
/// the rename), or the id alone in the fence. None for anything else.
pub fn fence_id(language: Option<&str>, code: &str) -> Option<String> {
    let mut words = language
        .unwrap_or("")
        .split([' ', '\t'])
        .filter(|w| !w.is_empty());
    let word = words.next()?.to_lowercase();
    if word != "print" && word != "page" {
        return None;
    }
    let candidate = match words.next() {
        Some(w) => w.to_owned(),
        None => code.split_whitespace().next().unwrap_or("").to_owned(),
    };
    is_page_id(&candidate).then_some(candidate)
}

/// `pg_` and lowercase letters, digits and underscores, at most 80.
pub fn is_page_id(s: &str) -> bool {
    s.len() > 3
        && s.len() <= 80
        && s.starts_with("pg_")
        && s[3..]
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Where a Print's version lives on the runner's Mac:
/// `<home>/pages/<id>/v<n>/index.html`. A side pane showing one follows new
/// versions.
pub mod path {
    use super::Page;
    use std::path::Path;

    /// The Print id and version a file under a pages folder belongs to.
    pub fn parse(path: &str) -> Option<(String, u32)> {
        let parts: Vec<&str> = Path::new(path)
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();
        let n = parts.len();
        if n < 4 || parts[n - 1] != "index.html" || parts[n - 4] != "pages" {
            return None;
        }
        let v: u32 = parts[n - 2]
            .strip_prefix('v')?
            .parse()
            .ok()
            .filter(|v| *v > 0)?;
        Some((parts[n - 3].to_owned(), v))
    }

    /// The runner's Prints folder (`<home>/pages`) a Print's file is in.
    pub fn root(file: &str) -> Option<String> {
        parse(file)?;
        let root = Path::new(file).parent()?.parent()?.parent()?;
        Some(root.to_string_lossy().into_owned())
    }

    /// The same Print's file at another version.
    pub fn at_version(path: &str, version: u32) -> Option<String> {
        parse(path)?;
        let folder = Path::new(path).parent()?.parent()?;
        Some(
            folder
                .join(format!("v{version}"))
                .join("index.html")
                .to_string_lossy()
                .into_owned(),
        )
    }

    /// The same Print's new newest version, when the file shown was the
    /// newest in `before` and `after` has a newer one; None to leave it as
    /// it is. An older version opened on purpose stays put.
    pub fn newer(path: &str, before: &[Page], after: &[Page]) -> Option<String> {
        let (id, v) = parse(path)?;
        let page = after.iter().find(|p| p.id == id)?;
        if page.version <= v
            || page.deleted_at.is_some()
            || before.iter().find(|p| p.id == id)?.version != v
        {
            return None;
        }
        at_version(path, page.version)
    }
}

/// `path::newer`, by the name the side pane calls it.
pub fn newer_file(path: &str, before: &[Page], after: &[Page]) -> Option<String> {
    path::newer(path, before, after)
}

/// A click inside a todo list or board Print (runner/src/page-list.ts):
/// `threadbear-print:toggle?v=3&item=2` or
/// `threadbear-print:move?v=3&card=1&to=2`, with the version the page was
/// drawn from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrintLink {
    Toggle { version: u32, item: u32 },
    Move { version: u32, card: u32, to: u32 },
}

impl PrintLink {
    pub const SCHEME: &'static str = "threadbear-print";

    pub fn parse(url: &str) -> Option<PrintLink> {
        let rest = url.strip_prefix("threadbear-print:")?;
        let (what, query) = rest.split_once('?').unwrap_or((rest, ""));
        let number = |name: &str| -> Option<u32> {
            query
                .split('&')
                .filter_map(|kv| kv.split_once('='))
                .find(|(k, _)| *k == name)
                .and_then(|(_, v)| v.parse().ok())
        };
        let version = number("v").filter(|v| *v > 0)?;
        match what {
            "toggle" => Some(PrintLink::Toggle {
                version,
                item: number("item")?,
            }),
            "move" => Some(PrintLink::Move {
                version,
                card: number("card")?,
                to: number("to")?,
            }),
            _ => None,
        }
    }

    pub fn version(&self) -> u32 {
        match *self {
            PrintLink::Toggle { version, .. } | PrintLink::Move { version, .. } => version,
        }
    }

    /// The runner call it makes, and its params.
    pub fn call(&self, page_id: &str) -> (&'static str, Value) {
        match *self {
            PrintLink::Toggle { version, item } => (
                "pages.toggleItem",
                serde_json::json!({ "pageId": page_id, "version": version, "item": item }),
            ),
            PrintLink::Move { version, card, to } => (
                "pages.moveCard",
                serde_json::json!({ "pageId": page_id, "version": version, "card": card, "to": to }),
            ),
        }
    }
}

/// The menu's words, which the scripted app and the specs find items by.
pub mod words {
    pub const SECTION_EMPTY: &str = "None yet. Ask a thread in this project for a chart, a dashboard, a report or a doc, and it shows here.";
    pub const RAIL_EMPTY: &str = "No Prints yet. Ask a thread for a chart, a report or a doc.";
    pub const ASK_ABOUT: &str = "Say what should change. The thread that made it does the work and saves a new version; the old one is kept.";
    pub const ASK_PLACEHOLDER: &str =
        "Make the region chart a pie, and add a table of the top 10 rows";
    pub const DOC_HINT: &str =
        "Markdown: # headings, - lists, - [ ] tasks, | tables |, **bold**, [links](https://…)";
    pub const DIAGRAM_HINT: &str =
        "Mermaid source. Save redraws the diagram and keeps the previous version.";
    pub const NO_SOURCE: &str = "This print has no editable source. Ask its thread to rebuild it.";
    pub const PUBLISH_ABOUT: &str = "It goes to your Convex backend, so it opens on your phone and the website, and anyone with its link can open it. New versions go up by themselves. Unpublish takes it down.";
    pub const WHEN_ASKED_ABOUT: &str = "Threads can still make Prints, but only when you ask for one. If a thread goes to make one your message didn't ask for, like a progress Print on a long task, it asks you first, and No means none is made. Saying print, chart, graph, dashboard or diagram counts as asking.";
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn page(v: Value) -> Page {
        Page::decode(&v).unwrap()
    }

    #[test]
    fn decodes_with_defaults() {
        let p = page(json!({ "id": "pg_a", "title": "A" }));
        assert_eq!(p.kind, "page");
        assert_eq!(p.version, 1);
        assert_eq!(p.kind_label(), "Print");
        assert!(!p.has_editable_source());
        let p = page(
            json!({ "id": "pg_b", "title": "B", "kind": "todo", "version": 12, "editedBy": "user", "published": { "slug": "abc", "version": 3, "at": "x" }, "projects": [{ "id": "p1", "by": "mike", "pinned": true }, { "nope": 1 }], "updatedAt": "2026-11-14T10:00:00.000Z" }),
        );
        assert_eq!(p.kind_label(), "Todo list");
        assert!(p.is_list() && p.has_editable_source());
        assert!(p.edited_by_user);
        assert_eq!(p.published_slug.as_deref(), Some("abc"));
        assert_eq!(
            p.projects,
            vec![ProjectRef {
                id: "p1".into(),
                pinned: true
            }]
        );
        assert_eq!(p.kept_versions(), (3..=12).rev().collect::<Vec<_>>());
        assert_eq!(
            page(json!({ "id": "pg_c", "title": "C", "kind": "dashboard" })).kind_label(),
            "Dashboard"
        );
        assert!(Page::decode(&json!({ "id": "pg_x" })).is_none());
    }

    #[test]
    fn rows_and_words() {
        let now = parse_ms("2026-11-14T12:00:00.000Z").unwrap();
        let a = page(
            json!({ "id": "pg_a", "title": "Launch notes", "ownerThread": "t1", "kind": "doc", "version": 3, "projects": [{ "id": "p1" }], "updatedAt": "2026-11-14T10:00:00.000Z" }),
        );
        let b = page(
            json!({ "id": "pg_b", "title": "Bundle", "ownerThread": "t2", "kind": "chart", "updatedAt": "2026-11-14T11:59:30.000Z", "published": { "slug": "s" } }),
        );
        let gone = page(
            json!({ "id": "pg_c", "title": "Gone", "ownerThread": "t1", "deletedAt": "2026-11-14T11:00:00.000Z", "updatedAt": "2026-11-14T11:30:00.000Z" }),
        );
        let all = rows::all(&[a.clone(), b.clone(), gone]);
        assert_eq!(
            all.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["pg_b", "pg_a"]
        );
        assert_eq!(rows::made_by("t1", &all).len(), 1);
        assert_eq!(rows::in_project("p1", &all).len(), 1);
        let titles = HashMap::from([("p1".to_owned(), "Demo Shop".to_owned())]);
        assert_eq!(rows::projects(&a, &titles), "Demo Shop");
        assert_eq!(rows::projects(&b, &titles), "No project");
        assert_eq!(rows::kind_and_version(&a), "Doc · v3");
        assert_eq!(a.row_detail(now), "Doc · v3 · 2h");
        assert_eq!(b.card_meta(false, now), "Chart · v1 · just now · published");
        assert_eq!(a.card_meta(false, now), "Doc · v3 · 2h ago");
        assert_eq!(
            a.card_meta(true, now),
            "Doc · v3 · refreshing in its thread…"
        );
        assert!(!a.is_new_version(false, now));
        assert!(page(json!({ "id": "pg_d", "title": "D", "version": 2, "updatedAt": "2026-11-14T11:50:00.000Z" })).is_new_version(false, now));
        assert_eq!(
            page(json!({ "id": "pg_e", "title": "Q3: a/b" })).file_name(),
            "Q3- a-b"
        );
    }

    #[test]
    fn fences() {
        assert_eq!(
            fence_id(Some("print pg_bundle_size"), "").as_deref(),
            Some("pg_bundle_size")
        );
        assert_eq!(fence_id(Some("page"), "pg_x1\n").as_deref(), Some("pg_x1"));
        assert_eq!(
            fence_id(Some("Print"), "  pg_x1  ").as_deref(),
            Some("pg_x1")
        );
        assert_eq!(fence_id(Some("print"), "PG_X").as_deref(), None);
        assert_eq!(fence_id(Some("html"), "pg_x").as_deref(), None);
        assert_eq!(fence_id(None, "pg_x").as_deref(), None);
        assert!(!is_page_id("pg_"));
        assert!(!is_page_id("pg_a-b"));
    }

    #[test]
    fn paths_follow_new_versions() {
        let file = "/Users/me/Library/Application Support/Threadbear/pages/pg_a/v2/index.html";
        assert_eq!(path::parse(file), Some(("pg_a".into(), 2)));
        assert_eq!(
            path::root(file).as_deref(),
            Some("/Users/me/Library/Application Support/Threadbear/pages")
        );
        assert_eq!(path::parse("/x/pg_a/v2/index.html"), None);
        assert_eq!(path::parse("/x/pages/pg_a/v0/index.html"), None);
        let v = |n: u32| page(json!({ "id": "pg_a", "title": "A", "version": n }));
        // Compare filesystem paths: Windows joins with backslashes even for slash input.
        let newer = path::newer(file, &[v(2)], &[v(3)]).unwrap();
        assert_eq!(
            std::path::Path::new(&newer),
            std::path::Path::new(
                "/Users/me/Library/Application Support/Threadbear/pages/pg_a/v3/index.html"
            )
        );
        // An older version opened on purpose stays.
        assert_eq!(path::newer(file, &[v(4)], &[v(5)]), None);
        assert_eq!(path::newer(file, &[v(2)], &[v(2)]), None);
    }

    #[test]
    fn links() {
        assert_eq!(
            PrintLink::parse("threadbear-print:toggle?v=3&item=2"),
            Some(PrintLink::Toggle {
                version: 3,
                item: 2
            })
        );
        assert_eq!(
            PrintLink::parse("threadbear-print:move?v=3&card=1&to=0"),
            Some(PrintLink::Move {
                version: 3,
                card: 1,
                to: 0
            })
        );
        assert_eq!(PrintLink::parse("threadbear-print:toggle?v=0&item=2"), None);
        assert_eq!(
            PrintLink::parse("threadbear-print:toggle?v=1&item=-1"),
            None
        );
        assert_eq!(PrintLink::parse("https://x/toggle?v=1&item=1"), None);
        assert_eq!(PrintLink::parse("threadbear-print:move?v=3&card=1"), None);
        let (method, params) = PrintLink::Toggle {
            version: 3,
            item: 2,
        }
        .call("pg_a");
        assert_eq!(method, "pages.toggleItem");
        assert_eq!(params, json!({ "pageId": "pg_a", "version": 3, "item": 2 }));
    }
}
