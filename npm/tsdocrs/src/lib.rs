//! The tsdocrs npm package: [`parse`] reads a TSDoc comment into the tree and messages
//! `@microsoft/tsdoc` would, as plain objects, with offsets in UTF-16 code units as JavaScript
//! strings index.

use serde::{
    Serialize, Serializer,
    ser::{SerializeMap as _, SerializeSeq as _},
};
use tsdocrs::{
    ParseError, ParserContext, TSDocConfigFile, TSDocConfiguration, TSDocParser, nodes::NodeRef, parser::TextRange,
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
 * comment's text a node was read from, with its `excerptKind`, `content`, `pos` and `end`. The other
 * fields are the node's own, where its kind has them.
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
    let context = with_configuration(configuration, |configuration| parse_with(text, configuration))?;
    let utf16 = Utf16::new(&context.buffer);
    // One JSON string across the boundary, parsed by the engine, is several times faster than
    // building each object with its own calls into JavaScript.
    let json = serde_json::to_string(&Found { context: &context, utf16: &utf16 })?;
    Ok(js_sys::JSON::parse(&json).expect_throw("serde_json writes valid JSON"))
}

/// Parses `text` as [`parse`] does and writes the comment back out as TSDoc, normalized, as
/// `@microsoft/tsdoc`'s `DocComment.emitAsTsdoc()` does.
#[wasm_bindgen]
pub fn emit(
    text: &str,
    #[wasm_bindgen(unchecked_param_type = "TSDocConfigFileJson | undefined")] configuration: JsValue,
) -> Result<String, JsError> {
    Ok(with_configuration(configuration, |configuration| parse_with(text, configuration))?.doc_comment.emit_as_tsdoc())
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

fn parse_with(text: &str, configuration: &TSDocConfiguration) -> Result<ParserContext, JsError> {
    TSDocParser::new(configuration).parse_string(text).map_err(|ParseError { message, .. }| JsError::new(&message))
}

/// UTF-16 offsets for the byte offsets of a string's characters and its end.
struct Utf16(Vec<u32>);

impl Utf16 {
    fn new(text: &str) -> Self {
        let mut offsets = vec![0; text.len() + 1];
        let mut utf16 = 0;
        for (at, c) in text.char_indices() {
            offsets[at] = utf16;
            utf16 += c.len_utf16() as u32;
        }
        offsets[text.len()] = utf16;
        Self(offsets)
    }

    fn range(&self, range: TextRange) -> (u32, u32) {
        (self.0[range.pos], self.0[range.end])
    }
}

/// What a parse found, written as JSON straight from the tree.
struct Found<'a> {
    context: &'a ParserContext,
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

/// The text of each of the comment's lines.
struct Lines<'a>(&'a ParserContext);

impl Serialize for Lines<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.lines.iter().map(|&line| self.0.range_text(line)))
    }
}

struct Messages<'a> {
    context: &'a ParserContext,
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
    context: &'a ParserContext,
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
                map.serialize_entry("content", &context.text(excerpt.content))?;
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
                map.serialize_entry("selectorKind", &format!("{:?}", selector.selector_kind).to_lowercase())?;
            }
            _ => {}
        }
        let children = node.child_nodes();
        if !children.is_empty() {
            let nodes: Vec<Node> = children.into_iter().map(|node| Node { context, utf16, node }).collect();
            map.serialize_entry("nodes", &nodes)?;
        }
        map.end()
    }
}
