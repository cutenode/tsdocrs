//! Transforms of a tree: `@microsoft/tsdoc`'s DocNodeTransforms.

use std::borrow::Cow;

use crate::{
    nodes::{DocNode, DocParagraph, DocPlainText, DocSoftBreak},
    parser::is_whitespace,
};

/// A copy of `paragraph` with its spacing collapsed, as rendered text would show it: a run of
/// whitespace becomes one space, spaces at the paragraph's start and end go, and its plain text
/// and soft breaks are rebuilt by hand, one plain text a line.
pub fn trim_spaces_in_paragraph(paragraph: &DocParagraph) -> DocParagraph {
    DocParagraph { nodes: trimmed_nodes(paragraph).into_iter().map(Cow::into_owned).collect() }
}

/// The nodes [`trim_spaces_in_paragraph`] gives, borrowing the ones it keeps as they are, so the
/// emitter can render them without copying.
pub(crate) fn trimmed_nodes(paragraph: &DocParagraph) -> Vec<Cow<'_, DocNode>> {
    let mut transformed = Vec::new();
    // The text since the last node that isn't text, and whether there's any, even empty.
    let (mut accumulated, mut accumulating) = (String::new(), false);
    let mut pending_space = false;
    let mut finished_skipping_leading_spaces = false;
    for node in &paragraph.nodes {
        match node {
            DocNode::PlainText(text) => {
                let mut words = text.text.split(is_whitespace).filter(|word| !word.is_empty()).peekable();
                if text.text.starts_with(is_whitespace) && finished_skipping_leading_spaces {
                    pending_space = true;
                }
                if words.peek().is_some() {
                    if pending_space {
                        accumulated.push(' ');
                        pending_space = false;
                    }
                    for (i, word) in words.enumerate() {
                        if i > 0 {
                            accumulated.push(' ');
                        }
                        accumulated += word;
                    }
                    accumulating = true;
                    finished_skipping_leading_spaces = true;
                }
                if text.text.ends_with(is_whitespace) && finished_skipping_leading_spaces {
                    pending_space = true;
                }
            }
            DocNode::SoftBreak(_) => {
                accumulated.push('\n');
                accumulating = true;
            }
            node => {
                if pending_space {
                    accumulated.push(' ');
                    accumulating = true;
                    pending_space = false;
                }
                if accumulating {
                    push_accumulated_text(&std::mem::take(&mut accumulated), true, &mut transformed);
                    accumulating = false;
                }
                transformed.push(Cow::Borrowed(node));
                finished_skipping_leading_spaces = true;
            }
        }
    }
    if accumulating {
        push_accumulated_text(&accumulated, false, &mut transformed);
    }
    transformed
}

/// Adds `text` as plain text, a soft break between its lines.
fn push_accumulated_text(text: &str, followed_by_inline_node: bool, out: &mut Vec<Cow<'_, DocNode>>) {
    let soft_break = || Cow::Owned(DocSoftBreak::default().into());
    let last = text.matches('\n').count();
    for (i, line) in text.split('\n').enumerate() {
        if line.is_empty() {
            if followed_by_inline_node && i != 0 && i == last {
                out.push(soft_break());
            }
            continue;
        }
        if i != 0 {
            out.push(soft_break());
        }
        out.push(Cow::Owned(DocPlainText { text: line.to_owned(), text_excerpt: None }.into()));
    }
}
