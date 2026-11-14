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

    /// Reads the next token, which mustn't be past the end or the end-of-input token.
    pub fn read_token(&mut self) -> Result<Token, ParseError> {
        if self.current >= self.end {
            return Err(ParseError::new("Cannot read past end of stream"));
        }
        let token = self.tokens[self.current];
        if token.kind == TokenKind::EndOfInput {
            return Err(ParseError::new("The EndOfInput token cannot be read"));
        }
        self.current += 1;
        Ok(token)
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

/// `s` as a JSON string, as `JSON.stringify` writes it.
pub(crate) fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
