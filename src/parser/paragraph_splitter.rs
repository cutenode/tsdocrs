//! The paragraph splitter, which runs after the node parser: it splits each section's paragraphs
//! at blank lines, as CommonMark does. A blank line stays, at the end of the paragraph before it,
//! and blank lines starting a paragraph stay at its start.

use super::is_whitespace;
use crate::nodes::{DocComment, DocNode, DocParagraph, DocSection};

/// Splits the paragraphs of every section in `comment`.
pub(crate) fn split_paragraphs(comment: &mut DocComment) {
    let blocks = [&mut comment.remarks_block, &mut comment.private_remarks, &mut comment.deprecated_block]
        .into_iter()
        .flatten()
        .chain(&mut comment.returns_block)
        .chain(&mut comment.custom_blocks)
        .chain(&mut comment.see_blocks);
    let mut sections: Vec<&mut DocSection> = blocks.map(|block| &mut block.content).collect();
    sections.push(&mut comment.summary_section);
    sections.extend(comment.params.blocks.iter_mut().chain(&mut comment.type_params.blocks).map(|b| &mut b.content));
    sections.into_iter().for_each(split_paragraphs_for_section);
}

/// Where the splitter is in a paragraph.
#[derive(PartialEq, Eq)]
enum State {
    /// Skipping the blank lines it starts with.
    Start,
    /// In its text, looking for a blank line.
    AwaitingTrailer,
    /// In the blank lines after its text, looking for the next paragraph.
    ReadingTrailer,
}

/// Splits the paragraphs of `section`.
pub fn split_paragraphs_for_section(section: &mut DocSection) {
    let mut output = Vec::new();
    for node in std::mem::take(&mut section.nodes) {
        match node {
            DocNode::Paragraph(paragraph) => split_paragraph(paragraph, &mut output),
            node => output.push(node),
        }
    }
    section.nodes = output;
}

fn split_paragraph(old: DocParagraph, output: &mut Vec<DocNode>) {
    let mut paragraphs = vec![DocParagraph::default()];
    let mut state = State::Start;
    let mut nodes = old.nodes.into_iter().peekable();
    while nodes.peek().is_some() {
        // A line, up to and with its soft break.
        let mut line = Vec::new();
        let mut blank = true;
        for node in nodes.by_ref() {
            let soft_break = matches!(node, DocNode::SoftBreak(_));
            blank &= soft_break || is_whitespace_text(&node);
            line.push(node);
            if soft_break {
                break;
            }
        }
        match state {
            State::Start if !blank => state = State::AwaitingTrailer,
            State::AwaitingTrailer if blank => state = State::ReadingTrailer,
            State::ReadingTrailer if !blank => {
                paragraphs.push(DocParagraph::default());
                state = State::AwaitingTrailer;
            }
            _ => {}
        }
        paragraphs.last_mut().expect("there's always one").nodes.extend(line);
    }
    output.extend(paragraphs.into_iter().map(DocNode::Paragraph));
}

/// Whether `node` is plain text of nothing but whitespace.
fn is_whitespace_text(node: &DocNode) -> bool {
    match node {
        DocNode::PlainText(text) => text.text.chars().all(is_whitespace),
        _ => false,
    }
}
