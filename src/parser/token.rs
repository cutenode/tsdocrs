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

/// A token: its kind, where it is, and the line it's on. It keeps its offsets as `u32`, which
/// holds it to 16 bytes, so the text parsed is at most 4 GiB.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Token {
    /// What it is.
    pub kind: TokenKind,
    pos: u32,
    end: u32,
    line: u32,
}

impl Token {
    /// A token of `kind` from `pos` to `end`, on the line at `line` of the parse's lines.
    pub(crate) fn new(kind: TokenKind, pos: usize, end: usize, line: usize) -> Self {
        let offset = |at: usize| u32::try_from(at).expect("tsdocrs parses text of at most 4 GiB");
        Self { kind, pos: offset(pos), end: offset(end), line: offset(line) }
    }

    /// Where it is.
    pub fn range(&self) -> TextRange {
        TextRange::new(self.pos as usize, self.end as usize)
    }

    /// The index of the line it's on, in the parse's lines.
    pub fn line_index(&self) -> usize {
        self.line as usize
    }

    /// The line it's on, of the parse's `lines`, or [`TextRange::EMPTY`] for the end-of-input
    /// token of a comment with no lines.
    pub fn line(&self, lines: &[TextRange]) -> TextRange {
        lines.get(self.line_index()).copied().unwrap_or(TextRange::EMPTY)
    }

    /// Its text in `buffer`, the parsed text: `\n` for a newline.
    pub fn text<'b>(&self, buffer: &'b str) -> &'b str {
        match self.kind {
            TokenKind::Newline => "\n",
            _ => &buffer[self.pos as usize..self.end as usize],
        }
    }
}
