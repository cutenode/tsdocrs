//! The tokenizer: splits each line into tokens.

use super::{TextRange, Token, TokenKind};

/// Reads `lines` of `buffer` into tokens: each line's, then a newline, and an end-of-input token
/// last.
pub fn read_tokens(buffer: &str, lines: &[TextRange]) -> Vec<Token> {
    let mut tokens = Vec::new();
    for &line in lines {
        push_tokens_for_line(&mut tokens, buffer, line);
    }
    let end = match lines.last() {
        Some(&last) => Token { kind: TokenKind::EndOfInput, range: TextRange::new(last.end, last.end), line: last },
        None => Token { kind: TokenKind::EndOfInput, range: TextRange::EMPTY, line: TextRange::EMPTY },
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

fn push_tokens_for_line(tokens: &mut Vec<Token>, buffer: &str, line: TextRange) {
    let mut current: Option<(TokenKind, usize)> = None;
    for (at, c) in buffer[line.pos..line.end].char_indices() {
        let (kind, at) = (kind_of(c), line.pos + at);
        match current {
            // Runs of these are one token; punctuation is one character each.
            Some((same, _)) if same == kind && matches!(kind, TokenKind::Spacing | TokenKind::AsciiWord | TokenKind::Other) => {
            }
            _ => {
                if let Some((kind, pos)) = current {
                    tokens.push(Token { kind, range: TextRange::new(pos, at), line });
                }
                current = Some((kind, at));
            }
        }
    }
    if let Some((kind, pos)) = current {
        tokens.push(Token { kind, range: TextRange::new(pos, line.end), line });
    }
    tokens.push(Token { kind: TokenKind::Newline, range: TextRange::new(line.end, line.end), line });
}

/// The kind of `c`'s token.
fn kind_of(c: char) -> TokenKind {
    match c {
        ' ' | '\t' => TokenKind::Spacing,
        'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => TokenKind::AsciiWord,
        '\\' => TokenKind::Backslash,
        '<' => TokenKind::LessThan,
        '>' => TokenKind::GreaterThan,
        '=' => TokenKind::Equals,
        '\'' => TokenKind::SingleQuote,
        '"' => TokenKind::DoubleQuote,
        '/' => TokenKind::Slash,
        '-' => TokenKind::Hyphen,
        '@' => TokenKind::AtSign,
        '{' => TokenKind::LeftCurlyBracket,
        '}' => TokenKind::RightCurlyBracket,
        '`' => TokenKind::Backtick,
        '.' => TokenKind::Period,
        ':' => TokenKind::Colon,
        ',' => TokenKind::Comma,
        '[' => TokenKind::LeftSquareBracket,
        ']' => TokenKind::RightSquareBracket,
        '|' => TokenKind::Pipe,
        '(' => TokenKind::LeftParenthesis,
        ')' => TokenKind::RightParenthesis,
        '#' => TokenKind::PoundSymbol,
        '+' => TokenKind::Plus,
        '$' => TokenKind::DollarSign,
        c if c.is_ascii_punctuation() => TokenKind::OtherPunctuation,
        _ => TokenKind::Other,
    }
}
