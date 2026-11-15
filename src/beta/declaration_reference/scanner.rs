//! The tokens of a declaration reference.
//!
//! Upstream scans UTF-16 code units, and this scans bytes: every character it tells apart is ASCII,
//! so the tokens end in the same places.

/// A token, as upstream's `Token` has them. Its keywords and decimal digits are text here, which
/// the parser checks where it reads them. It has no `None` or `ModuleSource`: the scanner reads its
/// first token when it's made, and [`Scanner::rescan_module_source`] says whether a module source is
/// next instead of making it the token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Token {
    Eof,
    OpenBrace,
    CloseBrace,
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    Exclamation,
    Dot,
    Hash,
    Tilde,
    Colon,
    Comma,
    At,
    String,
    Text,
}

impl Token {
    /// How upstream names it in messages.
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Eof => "<eof>",
            Self::OpenBrace => "{",
            Self::CloseBrace => "}",
            Self::OpenParen => "(",
            Self::CloseParen => ")",
            Self::OpenBracket => "[",
            Self::CloseBracket => "]",
            Self::Exclamation => "!",
            Self::Dot => ".",
            Self::Hash => "#",
            Self::Tilde => "~",
            Self::Colon => ":",
            Self::Comma => ",",
            Self::At => "@",
            Self::String => "<string>",
            Self::Text => "<text>",
        }
    }
}

/// The punctuator token `ch` is.
fn punctuator(ch: u8) -> Option<Token> {
    Some(match ch {
        b'{' => Token::OpenBrace,
        b'}' => Token::CloseBrace,
        b'(' => Token::OpenParen,
        b')' => Token::CloseParen,
        b'[' => Token::OpenBracket,
        b']' => Token::CloseBracket,
        b'!' => Token::Exclamation,
        b'.' => Token::Dot,
        b'#' => Token::Hash,
        b'~' => Token::Tilde,
        b':' => Token::Colon,
        b',' => Token::Comma,
        b'@' => Token::At,
        _ => return None,
    })
}

fn is_punctuator(ch: u8) -> bool {
    punctuator(ch).is_some()
}

/// The line terminators upstream checks for. (It leaves out U+2028 and U+2029.)
fn is_line_terminator(ch: u8) -> bool {
    matches!(ch, b'\r' | b'\n')
}

/// Whether `text` is one string or text token, or nothing.
pub(super) fn is_well_formed_component_string(text: &str) -> bool {
    let mut scanner = Scanner::new(text);
    match scanner.token() {
        Token::String | Token::Text => scanner.scan() == Token::Eof,
        token => token == Token::Eof,
    }
}

/// Whether `text` and a `!` after it are one terminated module source.
pub(super) fn is_well_formed_module_source_string(text: &str) -> bool {
    let text = format!("{text}!");
    let mut scanner = Scanner::new(&text);
    scanner.rescan_module_source()
        && !scanner.string_is_unterminated
        && scanner.scan() == Token::Exclamation
        && scanner.scan() == Token::Eof
}

/// What [`Scanner::rescan_module_source`] has read so far.
#[derive(PartialEq, Eq)]
enum Scanned {
    None,
    String,
    Other,
}

pub(super) struct Scanner<'a> {
    text: &'a str,
    token_pos: usize,
    pos: usize,
    token: Token,
    pub(super) string_is_unterminated: bool,
}

impl<'a> Scanner<'a> {
    /// The scanner at the first token of `text`.
    pub(super) fn new(text: &'a str) -> Self {
        let mut scanner = Self { text, token_pos: 0, pos: 0, token: Token::Eof, string_is_unterminated: false };
        scanner.scan();
        scanner
    }

    pub(super) fn token(&self) -> Token {
        self.token
    }

    pub(super) fn token_text(&self) -> &'a str {
        &self.text[self.token_pos..self.pos]
    }

    fn eof(&self) -> bool {
        self.pos >= self.text.len()
    }

    fn byte(&self, at: usize) -> Option<u8> {
        self.text.as_bytes().get(at).copied()
    }

    pub(super) fn scan(&mut self) -> Token {
        let Some(ch) = self.byte(self.pos) else {
            self.token = Token::Eof;
            return self.token;
        };
        self.token_pos = self.pos;
        self.string_is_unterminated = false;
        self.pos += 1;
        self.token = match punctuator(ch) {
            Some(token) => token,
            None if ch == b'"' => {
                self.scan_string();
                Token::String
            }
            None => {
                self.scan_text();
                Token::Text
            }
        };
        self.token
    }

    /// Reads the token again, as a module source if there's a `!` after it: one string, or text
    /// and punctuators. Whether it was one; if so, the token's text is the module source.
    pub(super) fn rescan_module_source(&mut self) -> bool {
        if matches!(self.token, Token::Exclamation | Token::Eof) || self.eof() {
            return false;
        }
        let (pos, string_is_unterminated) = (self.pos, self.string_is_unterminated);
        self.pos = self.token_pos;
        self.string_is_unterminated = false;
        let mut scanned = Scanned::None;
        while let Some(ch) = self.byte(self.pos) {
            // The token isn't a `!`, so a `!` here has something before it.
            if ch == b'!' {
                return true;
            }
            self.pos += 1;
            if ch == b'"' {
                // A string can't follow anything else.
                if scanned == Scanned::Other {
                    break;
                }
                scanned = Scanned::String;
                self.scan_string();
            } else {
                // Nothing can follow a string.
                if scanned == Scanned::String {
                    break;
                }
                scanned = Scanned::Other;
                if !is_punctuator(ch) {
                    self.scan_text();
                }
            }
        }
        self.pos = pos;
        self.string_is_unterminated = string_is_unterminated;
        false
    }

    fn scan_string(&mut self) {
        while let Some(ch) = self.byte(self.pos) {
            self.pos += 1;
            match ch {
                b'"' => return,
                b'\\' => self.scan_escape_sequence(),
                ch if is_line_terminator(ch) => {
                    self.string_is_unterminated = true;
                    return;
                }
                _ => {}
            }
        }
        self.string_is_unterminated = true;
    }

    /// Reads an ECMAScript escape sequence after a `\`, or marks the string unterminated and reads
    /// nothing if there isn't one.
    fn scan_escape_sequence(&mut self) {
        let Some(ch) = self.byte(self.pos) else {
            self.string_is_unterminated = true;
            return;
        };
        let is_hex = |at| self.byte(at).is_some_and(|ch: u8| ch.is_ascii_hexdigit());
        let length = match ch {
            // A character escape: anything but a line terminator, digit, `x` or `u`.
            _ if !is_line_terminator(ch) && !ch.is_ascii_digit() && ch != b'x' && ch != b'u' => Some(1),
            b'0' if !self.byte(self.pos + 1).is_some_and(|ch| ch.is_ascii_digit()) => Some(1),
            b'x' if is_hex(self.pos + 1) && is_hex(self.pos + 2) => Some(3),
            b'u' if (1..=4).all(|i| is_hex(self.pos + i)) => Some(5),
            // Upstream reads only the first digit of a `\u{...}` code point, which is never too
            // big, so any run of hex digits and a `}` is accepted.
            b'u' if self.byte(self.pos + 1) == Some(b'{')
                && is_hex(self.pos + 2)
                && self.pos + 4 <= self.text.len() =>
            {
                let digits = self.text.as_bytes()[self.pos + 3..].iter().position(|ch| !ch.is_ascii_hexdigit());
                digits.filter(|&digits| self.byte(self.pos + 3 + digits) == Some(b'}')).map(|digits| digits + 4)
            }
            _ => None,
        };
        match length {
            Some(length) => self.pos += length,
            None => self.string_is_unterminated = true,
        }
    }

    fn scan_text(&mut self) {
        while let Some(ch) = self.byte(self.pos) {
            if is_punctuator(ch) || ch == b'"' {
                return;
            }
            self.pos += 1;
        }
    }
}
