//! Token sequences: runs of the parsed tokens, which every excerpt of the tree is.

use std::borrow::Cow;

use super::{TextRange, Token, TokenKind};

/// A run of tokens, by indices into the parse's tokens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TokenSequence {
    /// The first token's index.
    pub start_index: usize,
    /// The index past the last token.
    pub end_index: usize,
}

impl TokenSequence {
    /// The tokens from `start_index` up to `end_index`.
    pub fn new(start_index: usize, end_index: usize) -> Self {
        debug_assert!(start_index <= end_index, "TokenSequence.endIndex cannot be smaller than startIndex");
        Self { start_index, end_index }
    }

    /// Whether it has no tokens.
    pub fn is_empty(&self) -> bool {
        self.start_index == self.end_index
    }

    /// Its tokens, of `tokens`.
    pub fn tokens<'t>(&self, tokens: &'t [Token]) -> &'t [Token] {
        &tokens[self.start_index..self.end_index]
    }

    /// The range from its first token's start to its last token's end, past the `*`s between its
    /// lines, or [`TextRange::EMPTY`] when it has no tokens.
    pub fn containing_text_range(&self, tokens: &[Token]) -> TextRange {
        if self.is_empty() {
            return TextRange::EMPTY;
        }
        TextRange::new(tokens[self.start_index].range().pos, tokens[self.end_index - 1].range().end)
    }

    /// Its tokens' text, joined: what `@microsoft/tsdoc`'s `toString()` gives. Tokens on one line
    /// sit next to each other in `buffer`, so their text is borrowed from it; only a run across
    /// lines, past the `*`s between them, is copied.
    pub fn text<'b>(&self, buffer: &'b str, tokens: &[Token]) -> Cow<'b, str> {
        let tokens = self.tokens(tokens);
        if tokens.iter().any(|token| token.kind == TokenKind::Newline) {
            return Cow::Owned(tokens.iter().map(|token| token.text(buffer)).collect());
        }
        match tokens {
            [] => Cow::Borrowed(""),
            [first, .., last] | [first @ last] => Cow::Borrowed(&buffer[first.range().pos..last.range().end]),
        }
    }
}
