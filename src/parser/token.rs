//! Tokens: what the tokenizer splits a comment's lines into.

use super::TextRange;

/// What a token is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// Past the last line. Only the last token is one, and it's never read.
    EndOfInput,
    /// The end of a line. Its range is empty, and its text is `\n`.
    Newline,
    /// Spaces and tabs.
    Spacing,
    /// ASCII letters, digits and `_`.
    AsciiWord,
    /// A CommonMark punctuation character no other kind names.
    OtherPunctuation,
    /// Anything else, such as other letters.
    Other,
    /// `\`
    Backslash,
    /// `<`
    LessThan,
    /// `>`
    GreaterThan,
    /// `=`
    Equals,
    /// `'`
    SingleQuote,
    /// `"`
    DoubleQuote,
    /// `/`
    Slash,
    /// `-`
    Hyphen,
    /// `@`
    AtSign,
    /// `{`
    LeftCurlyBracket,
    /// `}`
    RightCurlyBracket,
    /// `` ` ``
    Backtick,
    /// `.`
    Period,
    /// `:`
    Colon,
    /// `,`
    Comma,
    /// `[`
    LeftSquareBracket,
    /// `]`
    RightSquareBracket,
    /// `|`
    Pipe,
    /// `(`
    LeftParenthesis,
    /// `)`
    RightParenthesis,
    /// `#`
    PoundSymbol,
    /// `+`
    Plus,
    /// `$`
    DollarSign,
}

impl TokenKind {
    /// Its name, as `@microsoft/tsdoc`'s enum names it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EndOfInput => "EndOfInput",
            Self::Newline => "Newline",
            Self::Spacing => "Spacing",
            Self::AsciiWord => "AsciiWord",
            Self::OtherPunctuation => "OtherPunctuation",
            Self::Other => "Other",
            Self::Backslash => "Backslash",
            Self::LessThan => "LessThan",
            Self::GreaterThan => "GreaterThan",
            Self::Equals => "Equals",
            Self::SingleQuote => "SingleQuote",
            Self::DoubleQuote => "DoubleQuote",
            Self::Slash => "Slash",
            Self::Hyphen => "Hyphen",
            Self::AtSign => "AtSign",
            Self::LeftCurlyBracket => "LeftCurlyBracket",
            Self::RightCurlyBracket => "RightCurlyBracket",
            Self::Backtick => "Backtick",
            Self::Period => "Period",
            Self::Colon => "Colon",
            Self::Comma => "Comma",
            Self::LeftSquareBracket => "LeftSquareBracket",
            Self::RightSquareBracket => "RightSquareBracket",
            Self::Pipe => "Pipe",
            Self::LeftParenthesis => "LeftParenthesis",
            Self::RightParenthesis => "RightParenthesis",
            Self::PoundSymbol => "PoundSymbol",
            Self::Plus => "Plus",
            Self::DollarSign => "DollarSign",
        }
    }
}

/// A token: its kind, where it is, and the line it's on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Token {
    /// What it is.
    pub kind: TokenKind,
    /// Where it is.
    pub range: TextRange,
    /// The line it's on.
    pub line: TextRange,
}

impl Token {
    /// Its text in `buffer`, the parsed text: `\n` for a newline.
    pub fn text<'b>(&self, buffer: &'b str) -> &'b str {
        match self.kind {
            TokenKind::Newline => "\n",
            _ => self.range.text(buffer),
        }
    }
}
