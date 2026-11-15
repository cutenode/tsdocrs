//! The token reader: a cursor over the tokens, or over the tokens of an inline tag's content,
//! which accumulates what it reads into sequences and can go back to a marker.

use super::{ParseError, Token, TokenKind, TokenSequence};

/// A cursor over `tokens[start..end]`.
pub(crate) struct TokenReader<'t> {
    pub tokens: &'t [Token],
    end: usize,
    current: usize,
    accumulated_start: usize,
}

impl<'t> TokenReader<'t> {
    /// A reader over `tokens`, or over `embedded` of them.
    pub fn new(tokens: &'t [Token], embedded: Option<TokenSequence>) -> Self {
        let (start, end) = embedded.map_or((0, tokens.len()), |s| (s.start_index, s.end_index));
        Self { tokens, end, current: start, accumulated_start: start }
    }

    /// The tokens read since the last sequence it gave. The parser only asks after reading some.
    pub fn extract_accumulated_sequence(&mut self) -> TokenSequence {
        debug_assert!(!self.is_accumulated_sequence_empty(), "the parser reads before it extracts");
        let sequence = TokenSequence::new(self.accumulated_start, self.current);
        self.accumulated_start = self.current;
        sequence
    }

    /// Whether it's read nothing since the last sequence it gave.
    pub fn is_accumulated_sequence_empty(&self) -> bool {
        self.accumulated_start == self.current
    }

    /// The tokens read since the last sequence it gave, if any.
    pub fn try_extract_accumulated_sequence(&mut self) -> Option<TokenSequence> {
        (!self.is_accumulated_sequence_empty()).then(|| self.extract_accumulated_sequence())
    }

    /// Checks, in debug builds, that it's read nothing since the last sequence it gave, as each
    /// construct's parse starts.
    pub fn assert_accumulated_sequence_is_empty(&self) {
        debug_assert!(self.is_accumulated_sequence_empty(), "a construct's parse starts with nothing accumulated");
    }

    /// The next token, which may be past the end of an inline tag's content.
    pub fn peek_token(&self) -> Token {
        self.tokens[self.current]
    }

    /// The next token's kind, or end of input past the end.
    pub fn peek_token_kind(&self) -> TokenKind {
        self.kind_at(self.current)
    }

    /// The kind of the token after the next.
    pub fn peek_token_after_kind(&self) -> TokenKind {
        self.kind_at(self.current + 1)
    }

    /// The kind of the token two after the next.
    pub fn peek_token_after_after_kind(&self) -> TokenKind {
        self.kind_at(self.current + 2)
    }

    fn kind_at(&self, at: usize) -> TokenKind {
        if at >= self.end { TokenKind::EndOfInput } else { self.tokens[at].kind }
    }

    /// Reads the next token. The parser reads a token only once it has peeked at it, or reads again
    /// tokens it read before going back, so the next token is always there and never the end.
    pub fn read_token(&mut self) -> Token {
        debug_assert!(self.peek_token_kind() != TokenKind::EndOfInput, "the parser peeks before it reads");
        self.current += 1;
        self.tokens[self.current - 1]
    }

    /// Reads the next token, or fails as TSDoc does when there's none: where a member selector's
    /// label is missing at the end of a `{@link}`, as in `{@link (Foo:}`, TSDoc reads past it.
    pub fn try_read_token(&mut self) -> Result<Token, ParseError> {
        if self.current >= self.end {
            return Err(ParseError::new("Cannot read past end of stream"));
        }
        Ok(self.read_token())
    }

    /// The kind of the token before the next, in all the tokens.
    pub fn peek_previous_token_kind(&self) -> TokenKind {
        match self.current {
            0 => TokenKind::EndOfInput,
            current => self.tokens[current - 1].kind,
        }
    }

    /// Where it is, to come back to.
    pub fn create_marker(&self) -> usize {
        self.current
    }

    /// Goes back to `marker`, which it took earlier.
    pub fn backtrack_to_marker(&mut self, marker: usize) {
        debug_assert!(marker <= self.current, "The marker has expired");
        self.current = marker;
        self.accumulated_start = self.accumulated_start.min(marker);
    }
}
