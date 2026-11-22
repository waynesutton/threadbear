// Slash skill names in ordinary words, matching the Swift SkillMentions rules.
use crate::slash::SlashCommand;
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mention {
    pub range: Range<usize>,
    pub name: String,
    pub path: Option<String>,
}

pub fn find(text: &str, commands: &[SlashCommand]) -> Vec<Mention> {
    let known: std::collections::HashMap<String, &SlashCommand> = commands
        .iter()
        .filter(|c| c.is_skill())
        .map(|c| (c.name.to_ascii_lowercase(), c))
        .collect();
    if known.is_empty() {
        return Vec::new();
    }
    let blocked = |c: char| c.is_alphanumeric() || c == '_' || "/:-".contains(c);
    let mut found = Vec::new();
    for (start, c) in text.char_indices() {
        if c != '/'
            || text[..start]
                .chars()
                .next_back()
                .is_some_and(|c| blocked(c) || ".~".contains(c))
        {
            continue;
        }
        let bytes = text.as_bytes();
        let mut end = start + 1;
        let part = |at: &mut usize| {
            let first = *at;
            if bytes.get(*at).is_none_or(|c| !c.is_ascii_alphanumeric()) {
                return false;
            }
            *at += 1;
            while bytes
                .get(*at)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'-')
            {
                *at += 1;
            }
            *at > first
        };
        if !part(&mut end) {
            continue;
        }
        if bytes.get(end) == Some(&b':') {
            end += 1;
            if !part(&mut end) {
                continue;
            }
        }
        if text[end..].chars().next().is_some_and(blocked) {
            continue;
        }
        let word = text[start + 1..end].to_ascii_lowercase();
        let name = if known.contains_key(&word) {
            Some(word.as_str())
        } else {
            word.strip_prefix("threadbear:")
                .filter(|n| known.contains_key(*n))
        };
        if let Some(name) = name {
            found.push(Mention {
                range: start..end,
                name: name.to_owned(),
                path: known[name].path.clone(),
            });
        }
    }
    found
}

/// Make only the known mentions into links, keeping the rest ordinary text.
/// Escaping all punctuation prevents message text from becoming Markdown or HTML.
pub fn linked_text(text: &str, mentions: &[Mention]) -> String {
    fn words(text: &str, into: &mut String) {
        for c in text.chars() {
            if c == '\n' {
                into.push_str("  \n");
            } else {
                if c.is_ascii_punctuation() {
                    into.push('\\');
                }
                into.push(c);
            }
        }
    }
    let mut out = String::new();
    let mut from = 0;
    for mention in mentions {
        words(&text[from..mention.range.start], &mut out);
        out.push('[');
        words(&text[mention.range.clone()], &mut out);
        out.push_str("](threadbear-skill:");
        out.push_str(&mention.name);
        out.push(')');
        from = mention.range.end;
    }
    words(&text[from..], &mut out);
    out
}

/// Keep the user's own Markdown while linking known skill words.
pub fn linked_markdown_text(text: &str, mentions: &[Mention]) -> String {
    let mut out = String::new();
    let mut from = 0;
    for mention in mentions {
        out.push_str(&text[from..mention.range.start]);
        out.push('[');
        out.push_str(&text[mention.range.clone()]);
        out.push_str("](threadbear-skill:");
        out.push_str(&mention.name);
        out.push(')');
        from = mention.range.end;
    }
    out.push_str(&text[from..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn commands() -> Vec<SlashCommand> {
        vec![
            SlashCommand {
                name: "review".into(),
                source: "skill".into(),
                path: Some("/skills/review/SKILL.md".into()),
                ..Default::default()
            },
            SlashCommand {
                name: "tool:check".into(),
                source: "skill".into(),
                ..Default::default()
            },
            SlashCommand {
                name: "compact".into(),
                source: "cli".into(),
                ..Default::default()
            },
        ]
    }
    #[test]
    fn whole_skill_words_include_plugin_names_and_store_aliases() {
        let text = "Start /review, then (/tool:check) and /threadbear:review.";
        let got = find(text, &commands());
        assert_eq!(
            got.iter()
                .map(|m| &text[m.range.clone()])
                .collect::<Vec<_>>(),
            ["/review", "/tool:check", "/threadbear:review"]
        );
        assert_eq!(got[2].name, "review");
        assert_eq!(got[0].path.as_deref(), Some("/skills/review/SKILL.md"));
    }
    #[test]
    fn paths_longer_words_unknown_names_and_cli_commands_stay_words() {
        let text = "/reviewing src/review ./review ~/review https://x/review /review/path /review:unknown /reviewé /review_abc /compact /missing";
        assert!(find(text, &commands()).is_empty());
    }
    #[test]
    fn links_preserve_literal_markup_and_only_use_known_names() {
        let text = "**use** /review <script>";
        let linked = linked_text(text, &find(text, &commands()));
        assert!(linked.contains("threadbear-skill:review"));
        assert!(!linked.contains("<script>"));
        assert!(linked.contains(r"\*\*use\*\*"));
    }
    #[test]
    fn links_can_keep_the_users_own_markdown() {
        let text = "**use** /review\nnext";
        assert_eq!(
            linked_markdown_text(text, &find(text, &commands())),
            "**use** [/review](threadbear-skill:review)\nnext"
        );
    }
    #[test]
    fn names_are_case_insensitive_with_utf8_byte_ranges() {
        let got = find("😀 /REVIEW", &commands());
        assert_eq!(got[0].range, 5..12);
        assert_eq!(got[0].name, "review");
    }
}
