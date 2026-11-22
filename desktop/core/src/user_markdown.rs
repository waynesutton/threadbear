//! Basic Markdown in the user's own messages, preserving chat line breaks.
use markdown::mdast::Node;

pub fn prepare(text: &str) -> Option<String> {
    let tree = markdown::to_mdast(text, &markdown::ParseOptions::gfm()).ok()?;
    if !has_formatting(&tree) {
        return None;
    }
    let lines: Vec<&str> = text.split('\n').collect();
    let (mut code, mut list) = (false, false);
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if fence(trimmed) {
            code = !code;
            out.push((*line).to_owned());
            continue;
        }
        if !code {
            let item = list_item(trimmed);
            let indented = line.starts_with([' ', '\t']);
            if list && !item && !trimmed.is_empty() && !indented {
                out.push(String::new());
            }
            list = item || (list && indented);
            if trimmed.is_empty() {
                list = false;
            }
        }
        let next = lines.get(i + 1).map(|s| s.trim()).unwrap_or_default();
        if !code && !trimmed.is_empty() && !next.is_empty() && !starts_block(next) {
            out.push(format!("{line}  "));
        } else {
            out.push((*line).to_owned());
        }
    }
    Some(out.join("\n"))
}
fn has_formatting(node: &Node) -> bool {
    matches!(
        node,
        Node::Strong(_)
            | Node::Emphasis(_)
            | Node::InlineCode(_)
            | Node::Code(_)
            | Node::Link(_)
            | Node::Heading(_)
            | Node::Blockquote(_)
            | Node::List(_)
            | Node::Delete(_)
    ) || node
        .children()
        .is_some_and(|children| children.iter().any(has_formatting))
}
fn fence(line: &str) -> bool {
    line.starts_with("```") || line.starts_with("~~~")
}
fn list_item(line: &str) -> bool {
    let bytes = line.as_bytes();
    if bytes.len() > 1 && matches!(bytes[0], b'-' | b'*' | b'+') && bytes[1].is_ascii_whitespace() {
        return true;
    }
    let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
    (1..=9).contains(&digits)
        && bytes.get(digits).is_some_and(|b| matches!(b, b'.' | b')'))
        && bytes.get(digits + 1).is_some_and(u8::is_ascii_whitespace)
}
fn starts_block(line: &str) -> bool {
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    list_item(line)
        || line.starts_with('>')
        || fence(line)
        || ((1..=6).contains(&hashes)
            && line
                .as_bytes()
                .get(hashes)
                .is_some_and(u8::is_ascii_whitespace))
}

#[cfg(test)]
mod tests {
    use super::prepare;
    #[test]
    fn plain_messages_stay_plain() {
        for text in [
            "Can you fix the login page?\nIt breaks on Safari.",
            "snake_case_names and 2*3 are fine",
            "#123 is still broken",
        ] {
            assert_eq!(prepare(text), None);
        }
    }
    #[test]
    fn lists_and_following_chat_lines() {
        assert_eq!(
            prepare("Todo:\n- milk\n- eggs").as_deref(),
            Some("Todo:\n- milk\n- eggs")
        );
        assert_eq!(
            prepare("- one\n- two\nthen this\nand this").as_deref(),
            Some("- one\n- two  \n\nthen this  \nand this")
        );
    }
    #[test]
    fn line_breaks_and_code() {
        assert_eq!(
            prepare("**Note**\nline two\nline three").as_deref(),
            Some("**Note**  \nline two  \nline three")
        );
        let code = "Run this:\n```\nnpm test\nnpm run build\n```";
        assert_eq!(prepare(code).as_deref(), Some(code));
    }
    #[test]
    fn inline_formatting_links_and_quotes() {
        for text in [
            "use `tb land` now",
            "see [the docs](https://example.com)",
            "this is *important*",
            "# Heading",
            "> quoted",
            "~~old~~",
        ] {
            assert!(prepare(text).is_some(), "{text}");
        }
    }
}
