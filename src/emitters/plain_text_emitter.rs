//! `@microsoft/tsdoc`'s PlainTextEmitter: whether a tree has any text.

use crate::nodes::NodeRef;

/// Whether `nodes` have at least `required_characters` (at least 1) characters of text besides
/// spaces, tabs and line breaks, in plain text, code, escapes and link text.
pub fn has_any_text_content<'a>(nodes: impl IntoIterator<Item = NodeRef<'a>>, required_characters: usize) -> bool {
    let required = required_characters.max(1);
    scan_text_content(nodes, required, 0) >= required
}

// TSDoc adds the children's running count to the count it already has, so a node's text can count
// twice. Only whether the count reaches `required` matters, and it reaches it here exactly when it
// does there.
fn scan_text_content<'a>(nodes: impl IntoIterator<Item = NodeRef<'a>>, required: usize, mut found: usize) -> usize {
    for node in nodes {
        found += match node {
            NodeRef::FencedCode(code) => count_non_space_characters(&code.code),
            NodeRef::CodeSpan(code) => count_non_space_characters(&code.code),
            NodeRef::EscapedText(text) => count_non_space_characters(&text.decoded_text),
            NodeRef::LinkTag(link) => count_non_space_characters(link.link_text.as_deref().unwrap_or_default()),
            NodeRef::PlainText(text) => count_non_space_characters(&text.text),
            _ => 0,
        };
        if found >= required {
            break;
        }
        found += scan_text_content(node.child_nodes(), required, found);
        if found >= required {
            break;
        }
    }
    found
}

fn count_non_space_characters(s: &str) -> usize {
    s.encode_utf16().filter(|&unit| !matches!(unit, 32 | 9 | 13 | 10)).count()
}
