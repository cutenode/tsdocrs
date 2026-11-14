//! The parser on any text, inside a comment's delimiters or not: it never panics, and its excerpts
//! never cover a token twice.
#![no_main]

use libfuzzer_sys::fuzz_target;
use tsdocrs::{TSDocParser, nodes::NodeRef};

fuzz_target!(|text: &str| {
    for text in [text.to_owned(), format!("/**{text}*/")] {
        let Ok(context) = TSDocParser::default().parse_string(&text) else { continue };
        let mut covered = vec![false; context.tokens.len()];
        let mut stack = vec![NodeRef::Comment(&context.doc_comment)];
        while let Some(node) = stack.pop() {
            if let NodeRef::Excerpt(excerpt) = node {
                for slot in &mut covered[excerpt.content.start_index..excerpt.content.end_index] {
                    assert!(!std::mem::replace(slot, true), "two excerpts cover one token");
                }
            }
            stack.extend(node.child_nodes());
        }
    }
});
