//! The tsdocrs npm package: [`parse`] reads a TSDoc comment into the tree and messages
//! `@microsoft/tsdoc` would, as plain objects, with offsets in UTF-16 code units as JavaScript
//! strings index. [`check`] gives the messages and tags alone.

use serde::{
    Serialize, Serializer,
    ser::{SerializeMap as _, SerializeSeq as _},
};
use tsdocrs::{
    ParseError, ParserContext, TSDocConfigFile, TSDocConfiguration, TSDocParser,
    nodes::{NodeRef, SelectorKind},
    parser::{TextRange, TokenKind},
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &str = r#"
/** A tsdoc.json's settings, without `extends`. */
export interface TSDocConfigFileJson {
  noStandardTags?: boolean;
  tagDefinitions?: { tagName: string; syntaxKind: "inline" | "block" | "modifier"; allowMultiple?: boolean }[];
  supportForTags?: Record<string, boolean>;
  supportedHtmlElements?: string[];
  reportUnsupportedHtmlElements?: boolean;
}

/** Something wrong in a comment, as TSDoc reports it. Offsets are UTF-16 code units. */
export interface ParserMessage {
  /** TSDoc's message id, like `tsdoc-undefined-tag`. */
  messageId: string;
  /** What's wrong, after its line and column, like `(2,5): ...`. */
  text: string;
  /** What's wrong. */
  unformattedText: string;
  pos: number;
  end: number;
}

/**
 * A node of the tree, with its children in `@microsoft/tsdoc`'s order. An `Excerpt` is a run of the
 * comment's text a node was read from, with its `excerptKind`, `pos` and `end`. Its text is the
 * comment's from `pos` to `end`, unless it runs across a line break: then `content` has it, without
 * the `*` that starts the next line, and a soft break's is `"\n"`. The other fields are the node's
 * own, where its kind has them.
 */
export interface DocNode {
  kind: string;
  nodes?: DocNode[];
  excerptKind?: string;
  content?: string;
  pos?: number;
  end?: number;
  tagName?: string;
  text?: string;
  messageId?: string;
  errorMessage?: string;
  encodedText?: string;
  decodedText?: string;
  code?: string;
  language?: string;
  name?: string;
  value?: string;
  selfClosingTag?: boolean;
  tagContent?: string;
  urlDestination?: string;
  linkText?: string;
  parameterName?: string;
  packageName?: string;
  importPath?: string;
  identifier?: string;
  hasQuotes?: boolean;
  hasDot?: boolean;
  selector?: string;
  selectorKind?: "error" | "system" | "index" | "label";
}

/** A tag the parser read, the ones the tree leaves out included, like a second `@remarks`. */
export interface ParsedTag {
  /** Its name, with its `@`, as written. */
  tagName: string;
  /** Whether it's written in braces, as an inline tag. */
  inline: boolean;
  /** Where its name starts, in UTF-16 code units. */
  pos: number;
  /** Where its name ends. */
  end: number;
}

/** What a check found. */
export interface CheckResult {
  /** What's wrong, in the order the parser found it. */
  messages: ParserMessage[];
  /** Every tag the parser read, in order. */
  tags: ParsedTag[];
}

/** What a parse found. */
export interface ParseResult {
  /** The text of each of the comment's lines. */
  lines: string[];
  /** What's wrong, in the order the parser found it. */
  messages: ParserMessage[];
  /** The comment's tree, a `Comment` node. */
  docComment: DocNode;
}
"#;

/// Parses `text`, a TSDoc comment with its delimiters, with the tsdoc.json settings `configuration`. It throws
/// where `@microsoft/tsdoc` throws, with the same message, and on settings that can't be used.
#[wasm_bindgen(unchecked_return_type = "ParseResult")]
pub fn parse(
    text: &str,
    #[wasm_bindgen(unchecked_param_type = "TSDocConfigFileJson | undefined")] configuration: JsValue,
) -> Result<JsValue, JsError> {
    let context = parse_with(text, configuration, |parser, text| parser.parse_string(text))?;
    to_js(&Found { context: &context, utf16: &Utf16::new(context.buffer) })
}

/// Parses `text` as [`parse`] does, for its messages and tags alone. It leaves out the tree, which
/// is most of what [`parse`] costs, and skips splitting paragraphs, a step that logs nothing.
#[wasm_bindgen(unchecked_return_type = "CheckResult")]
pub fn check(
    text: &str,
    #[wasm_bindgen(unchecked_param_type = "TSDocConfigFileJson | undefined")] configuration: JsValue,
) -> Result<JsValue, JsError> {
    let context = parse_with(text, configuration, |parser, text| parser.check_string(text))?;
    to_js(&Checked { context: &context, utf16: &Utf16::new(context.buffer) })
}

/// `value` as a JavaScript object. One JSON string across the boundary, parsed by the engine, is
/// several times faster than building each object with its own calls into JavaScript.
fn to_js(value: &impl Serialize) -> Result<JsValue, JsError> {
    let json = serde_json::to_string(value)?;
    Ok(js_sys::JSON::parse(&json).expect_throw("serde_json writes valid JSON"))
}

/// Parses `text` as [`parse`] does and writes the comment back out as TSDoc, normalized, as
/// `@microsoft/tsdoc`'s `DocComment.emitAsTsdoc()` does.
#[wasm_bindgen]
pub fn emit(
    text: &str,
    #[wasm_bindgen(unchecked_param_type = "TSDocConfigFileJson | undefined")] configuration: JsValue,
) -> Result<String, JsError> {
    Ok(parse_with(text, configuration, |parser, text| parser.parse_string(text))?.doc_comment.emit_as_tsdoc())
}

thread_local! {
    /// The default configuration, built once: building it costs about a third as much as parsing a comment.
    static DEFAULT_CONFIGURATION: TSDocConfiguration = TSDocConfiguration::new();
}

/// Runs `f` with the configuration tsdoc.json settings set up, or the default one when there are none.
fn with_configuration<T>(
    configuration: JsValue,
    f: impl FnOnce(&TSDocConfiguration) -> Result<T, JsError>,
) -> Result<T, JsError> {
    if configuration.is_undefined() || configuration.is_null() {
        return DEFAULT_CONFIGURATION.with(f);
    }
    let file: TSDocConfigFile = serde_wasm_bindgen::from_value(configuration)?;
    f(&file.configure_parser().map_err(|(id, text)| JsError::new(&format!("{id}: {text}")))?)
}

/// Runs `parse`, a parser's `parse_string` or `check_string`, on `text` with the parser the
/// tsdoc.json settings `configuration` set up.
fn parse_with<'t>(
    text: &'t str,
    configuration: JsValue,
    parse: for<'p> fn(&TSDocParser<'p>, &'t str) -> Result<ParserContext<'t>, ParseError>,
) -> Result<ParserContext<'t>, JsError> {
    with_configuration(configuration, |configuration| {
        parse(&TSDocParser::new(configuration), text).map_err(|ParseError { message, .. }| JsError::new(&message))
    })
}

/// UTF-16 offsets for the byte offsets of a string's characters and its end, or none for ASCII,
/// where they're the same.
struct Utf16(Option<Vec<u32>>);

impl Utf16 {
    fn new(text: &str) -> Self {
        if text.is_ascii() {
            return Self(None);
        }
        let mut offsets = vec![0; text.len() + 1];
        let mut utf16 = 0;
        for (at, c) in text.char_indices() {
            offsets[at] = utf16;
            utf16 += c.len_utf16() as u32;
        }
        offsets[text.len()] = utf16;
        Self(Some(offsets))
    }

    fn range(&self, range: TextRange) -> (u32, u32) {
        match &self.0 {
            Some(offsets) => (offsets[range.pos], offsets[range.end]),
            None => (range.pos as u32, range.end as u32),
        }
    }
}

/// What a parse found, written as JSON straight from the tree.
struct Found<'a> {
    context: &'a ParserContext<'a>,
    utf16: &'a Utf16,
}

impl Serialize for Found<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Self { context, utf16 } = *self;
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("lines", &Lines(context))?;
        map.serialize_entry("messages", &Messages { context, utf16 })?;
        map.serialize_entry("docComment", &Node { context, utf16, node: NodeRef::Comment(&context.doc_comment) })?;
        map.end()
    }
}

/// The messages and tags of a check.
struct Checked<'a> {
    context: &'a ParserContext<'a>,
    utf16: &'a Utf16,
}

impl Serialize for Checked<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Self { context, utf16 } = *self;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("messages", &Messages { context, utf16 })?;
        map.serialize_entry("tags", &Tags { context, utf16 })?;
        map.end()
    }
}

/// Every tag the parser read, with where its name is.
struct Tags<'a> {
    context: &'a ParserContext<'a>,
    utf16: &'a Utf16,
}

impl Serialize for Tags<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Self { context, utf16 } = *self;
        serializer.collect_seq(context.tags.iter().map(|tag| {
            let (pos, end) = utf16.range(tag.token_sequence.containing_text_range(&context.tokens));
            Tag { tag_name: &tag.tag_name, inline: tag.inline, pos, end }
        }))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Tag<'a> {
    tag_name: &'a str,
    inline: bool,
    pos: u32,
    end: u32,
}

/// The text of each of the comment's lines.
struct Lines<'a>(&'a ParserContext<'a>);

impl Serialize for Lines<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.lines.iter().map(|&line| self.0.range_text(line)))
    }
}

struct Messages<'a> {
    context: &'a ParserContext<'a>,
    utf16: &'a Utf16,
}

impl Serialize for Messages<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut messages = serializer.serialize_seq(Some(self.context.log.len()))?;
        for message in &self.context.log {
            let (pos, end) = self.utf16.range(message.text_range);
            messages.serialize_element(&Message {
                message_id: message.message_id.as_str(),
                text: &message.text,
                unformatted_text: &message.unformatted_text,
                pos,
                end,
            })?;
        }
        messages.end()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Message<'a> {
    message_id: &'a str,
    text: &'a str,
    unformatted_text: &'a str,
    pos: u32,
    end: u32,
}

/// A node and its children as a plain object.
struct Node<'a> {
    context: &'a ParserContext<'a>,
    utf16: &'a Utf16,
    node: NodeRef<'a>,
}

impl Serialize for Node<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Self { context, utf16, node } = *self;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("kind", node.kind().as_str())?;
        match node {
            NodeRef::Excerpt(excerpt) => {
                let (pos, end) = utf16.range(excerpt.content.containing_text_range(&context.tokens));
                map.serialize_entry("excerptKind", excerpt.excerpt_kind.as_str())?;
                // Most excerpts are on one line, where JavaScript can slice the text itself, so only
                // the others carry it.
                let tokens = excerpt.content.tokens(&context.tokens);
                if tokens.iter().any(|token| token.kind == TokenKind::Newline) {
                    map.serialize_entry("content", &context.text(excerpt.content))?;
                }
                map.serialize_entry("pos", &pos)?;
                map.serialize_entry("end", &end)?;
            }
            NodeRef::BlockTag(tag) => map.serialize_entry("tagName", &tag.tag_name)?,
            NodeRef::PlainText(text) => map.serialize_entry("text", &text.text)?,
            NodeRef::ErrorText(error) => {
                map.serialize_entry("text", &error.text)?;
                map.serialize_entry("messageId", error.message_id.as_str())?;
                map.serialize_entry("errorMessage", &error.error_message)?;
            }
            NodeRef::EscapedText(escaped) => {
                map.serialize_entry("encodedText", &escaped.encoded_text)?;
                map.serialize_entry("decodedText", &escaped.decoded_text)?;
            }
            NodeRef::CodeSpan(code) => map.serialize_entry("code", &code.code)?,
            NodeRef::FencedCode(code) => {
                map.serialize_entry("language", &code.language)?;
                map.serialize_entry("code", &code.code)?;
            }
            NodeRef::HtmlStartTag(tag) => {
                map.serialize_entry("name", &tag.name)?;
                map.serialize_entry("selfClosingTag", &tag.self_closing_tag)?;
            }
            NodeRef::HtmlEndTag(tag) => map.serialize_entry("name", &tag.name)?,
            NodeRef::HtmlAttribute(attribute) => {
                map.serialize_entry("name", &attribute.name)?;
                map.serialize_entry("value", &attribute.value)?;
            }
            NodeRef::InlineTag(tag) => {
                map.serialize_entry("tagName", &tag.base.tag_name)?;
                map.serialize_entry("tagContent", &tag.tag_content)?;
            }
            NodeRef::LinkTag(link) => {
                map.serialize_entry("tagName", &link.base.tag_name)?;
                if let Some(url) = &link.url_destination {
                    map.serialize_entry("urlDestination", url)?;
                }
                if let Some(text) = &link.link_text {
                    map.serialize_entry("linkText", text)?;
                }
            }
            NodeRef::InheritDocTag(tag) => map.serialize_entry("tagName", &tag.base.tag_name)?,
            NodeRef::ParamBlock(block) => map.serialize_entry("parameterName", &block.parameter_name)?,
            NodeRef::DeclarationReference(reference) => {
                if let Some(name) = &reference.package_name {
                    map.serialize_entry("packageName", name)?;
                }
                if let Some(path) = &reference.import_path {
                    map.serialize_entry("importPath", path)?;
                }
            }
            NodeRef::MemberIdentifier(identifier) => {
                map.serialize_entry("identifier", &identifier.identifier)?;
                map.serialize_entry("hasQuotes", &identifier.has_quotes())?;
            }
            NodeRef::MemberReference(member) => map.serialize_entry("hasDot", &member.has_dot)?,
            NodeRef::MemberSelector(selector) => {
                map.serialize_entry("selector", &selector.selector)?;
                let kind = match selector.selector_kind {
                    SelectorKind::Error => "error",
                    SelectorKind::System => "system",
                    SelectorKind::Index => "index",
                    SelectorKind::Label => "label",
                };
                map.serialize_entry("selectorKind", kind)?;
            }
            _ => {}
        }
        let nodes = node.child_nodes();
        if !nodes.is_empty() {
            map.serialize_entry("nodes", &Nodes { context, utf16, nodes })?;
        }
        map.end()
    }
}

/// A node's children.
struct Nodes<'a> {
    context: &'a ParserContext<'a>,
    utf16: &'a Utf16,
    nodes: Vec<NodeRef<'a>>,
}

impl Serialize for Nodes<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Self { context, utf16, .. } = *self;
        serializer.collect_seq(self.nodes.iter().map(|&node| Node { context, utf16, node }))
    }
}
