//! The tokenizer: splits each line into tokens.

use std::sync::LazyLock;

use super::{TextRange, Token, TokenKind};

/// Reads `lines` of `buffer` into tokens: each line's, then a newline, and an end-of-input token
/// last.
pub fn read_tokens(buffer: &str, lines: &[TextRange]) -> Vec<Token> {
    // Tokens average a little over 3 bytes of text in real comments, so this rarely grows.
    let mut tokens = Vec::with_capacity(buffer.len() / 3 + lines.len() + 1);
    let kinds = &*KINDS;
    for (index, &line) in lines.iter().enumerate() {
        push_tokens_for_line(&mut tokens, kinds, buffer.as_bytes(), line, index);
    }
    let end = match lines.last() {
        Some(&last) => Token::new(TokenKind::EndOfInput, last.end, last.end, lines.len() - 1),
        None => Token::new(TokenKind::EndOfInput, 0, 0, 0),
    };
    tokens.push(end);
    tokens
}

/// Whether a backslash escapes a token of `kind`: whether it's CommonMark punctuation.
pub fn is_punctuation(kind: TokenKind) -> bool {
    !matches!(
        kind,
        TokenKind::EndOfInput | TokenKind::Newline | TokenKind::Spacing | TokenKind::AsciiWord | TokenKind::Other
    )
}

/// Each byte's token kind. Every character outside ASCII is `Other`, and runs of `Other` are one
/// token, so its bytes can be read one at a time like the rest.
static KINDS: LazyLock<[TokenKind; 256]> = LazyLock::new(|| std::array::from_fn(|byte| kind_of(byte as u8)));

fn push_tokens_for_line(
    tokens: &mut Vec<Token>,
    kinds: &[TokenKind; 256],
    bytes: &[u8],
    line: TextRange,
    index: usize,
) {
    let mut at = line.pos;
    while at < line.end {
        let (kind, pos) = (kinds[bytes[at] as usize], at);
        at += 1;
        // Runs of these are one token; punctuation is one character each.
        if matches!(kind, TokenKind::Spacing | TokenKind::AsciiWord | TokenKind::Other) {
            while at < line.end && kinds[bytes[at] as usize] == kind {
                at += 1;
            }
        }
        tokens.push(Token::new(kind, pos, at, index));
    }
    tokens.push(Token::new(TokenKind::Newline, line.end, line.end, index));
}

/// The kind of `byte`'s token. A byte outside ASCII is part of a character that's `Other`.
fn kind_of(byte: u8) -> TokenKind {
    match byte {
        b' ' | b'\t' => TokenKind::Spacing,
        b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' => TokenKind::AsciiWord,
        b'\\' => TokenKind::Backslash,
        b'<' => TokenKind::LessThan,
        b'>' => TokenKind::GreaterThan,
        b'=' => TokenKind::Equals,
        b'\'' => TokenKind::SingleQuote,
        b'"' => TokenKind::DoubleQuote,
        b'/' => TokenKind::Slash,
        b'-' => TokenKind::Hyphen,
        b'@' => TokenKind::AtSign,
        b'{' => TokenKind::LeftCurlyBracket,
        b'}' => TokenKind::RightCurlyBracket,
        b'`' => TokenKind::Backtick,
        b'.' => TokenKind::Period,
        b':' => TokenKind::Colon,
        b',' => TokenKind::Comma,
        b'[' => TokenKind::LeftSquareBracket,
        b']' => TokenKind::RightSquareBracket,
        b'|' => TokenKind::Pipe,
        b'(' => TokenKind::LeftParenthesis,
        b')' => TokenKind::RightParenthesis,
        b'#' => TokenKind::PoundSymbol,
        b'+' => TokenKind::Plus,
        b'$' => TokenKind::DollarSign,
        byte if byte.is_ascii_punctuation() => TokenKind::OtherPunctuation,
        _ => TokenKind::Other,
    }
}
