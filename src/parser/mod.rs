//! The parser: [`TSDocParser`] reads a comment into a [`ParserContext`], which has its lines,
//! tokens, [`DocComment`] and messages.

mod line_extractor;
mod message_id;
mod node_parser;
mod paragraph_splitter;
mod parser_message;
pub(crate) mod string_checks;
mod text_range;
mod token;
mod token_reader;
mod token_sequence;
pub mod tokenizer;

pub use self::{
    message_id::TSDocMessageId,
    paragraph_splitter::split_paragraphs_for_section,
    parser_message::ParserMessage,
    text_range::{TextLocation, TextRange},
    token::{Token, TokenKind},
    token_sequence::TokenSequence,
};
use crate::{TSDocConfiguration, nodes::DocComment};

/// Whether `c` is whitespace to JavaScript's `\s`, which TSDoc's checks of whitespace use.
pub(crate) fn is_whitespace(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{feff}'
}

/// Why the parser stopped: where `@microsoft/tsdoc` throws an error, with its error's message.
/// TSDoc throws on a few inputs, like `{@link (Foo:}`, and this reports the same ones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// TSDoc's error message.
    pub message: String,
    /// The messages the parser logged before it stopped, which TSDoc doesn't give back.
    pub log: Vec<ParserMessage>,
}

impl ParseError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), log: Vec::new() }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

/// What a parse found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParserContext {
    /// The whole text it was given.
    pub buffer: String,
    /// The part of it that was parsed.
    pub source_range: TextRange,
    /// The comment, from its `/**` to its `*/`, or empty when there wasn't one.
    pub comment_range: TextRange,
    /// The text of each of the comment's lines.
    pub lines: Vec<TextRange>,
    /// The tokens of those lines, an end-of-input token last.
    pub tokens: Vec<Token>,
    /// The comment's tree.
    pub doc_comment: DocComment,
    /// What's wrong with it, in the order the parser found it.
    pub log: Vec<ParserMessage>,
    /// Every tag the parser read, in order, with the ones the tree leaves out: a block replaced by
    /// a later one of its kind, like a second `@remarks`, and a modifier tag used again. Not part
    /// of `@microsoft/tsdoc`'s model; it's for checks of what the tree loses, like a repeated tag.
    pub tags: Vec<ParsedTag>,
}

/// A tag the parser read: one it checked against the configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedTag {
    /// Its name, with its `@`, as written.
    pub tag_name: String,
    /// Whether it's written in braces, as an inline tag.
    pub inline: bool,
    /// The tokens of its name.
    pub token_sequence: TokenSequence,
}

impl ParserContext {
    /// The text of `sequence`'s tokens.
    pub fn text(&self, sequence: TokenSequence) -> String {
        sequence.text(&self.buffer, &self.tokens)
    }

    /// The text of `range`.
    pub fn range_text(&self, range: TextRange) -> &str {
        range.text(&self.buffer)
    }
}

/// The parser, set up with a configuration.
#[derive(Clone, Copy, Debug)]
pub struct TSDocParser<'c> {
    /// What it's set up with.
    pub configuration: &'c TSDocConfiguration,
}

/// The default configuration, shared.
static DEFAULT: std::sync::LazyLock<TSDocConfiguration> = std::sync::LazyLock::new(TSDocConfiguration::new);

impl Default for TSDocParser<'static> {
    /// A parser with the default configuration.
    fn default() -> Self {
        Self { configuration: &DEFAULT }
    }
}

impl<'c> TSDocParser<'c> {
    /// A parser set up with `configuration`.
    pub fn new(configuration: &'c TSDocConfiguration) -> Self {
        Self { configuration }
    }

    /// Parses `text`, a `/** */` comment with nothing but whitespace around it.
    pub fn parse_string(&self, text: &str) -> Result<ParserContext, ParseError> {
        self.parse_range(text, TextRange::new(0, text.len()))
    }

    /// Parses `text` as [`parse_string`](Self::parse_string) does, for its messages and tags,
    /// without splitting the comment's paragraphs at blank lines. That's the one step that logs
    /// nothing, so the result is the same but for `doc_comment`, where each paragraph runs on
    /// across blank lines, and a parse that fails fails the same way.
    pub fn check_string(&self, text: &str) -> Result<ParserContext, ParseError> {
        self.parse_range_with(text, TextRange::new(0, text.len()), false)
    }

    /// Parses the comment in `range` of `buffer`.
    pub fn parse_range(&self, buffer: &str, range: TextRange) -> Result<ParserContext, ParseError> {
        self.parse_range_with(buffer, range, true)
    }

    /// Parses the comment in `range` of `buffer`, splitting its paragraphs when `split` is set.
    fn parse_range_with(&self, buffer: &str, range: TextRange, split: bool) -> Result<ParserContext, ParseError> {
        let mut context = ParserContext {
            buffer: buffer.into(),
            source_range: range,
            comment_range: TextRange::EMPTY,
            lines: Vec::new(),
            tokens: Vec::new(),
            doc_comment: DocComment::default(),
            log: Vec::new(),
            tags: Vec::new(),
        };
        if let Some(extracted) = line_extractor::extract(buffer, range, &mut context.log) {
            context.comment_range = extracted.comment_range;
            context.lines = extracted.lines;
            context.tokens = tokenizer::read_tokens(buffer, &context.lines);
            if let Err(mut error) = node_parser::parse(&mut context, self.configuration) {
                error.log = context.log;
                return Err(error);
            }
            if split {
                paragraph_splitter::split_paragraphs(&mut context.doc_comment);
            }
        }
        Ok(context)
    }
}
