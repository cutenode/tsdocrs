//! The sizes of the types a parse makes many of, pinned, so a change that grows one shows up here.
//! Every node in a section or paragraph takes a `DocNode`'s size however small its own kind is, and
//! a parse makes a few tokens per input byte.

#![cfg(target_pointer_width = "64")]

use std::mem::size_of;

use tsdocrs::{
    ParserContext,
    nodes::{
        DocComment, DocErrorText, DocExcerpt, DocFencedCode, DocInlineTag, DocLinkTag, DocNode, DocParamBlock,
        DocPlainText, DocSoftBreak,
    },
    parser::{TextRange, Token},
};

#[test]
fn sizes_hold() {
    let sizes = [
        ("DocNode", size_of::<DocNode>(), 48),
        ("DocComment", size_of::<DocComment>(), 840),
        ("DocLinkTag", size_of::<DocLinkTag>(), 504),
        ("DocParamBlock", size_of::<DocParamBlock>(), 360),
        ("DocFencedCode", size_of::<DocFencedCode>(), 240),
        ("DocInlineTag", size_of::<DocInlineTag>(), 192),
        ("DocErrorText", size_of::<DocErrorText>(), 96),
        ("DocPlainText", size_of::<DocPlainText>(), 48),
        ("DocSoftBreak", size_of::<DocSoftBreak>(), 24),
        ("DocExcerpt", size_of::<DocExcerpt>(), 24),
        ("Token", size_of::<Token>(), 16),
        ("TextRange", size_of::<TextRange>(), 16),
        ("ParserContext", size_of::<ParserContext>(), 984),
    ];
    let wrong: Vec<String> = sizes
        .iter()
        .filter(|(_, size, pinned)| size != pinned)
        .map(|(name, size, pinned)| format!("{name} is {size} bytes, pinned at {pinned}"))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
