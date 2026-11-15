//! Messages: what the parser reports wrong with a comment.

use super::{TSDocMessageId, TextRange, TokenSequence};

/// Something wrong in a comment, as TSDoc reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParserMessage {
    /// What it's about.
    pub message_id: TSDocMessageId,
    /// What's wrong, in TSDoc's words.
    pub unformatted_text: String,
    /// The same, after its line and column, like `(2,5): ...`, as `@microsoft/tsdoc`'s `text`.
    pub text: String,
    /// Where it is.
    pub text_range: TextRange,
    /// The tokens it's about, when it's about tokens.
    pub token_sequence: Option<TokenSequence>,
}

impl ParserMessage {
    /// A message about `text_range` of `buffer`, the parsed text. Only the parser makes them, with a
    /// text and a range inside `buffer`.
    pub(crate) fn new(
        buffer: &str,
        message_id: TSDocMessageId,
        unformatted_text: String,
        text_range: TextRange,
        token_sequence: Option<TokenSequence>,
    ) -> Self {
        let text = format_message_text(buffer, &unformatted_text, text_range);
        Self { message_id, unformatted_text, text, text_range, token_sequence }
    }
}

impl std::fmt::Display for ParserMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

/// `message` after the line and column `range` starts at, unless it's the empty range at the start.
/// TSDoc also leaves the prefix off when the range is outside the text, or fills in an empty
/// message; neither happens to a message the parser makes.
fn format_message_text(buffer: &str, message: &str, range: TextRange) -> String {
    if range == TextRange::EMPTY {
        return message.to_owned();
    }
    let location = TextRange::location(buffer, range.pos as isize);
    format!("({},{}): {message}", location.line, location.column)
}
