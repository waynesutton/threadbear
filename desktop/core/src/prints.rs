// Prints (docs/protocol.md, "Prints"): the pages experiment, called Prints
// since 10 Nov 2026 while the protocol keeps "page". A port of the Swift
// app's Page.swift (Page, PageFence, PagePath) and the words on its reply
// card (PageCardCell.meta, isNewVersion), so the GPUI card says the same.
// Decoded as forgivingly as the other models: a missing field takes its
// default, and an unknown one is ignored.
use chrono::{DateTime, TimeZone};
use serde_json::Value;

/// The experiment's id, as in hello's `experiments` (`Page.experiment`).
pub const EXPERIMENT: &str = "pages";

#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub owner_thread: String,
    /// The projects it's in, by id.
    pub projects: Vec<String>,
    pub version: i64,
    /// chart, dashboard, report, doc, diagram, bundle, todo, board or page.
    pub kind: String,
    /// The newest version was written in the app's editor.
    pub edited_by_user: bool,
    pub published: bool,
    pub deleted: bool,
    /// Milliseconds since 1970, or 0 when it didn't say.
    pub updated_ms: i64,
}

impl Page {
    pub fn decode(v: &Value) -> Option<Page> {
        let id = v["id"].as_str()?.to_owned();
        let title = v["title"].as_str()?.to_owned();
        Some(Page {
            id,
            title,
            summary: v["summary"].as_str().unwrap_or("").to_owned(),
            owner_thread: v["ownerThread"].as_str().unwrap_or("").to_owned(),
            projects: v["projects"].as_array().into_iter().flatten().filter_map(|p| p["id"].as_str().map(str::to_owned)).collect(),
            version: v["version"].as_i64().filter(|n| *n >= 1).unwrap_or(1),
            kind: v["kind"].as_str().filter(|k| !k.is_empty()).unwrap_or("page").to_owned(),
            edited_by_user: v["editedBy"].as_str() == Some("user"),
            published: v["published"]["slug"].is_string(),
            deleted: v["deletedAt"].is_string(),
            updated_ms: v["updatedAt"].as_str().and_then(crate::inbox::parse_ms).unwrap_or(0),
        })
    }

    /// Every page in a `pages.list` result or a `pages.updated` note.
    pub fn list(v: &Value) -> Vec<Page> {
        v["pages"].as_array().into_iter().flatten().filter_map(Page::decode).collect()
    }

    /// What its kind is called: a plain one is a Print (`kindLabel`).
    pub fn kind_label(&self) -> String {
        match self.kind.as_str() {
            "page" => "Print".into(),
            "todo" => "Todo list".into(),
            kind => {
                let mut chars = kind.chars();
                chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
            }
        }
    }

    /// The SF Symbol the Swift app gives its kind (`symbol`).
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
}

/// The page id in a ```print fence (```page, its name before the rename, still
/// works): the id in the info string, or alone in its body (`PageFence.id`).
pub fn fence_id(language: Option<&str>, code: &str) -> Option<String> {
    let mut words = language.unwrap_or("").split([' ', '\t']).filter(|w| !w.is_empty());
    let word = words.next()?.to_lowercase();
    if word != "print" && word != "page" {
        return None;
    }
    let candidate = match words.next() {
        Some(id) => id.to_owned(),
        None => code.split_whitespace().next().unwrap_or("").to_owned(),
    };
    is_page_id(&candidate).then_some(candidate)
}

/// `pg_` and then lowercase letters, digits and underscores, at most 80 in all.
pub fn is_page_id(s: &str) -> bool {
    s.len() > 3 && s.len() <= 80 && s.starts_with("pg_") && s[3..].chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Whether a reply may hold a print fence, before it's parsed.
pub fn may_hold_fence(text: &str) -> bool {
    text.contains("```print") || text.contains("```page") || text.contains("~~~print") || text.contains("~~~page")
}

/// The Print id and version a file under the runner's pages folder is:
/// `<home>/pages/<id>/v<n>/index.html` (`PagePath.parse`).
pub fn path_version(path: &str) -> Option<(String, i64)> {
    let parts: Vec<&str> = path.split(['/', '\\']).filter(|p| !p.is_empty()).collect();
    let n = parts.len();
    if n < 4 || parts[n - 1] != "index.html" || parts[n - 4] != "pages" {
        return None;
    }
    let version = parts[n - 2].strip_prefix('v')?.parse::<i64>().ok().filter(|v| *v > 0)?;
    Some((parts[n - 3].to_owned(), version))
}

/// The same Print's new newest version's file, when `path` was the newest in
/// `before` and `after` has a newer one; None to leave the pane as it is. An
/// older version opened on purpose stays put (`PagePath.newer`).
pub fn newer_file(path: &str, before: &[Page], after: &[Page]) -> Option<String> {
    let (id, shown) = path_version(path)?;
    let page = after.iter().find(|p| p.id == id && !p.deleted)?;
    if page.version <= shown || before.iter().find(|p| p.id == id)?.version != shown {
        return None;
    }
    // Swap only the version folder, keeping the separators the path had.
    let file = path.strip_suffix("index.html")?;
    let folder = file.trim_end_matches(['/', '\\']);
    let sep = &file[folder.len()..];
    let parent = folder.strip_suffix(&format!("v{shown}"))?;
    Some(format!("{parent}v{}{sep}index.html", page.version))
}

/// The card's line under its title: "Chart · v3 · 2h ago", with
/// "published" when it is, or what's under way (`PageCardCell.meta`).
pub fn card_meta<Tz: TimeZone>(page: Option<&Page>, refreshing: bool, now: &DateTime<Tz>) -> String {
    let Some(page) = page else { return "Not on this Mac. It may be deleted, or on another Mac's runner.".into() };
    if refreshing {
        return format!("{} · v{} · refreshing in its thread…", page.kind_label(), page.version);
    }
    let age = if now.timestamp_millis() - page.updated_ms < 90_000 { "just now".to_owned() } else { format!("{} ago", crate::sidebar::relative_short(page.updated_ms, now)) };
    format!("{} · v{} · {age}{}", page.kind_label(), page.version, if page.published { " · published" } else { "" })
}

/// `card_meta` now, in this Mac's time zone.
pub fn card_meta_now(page: Option<&Page>, refreshing: bool) -> String {
    card_meta(page, refreshing, &chrono::Local::now())
}

/// Now, in milliseconds since 1970.
pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// A version newer than the first, made in the last 15 minutes, which the
/// card marks "New version" (`PageCardCell.isNewVersion`).
pub fn is_new_version(page: &Page, refreshing: bool, now_ms: i64) -> bool {
    !refreshing && page.version > 1 && now_ms - page.updated_ms < 15 * 60_000
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;

    fn page(id: &str, version: i64) -> Page {
        Page::decode(&json!({ "id": id, "title": "T", "version": version })).unwrap()
    }

    #[test]
    fn pages_read_leniently() {
        let p = Page::decode(&json!({
            "id": "pg_sales", "title": "Sales", "kind": "chart", "version": 3, "ownerThread": "th1", "editedBy": "user",
            "projects": [{ "id": "p1", "by": "mike" }, { "by": "agent" }], "published": { "slug": "abc", "at": "x", "version": 3 },
            "updatedAt": "2026-11-14T10:00:00.000Z", "extra": true,
        }))
        .unwrap();
        assert_eq!((p.version, p.kind.as_str(), p.projects.clone()), (3, "chart", vec!["p1".to_owned()]));
        assert!(p.edited_by_user && p.published && !p.deleted);
        let bare = Page::decode(&json!({ "id": "pg_x", "title": "X" })).unwrap();
        assert_eq!((bare.version, bare.kind_label()), (1, "Print".to_owned()), "a plain one is a Print");
        assert!(Page::decode(&json!({ "title": "no id" })).is_none());
        assert_eq!(page("pg_a", 1).kind_label(), "Print");
        assert_eq!(Page { kind: "todo".into(), ..bare.clone() }.kind_label(), "Todo list");
        assert_eq!(Page { kind: "diagram".into(), ..bare }.kind_label(), "Diagram");
    }

    #[test]
    fn fences_name_a_page_as_the_swift_app_reads_them() {
        assert_eq!(fence_id(Some("print pg_sales"), "").as_deref(), Some("pg_sales"));
        assert_eq!(fence_id(Some("Page"), "pg_old_name\n").as_deref(), Some("pg_old_name"), "the name from before the rename");
        assert_eq!(fence_id(Some("print"), "  pg_body  ").as_deref(), Some("pg_body"));
        assert_eq!(fence_id(Some("print Not_An_Id"), ""), None);
        assert_eq!(fence_id(Some("python"), "pg_x"), None);
        assert_eq!(fence_id(None, "pg_x"), None);
        assert!(!is_page_id("pg_") && !is_page_id("pg_A") && is_page_id("pg_a_1"));
    }

    #[test]
    fn the_pane_follows_a_newer_version_unless_an_older_one_was_picked() {
        let path = "/Users/w/Library/Application Support/Threadbear/pages/pg_sales/v2/index.html";
        assert_eq!(path_version(path), Some(("pg_sales".to_owned(), 2)));
        assert_eq!(path_version("/x/pages/pg_sales/v0/index.html"), None);
        assert_eq!(path_version("/x/other/pg_sales/v2/index.html"), None);
        assert_eq!(newer_file(path, &[page("pg_sales", 2)], &[page("pg_sales", 3)]).as_deref(), Some("/Users/w/Library/Application Support/Threadbear/pages/pg_sales/v3/index.html"));
        assert_eq!(newer_file(path, &[page("pg_sales", 4)], &[page("pg_sales", 5)]), None, "version 2 was opened on purpose");
        assert_eq!(newer_file(path, &[page("pg_sales", 2)], &[page("pg_sales", 2)]), None);
        assert_eq!(newer_file(r"C:\h\pages\pg_sales\v2\index.html", &[page("pg_sales", 2)], &[page("pg_sales", 3)]).as_deref(), Some(r"C:\h\pages\pg_sales\v3\index.html"));
    }

    #[test]
    fn the_card_says_kind_version_and_age() {
        let now = Utc.with_ymd_and_hms(2026, 11, 14, 12, 0, 0).unwrap();
        let mut p = page("pg_sales", 3);
        p.kind = "chart".into();
        p.updated_ms = now.timestamp_millis() - 2 * 3600 * 1000;
        assert_eq!(card_meta(Some(&p), false, &now), "Chart · v3 · 2h ago");
        p.published = true;
        assert_eq!(card_meta(Some(&p), false, &now), "Chart · v3 · 2h ago · published");
        assert_eq!(card_meta(Some(&p), true, &now), "Chart · v3 · refreshing in its thread…");
        p.updated_ms = now.timestamp_millis() - 30_000;
        assert!(card_meta(Some(&p), false, &now).contains("just now"));
        assert!(is_new_version(&p, false, now.timestamp_millis()));
        assert!(!is_new_version(&page("pg_sales", 1), false, now.timestamp_millis()), "the first version isn't new");
        assert!(card_meta(None, false, &now).starts_with("Not on this Mac"));
    }
}
