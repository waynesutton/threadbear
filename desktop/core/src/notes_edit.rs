//! Editing plain Markdown in private notes.
use std::ops::Range;

#[derive(Debug, PartialEq)]
pub enum Newline { Plain, Continue(String), End(Range<usize>) }

/// The replacement for Return at a byte caret. Empty list items end the
/// list; other list items carry their marker forward, incrementing numbers.
pub fn newline(text: &str, caret: usize) -> Newline {
    if !text.is_char_boundary(caret) { return Newline::Plain; }
    fn in_code(node: &markdown::mdast::Node, caret: usize) -> bool {
        if matches!(node, markdown::mdast::Node::Code(_)) && node.position().is_some_and(|p|p.start.offset<=caret && caret<=p.end.offset) {return true}
        node.children().is_some_and(|nodes|nodes.iter().any(|n|in_code(n,caret)))
    }
    if markdown::to_mdast(text,&markdown::ParseOptions::gfm()).is_ok_and(|root|in_code(&root,caret)) {return Newline::Plain}
    let start = text[..caret].rfind('\n').map_or(0, |i|i+1);
    let end = text[caret..].find('\n').map_or(text.len(),|i|caret+i);
    let before = &text[start..caret];
    let indent_len = before.len()-before.trim_start_matches([' ', '\t']).len();
    let (indent,line) = before.split_at(indent_len);
    let Some(space) = line.find([' ', '\t']) else {return Newline::Plain};
    let bullet = &line[..space];
    let next = if matches!(bullet,"-"|"*"|"+") { bullet.to_owned() }
        else if bullet.len()>=2 && bullet.len()<=10 && matches!(bullet.as_bytes().last(),Some(b'.'|b')')) && bullet[..bullet.len()-1].bytes().all(|b|b.is_ascii_digit()) {
            let n: u32 = bullet[..bullet.len()-1].parse().unwrap_or(0);
            format!("{}{}",n+1,&bullet[bullet.len()-1..])
        } else {return Newline::Plain};
    let mut rest = &line[space+1..];
    let task = ["[ ]", "[x]", "[X]"].iter().any(|p|rest.starts_with(p) && (rest.len()==3 || rest.as_bytes().get(3).is_some_and(u8::is_ascii_whitespace)));
    if task { rest = rest[3..].trim_start_matches([' ', '\t']); }
    if rest.trim().is_empty() {
        if text[caret..end].trim().is_empty() { Newline::End(start..caret) } else {Newline::Plain}
    } else {Newline::Continue(format!("\n{indent}{next} {}",if task {"[ ] "}else{""}))}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lists_continue_or_end_at_an_empty_item() {
        assert_eq!(newline("- milk",6),Newline::Continue("\n- ".into()));
        assert_eq!(newline("  9) milk",9),Newline::Continue("\n  10) ".into()));
        assert_eq!(newline("- [x] milk",10),Newline::Continue("\n- [ ] ".into()));
        assert_eq!(newline("hello\n- [ ] ",12),Newline::End(6..12));
        assert_eq!(newline("- milk",2),Newline::Plain);
        assert_eq!(newline("ordinary",8),Newline::Plain);
        assert_eq!(newline("é",1),Newline::Plain);
        assert_eq!(newline("```md\n- milk\n```",11),Newline::Plain);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind { Heading(u8), Task(bool), Done, Bold, Italic, Code, CodeBlock, Link, Quote, Mark }
#[derive(Clone, Debug)]
pub struct Span { pub range: Range<usize>, pub kind: Kind }

/// Parse Markdown rather than treating text inside a fence as a task or
/// heading. Offsets are UTF-8 bytes, as the editor uses, even after emoji.
pub fn spans(text: &str) -> Vec<Span> {
    fn visit(node: &markdown::mdast::Node, text: &str, out: &mut Vec<Span>) {
        use markdown::mdast::Node;
        if let Some(pos) = node.position() {
            let range = pos.start.offset..pos.end.offset;
            match node {
                Node::Heading(h) if text.get(range.clone()).is_some_and(|s|s.starts_with('#')) => {
                    let marker = range.start..range.start+usize::from(h.depth);
                    out.push(Span {range,kind:Kind::Heading(h.depth)});
                    out.push(Span {range:marker,kind:Kind::Mark});
                    return;
                }
                Node::Strong(_) => out.push(Span {range,kind:Kind::Bold}),
                Node::Emphasis(_) => out.push(Span {range,kind:Kind::Italic}),
                Node::InlineCode(_) => out.push(Span {range,kind:Kind::Code}),
                Node::Code(_) => out.push(Span {range,kind:Kind::CodeBlock}),
                Node::Link(_) => out.push(Span {range,kind:Kind::Link}),
                Node::Blockquote(_) => out.push(Span {range,kind:Kind::Quote}),
                Node::ListItem(item) => {
                    if let Some(line) = text.get(range.clone()).and_then(|s|s.lines().next()) {
                        let marker_end = line.find([' ', '\t']).unwrap_or(0);
                        if marker_end>0 {out.push(Span {range:range.start..range.start+marker_end,kind:Kind::Mark});}
                        if let Some(at) = line.find('[').filter(|_|item.checked.is_some()) {
                            let start = range.start+at;
                            let checked = item.checked == Some(true);
                            out.push(Span {range:start..start+3,kind:Kind::Task(checked)});
                            if checked { out.push(Span {range:start+3..range.start+line.len(),kind:Kind::Done}); }
                        }
                    }
                }
                _ => {}
            }
        }
        if let Some(children) = node.children() { for child in children { visit(child,text,out); } }
    }
    let mut out=Vec::new();
    if let Ok(root)=markdown::to_mdast(text,&markdown::ParseOptions::gfm()) {visit(&root,text,&mut out);}
    out
}

/// The one byte to replace when clicking any of the task marker's three
/// characters. The document stays plain Markdown and the editor owns undo.
pub fn toggle(text: &str, at: usize) -> Option<(Range<usize>, &'static str)> {
    spans(text).into_iter().find_map(|s| match s.kind {
        Kind::Task(done) if s.range.contains(&at) => Some((s.range.start+1..s.range.start+2,if done {" "}else{"x"})),
        _=>None
    })
}

#[cfg(test)]
mod markdown_tests {
    use super::*;
    #[test]
    fn tasks_are_byte_ranges_and_code_blocks_are_left_alone() {
        let text="🐻\n# Plan\n- [ ] milk\n```md\n# code\n- [ ] code\n```\n- [x] done";
        let runs=spans(text);
        assert_eq!(runs.iter().filter(|s|matches!(s.kind,Kind::Heading(_))).count(),1);
        assert_eq!(runs.iter().filter(|s|matches!(s.kind,Kind::Task(_))).count(),2);
        let at=text.find("[ ]").unwrap();
        assert_eq!(toggle(text,at+2),Some((at+1..at+2,"x")));
        assert_eq!(toggle(text,text.find("[ ] code").unwrap()),None);
        assert!(runs.iter().any(|s|s.kind==Kind::Done && text[s.range.clone()].contains("done")));
    }
}
