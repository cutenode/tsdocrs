//! Transforms of a tree: `@microsoft/tsdoc`'s DocNodeTransforms.

use crate::{
    nodes::{DocNode, DocParagraph, DocPlainText, DocSoftBreak},
    parser::is_whitespace,
};

/// A copy of `paragraph` with its spacing collapsed, as rendered text would show it: a run of
/// whitespace becomes one space, spaces at the paragraph's start and end go, and its plain text
/// and soft breaks are rebuilt by hand, one plain text a line.
pub fn trim_spaces_in_paragraph(paragraph: &DocParagraph) -> DocParagraph {
    let mut transformed: Vec<DocNode> = Vec::new();
    // The text since the last node that isn't text, and whether there's any, even empty.
    let (mut accumulated, mut accumulating) = (String::new(), false);
    let mut pending_space = false;
    let mut finished_skipping_leading_spaces = false;
    for node in &paragraph.nodes {
        match node {
            DocNode::PlainText(text) => {
                let collapsed = text.text.split(is_whitespace).filter(|word| !word.is_empty()).collect::<Vec<_>>();
                if text.text.starts_with(is_whitespace) && finished_skipping_leading_spaces {
                    pending_space = true;
                }
                if !collapsed.is_empty() {
                    if pending_space {
                        accumulated.push(' ');
                        pending_space = false;
                    }
                    accumulated += &collapsed.join(" ");
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
                transformed.push(node.clone());
                finished_skipping_leading_spaces = true;
            }
        }
    }
    if accumulating {
        push_accumulated_text(&accumulated, false, &mut transformed);
    }
    DocParagraph { nodes: transformed }
}

/// Adds `text` as plain text, a soft break between its lines.
fn push_accumulated_text(text: &str, followed_by_inline_node: bool, out: &mut Vec<DocNode>) {
    let lines: Vec<&str> = text.split('\n').collect();
    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            if followed_by_inline_node && i != 0 && i == lines.len() - 1 {
                out.push(DocSoftBreak::default().into());
            }
            continue;
        }
        if i != 0 {
            out.push(DocSoftBreak::default().into());
        }
        out.push(DocPlainText { text: (*line).to_owned(), text_excerpt: None }.into());
    }
}
