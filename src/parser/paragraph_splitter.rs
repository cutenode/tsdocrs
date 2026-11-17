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

/// Splits the paragraphs of `section`. Most paragraphs don't split, so a paragraph's nodes move
/// only when it does, and only the ones after a split.
pub fn split_paragraphs_for_section(section: &mut DocSection) {
    let mut at = 0;
    while at < section.nodes.len() {
        if let DocNode::Paragraph(paragraph) = &mut section.nodes[at] {
            let rest = split_off_paragraphs(paragraph);
            let count = rest.len();
            section.nodes.splice(at + 1..at + 1, rest.into_iter().map(DocNode::Paragraph));
            at += count;
        }
        at += 1;
    }
}

/// Splits `paragraph` at its blank lines, keeping the first paragraph's nodes in it, and gives the
/// paragraphs after it.
fn split_off_paragraphs(paragraph: &mut DocParagraph) -> Vec<DocParagraph> {
    // Where each paragraph after the first starts.
    let mut starts = Vec::new();
    let mut state = State::Start;
    let mut line_start = 0;
    while line_start < paragraph.nodes.len() {
        // A line, up to and with its soft break.
        let mut blank = true;
        let mut line_end = line_start;
        for node in &paragraph.nodes[line_start..] {
            line_end += 1;
            let soft_break = matches!(node, DocNode::SoftBreak(_));
            blank &= soft_break || is_whitespace_text(node);
            if soft_break {
                break;
            }
        }
        match state {
            State::Start if !blank => state = State::AwaitingTrailer,
            State::AwaitingTrailer if blank => state = State::ReadingTrailer,
            State::ReadingTrailer if !blank => {
                starts.push(line_start);
                state = State::AwaitingTrailer;
            }
            _ => {}
        }
        line_start = line_end;
    }
    // From the last, so each split moves only the nodes after it.
    let mut rest: Vec<DocParagraph> =
        starts.iter().rev().map(|&start| DocParagraph { nodes: paragraph.nodes.split_off(start) }).collect();
    rest.reverse();
    rest
}

/// Whether `node` is plain text of nothing but whitespace.
fn is_whitespace_text(node: &DocNode) -> bool {
    match node {
        DocNode::PlainText(text) => text.text.chars().all(is_whitespace),
        _ => false,
    }
}
