//! The line extractor: finds the `/** */` comment and the text of each of its lines, past the
//! `/**` or a line's leading `*` and the one space after it, up to the line's last character that
//! isn't whitespace.

use super::{ParserMessage, TSDocMessageId, TextRange, is_whitespace, text_range::Locator};

/// Where the extractor is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// Looking for `/*`.
    BeginComment1,
    /// Looking for the `*` after `/*`.
    BeginComment2,
    /// Collecting a line, right after the `/**`.
    CollectingFirstLine,
    /// Collecting a line, up to its newline.
    CollectingLine,
    /// After a newline, looking for a line's `*` or the `*/`.
    AdvancingLine,
}

/// What the extractor found: the comment's range, from `/**` to `*/`, and its lines.
pub struct Extracted {
    /// The comment's range.
    pub comment_range: TextRange,
    /// Its lines.
    pub lines: Vec<TextRange>,
}

/// Finds the comment in `range` of `buffer`, or reports why there isn't one to `log`.
pub fn extract(buffer: &str, range: TextRange, log: &mut Vec<ParserMessage>) -> Option<Extracted> {
    let mut comment_start = 0;
    let (mut line_start, mut line_end) = (0, 0);
    let mut next_index = range.pos;
    let mut state = State::BeginComment1;
    let mut lines = Vec::new();
    // Comments are mostly ASCII, where a byte is its character and decoding one would cost more.
    let char_at = |at: usize| match buffer.as_bytes().get(at) {
        _ if at >= range.end => None,
        Some(&byte) if byte.is_ascii() => Some(byte as char),
        _ => buffer[at..].chars().next(),
    };
    let fail = |log: &mut Vec<ParserMessage>, id, text: &str, range| {
        log.push(ParserMessage::new(buffer, &mut Locator::default(), id, text.into(), range, None));
        None
    };
    loop {
        let Some(current) = char_at(next_index) else {
            return match state {
                State::BeginComment1 | State::BeginComment2 => {
                    fail(log, TSDocMessageId::CommentNotFound, "Expecting a \"/**\" comment", range)
                }
                _ => fail(log, TSDocMessageId::CommentMissingClosingDelimiter, "Unexpected end of input", range),
            };
        };
        let current_index = next_index;
        next_index += current.len_utf8();
        let next = char_at(next_index);
        match state {
            State::BeginComment1 => {
                if current == '/' && next == Some('*') {
                    comment_start = current_index;
                    next_index += 1;
                    state = State::BeginComment2;
                } else if !is_whitespace(current) {
                    let at = TextRange::new(current_index, next_index);
                    return fail(log, TSDocMessageId::CommentOpeningDelimiterSyntax, "Expecting a leading \"/**\"", at);
                }
            }
            State::BeginComment2 => {
                if current != '*' {
                    let at = TextRange::new(current_index, next_index);
                    return fail(log, TSDocMessageId::CommentOpeningDelimiterSyntax, "Expecting a leading \"/**\"", at);
                }
                if next == Some(' ') {
                    next_index += 1;
                }
                (line_start, line_end) = (next_index, next_index);
                state = State::CollectingFirstLine;
            }
            State::CollectingFirstLine | State::CollectingLine => {
                if current == '\n' {
                    // An empty line right after the `/**` isn't one.
                    if state != State::CollectingFirstLine || line_end > line_start {
                        lines.push(TextRange::new(line_start, line_end));
                    }
                    (line_start, line_end) = (next_index, next_index);
                    state = State::AdvancingLine;
                } else if current == '*' && next == Some('/') {
                    if line_end > line_start {
                        lines.push(TextRange::new(line_start, line_end));
                    }
                    return Some(Extracted { comment_range: TextRange::new(comment_start, next_index + 1), lines });
                } else if !is_whitespace(current) {
                    line_end = next_index;
                }
            }
            State::AdvancingLine => {
                if current == '*' {
                    if next == Some('/') {
                        return Some(Extracted { comment_range: TextRange::new(comment_start, next_index + 1), lines });
                    }
                    if next == Some(' ') {
                        next_index += 1;
                    }
                    (line_start, line_end) = (next_index, next_index);
                    state = State::CollectingLine;
                } else if current == '\n' {
                    lines.push(TextRange::new(current_index, current_index));
                    line_start = next_index;
                } else if !is_whitespace(current) {
                    // A line without its `*` starts where its text does.
                    line_end = next_index;
                    state = State::CollectingLine;
                }
            }
        }
    }
}
