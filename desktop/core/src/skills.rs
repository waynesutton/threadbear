//! The skills store (the skillsStore experiment), a port of the Swift app's
//! Skills.swift and SkillsPage's helpers: the runner's `skills.list` and
//! `skills.updated` read into rows, each row's source line and GitHub page,
//! and the slash name a New Skill form saves under (docs/protocol.md,
//! "Skills"). The page that draws them is desktop/gpui/src/skills_page.rs.
use serde_json::Value;

/// The experiment that turns the store on (runner/src/experiments/skillsStore.md).
pub const EXPERIMENT: &str = "skillsStore";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    GitHub { repo: String, path: Option<String> },
    Builtin,
    /// Written in the app (New Skill): its SKILL.md is his, and Edit changes it.
    Custom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub always: bool,
    pub source: Source,
    pub commit: String,
    pub dir: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggested {
    pub id: String,
    pub name: String,
    pub description: String,
}

impl Skill {
    pub fn decode(v: &Value) -> Option<Skill> {
        let name = v["name"].as_str().filter(|n| !n.is_empty())?.to_owned();
        let text = |k: &str| v[k].as_str().unwrap_or("").to_owned();
        let source = match v["source"]["kind"].as_str() {
            Some("github") => Source::GitHub {
                repo: v["source"]["repo"].as_str().unwrap_or("").to_owned(),
                path: v["source"]["path"].as_str().filter(|p| !p.is_empty()).map(str::to_owned),
            },
            Some("custom") => Source::Custom,
            _ => Source::Builtin,
        };
        Some(Skill { name, description: text("description"), always: v["always"].as_bool() != Some(false), source, commit: text("commit"), dir: text("dir") })
    }

    pub fn is_custom(&self) -> bool {
        self.source == Source::Custom
    }

    /// Under the description: where it came from, and the commit it's pinned to.
    pub fn source_line(&self) -> String {
        match &self.source {
            Source::Builtin => "Comes with Threadbear".into(),
            Source::Custom => "Written in Threadbear".into(),
            Source::GitHub { repo, path } => {
                let place = [Some(repo.as_str()), path.as_deref()].into_iter().flatten().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("/");
                let at = if self.commit.chars().count() >= 7 { format!(" at {}", &self.commit[..7]) } else { String::new() };
                format!("{}{at}", if place.is_empty() { "GitHub" } else { &place })
            }
        }
    }

    /// Its GitHub page at the pinned commit; none for a built-in or his own.
    pub fn source_url(&self) -> Option<String> {
        let Source::GitHub { repo, path } = &self.source else { return None };
        if repo.is_empty() {
            return None;
        }
        Some(match path {
            Some(p) => format!("https://github.com/{repo}/tree/{}/{p}", self.commit),
            None => format!("https://github.com/{repo}/tree/{}", self.commit),
        })
    }
}

impl Suggested {
    pub fn decode(v: &Value) -> Option<Suggested> {
        let id = v["id"].as_str().filter(|n| !n.is_empty())?.to_owned();
        Some(Suggested { name: v["name"].as_str().unwrap_or(&id).to_owned(), description: v["description"].as_str().unwrap_or("").to_owned(), id })
    }
}

/// `skills.list`'s and `skills.updated`'s `skills`.
pub fn decode_list(v: &Value) -> Vec<Skill> {
    v.as_array().map(|a| a.iter().filter_map(Skill::decode).collect()).unwrap_or_default()
}

pub fn decode_suggested(v: &Value) -> Vec<Suggested> {
    v.as_array().map(|a| a.iter().filter_map(Suggested::decode).collect()).unwrap_or_default()
}

/// What a typed name saves as (the runner's rule): lower case, letters,
/// digits and dashes, no leading slash.
pub fn slug(typed: &str) -> String {
    let lower = typed.trim().trim_start_matches('/').to_lowercase();
    let mut out = String::new();
    for c in lower.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_the_runners_list() {
        let list = decode_list(&json!([
            { "name": "pele", "description": "Diagrams", "always": true, "source": { "kind": "github", "repo": "mikecann/pele", "path": "skills/pele", "url": "x" }, "commit": "0123456789abcdef", "dir": "/d/pele" },
            { "name": "voice", "description": "", "always": false, "source": { "kind": "custom" }, "commit": "", "dir": "/d/voice" },
            { "name": "make-print", "source": { "kind": "builtin" } },
            { "description": "no name" },
        ]));
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].source_line(), "mikecann/pele/skills/pele at 0123456");
        assert_eq!(list[0].source_url().as_deref(), Some("https://github.com/mikecann/pele/tree/0123456789abcdef/skills/pele"));
        assert!(list[1].is_custom() && !list[1].always);
        assert_eq!(list[1].source_line(), "Written in Threadbear");
        assert_eq!(list[1].source_url(), None);
        assert_eq!(list[2].source_line(), "Comes with Threadbear");
        assert!(list[2].always);
    }

    #[test]
    fn a_typed_name_saves_as_the_runner_would() {
        assert_eq!(slug("/Brand Voice!"), "brand-voice");
        assert_eq!(slug("  release-notes "), "release-notes");
        assert_eq!(slug("!!!"), "");
    }

    #[test]
    fn suggestions_keep_their_id() {
        let s = decode_suggested(&json!([{ "id": "anatomy", "name": "anatomy", "description": "Figures" }, { "name": "no id" }]));
        assert_eq!(s, vec![Suggested { id: "anatomy".into(), name: "anatomy".into(), description: "Figures".into() }]);
    }
}
