//! The tsdocrs npm package: [`parse`] reads a TSDoc comment into the tree and messages
//! `@microsoft/tsdoc` would, as plain objects, with offsets in UTF-16 code units as JavaScript
//! strings index.

use serde::Serialize as _;
use serde_json::{Map, Value, json};
use tsdocrs::{
    ParseError, ParserContext, TSDocConfigFile, TSDocConfiguration, TSDocParser,
    nodes::NodeRef,
    parser::TextRange,
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

/// Parses `text`, a `/** */` comment, with the tsdoc.json settings `configuration`. It throws
/// where `@microsoft/tsdoc` throws, with the same message, and on settings that can't be used.
#[wasm_bindgen(unchecked_return_type = "ParseResult")]
pub fn parse(
    text: &str,
    #[wasm_bindgen(unchecked_param_type = "TSDocConfigFileJson | undefined")] configuration: JsValue,
) -> Result<JsValue, JsError> {
    let configuration = if configuration.is_undefined() || configuration.is_null() {
        TSDocConfiguration::new()
    } else {
        let file: TSDocConfigFile = serde_wasm_bindgen::from_value(configuration)?;
        file.configure_parser().map_err(|(id, text)| JsError::new(&format!("{id}: {text}")))?
    };
    let context = TSDocParser::new(&configuration)
        .parse_string(text)
        .map_err(|ParseError { message, .. }| JsError::new(&message))?;
    Ok(result(&context).serialize(&serde_wasm_bindgen::Serializer::json_compatible())?)
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

fn result(context: &ParserContext) -> Value {
    let utf16 = Utf16::new(&context.buffer);
    let messages: Vec<Value> = context
        .log
        .iter()
        .map(|message| {
            let (pos, end) = utf16.range(message.text_range);
            json!({
                "messageId": message.message_id.as_str(),
                "text": message.text,
                "unformattedText": message.unformatted_text,
                "pos": pos,
                "end": end,
            })
        })
        .collect();
    let lines: Vec<&str> = context.lines.iter().map(|&line| context.range_text(line)).collect();
    json!({
        "lines": lines,
        "messages": messages,
        "docComment": node(context, &utf16, NodeRef::Comment(&context.doc_comment)),
    })
}

/// `node` and its children as plain objects.
fn node(context: &ParserContext, utf16: &Utf16, node: NodeRef) -> Value {
    let mut item = Map::new();
    let mut set = |key: &str, value: Value| {
        item.insert(key.into(), value);
    };
    set("kind", json!(node.kind().as_str()));
    match node {
        NodeRef::Excerpt(excerpt) => {
            let (pos, end) = utf16.range(excerpt.content.containing_text_range(&context.tokens));
            set("excerptKind", json!(excerpt.excerpt_kind.as_str()));
            set("content", json!(context.text(excerpt.content)));
            set("pos", json!(pos));
            set("end", json!(end));
        }
        NodeRef::BlockTag(tag) => set("tagName", json!(tag.tag_name)),
        NodeRef::PlainText(text) => set("text", json!(text.text)),
        NodeRef::ErrorText(error) => {
            set("text", json!(error.text));
            set("messageId", json!(error.message_id.as_str()));
            set("errorMessage", json!(error.error_message));
        }
        NodeRef::EscapedText(escaped) => {
            set("encodedText", json!(escaped.encoded_text));
            set("decodedText", json!(escaped.decoded_text));
        }
        NodeRef::CodeSpan(code) => set("code", json!(code.code)),
        NodeRef::FencedCode(code) => {
            set("language", json!(code.language));
            set("code", json!(code.code));
        }
        NodeRef::HtmlStartTag(tag) => {
            set("name", json!(tag.name));
            set("selfClosingTag", json!(tag.self_closing_tag));
        }
        NodeRef::HtmlEndTag(tag) => set("name", json!(tag.name)),
        NodeRef::HtmlAttribute(attribute) => {
            set("name", json!(attribute.name));
            set("value", json!(attribute.value));
        }
        NodeRef::InlineTag(tag) => {
            set("tagName", json!(tag.base.tag_name));
            set("tagContent", json!(tag.tag_content));
        }
        NodeRef::LinkTag(link) => {
            set("tagName", json!(link.base.tag_name));
            if let Some(url) = &link.url_destination {
                set("urlDestination", json!(url));
            }
            if let Some(text) = &link.link_text {
                set("linkText", json!(text));
            }
        }
        NodeRef::InheritDocTag(tag) => set("tagName", json!(tag.base.tag_name)),
        NodeRef::ParamBlock(block) => set("parameterName", json!(block.parameter_name)),
        NodeRef::DeclarationReference(reference) => {
            if let Some(name) = &reference.package_name {
                set("packageName", json!(name));
            }
            if let Some(path) = &reference.import_path {
                set("importPath", json!(path));
            }
        }
        NodeRef::MemberIdentifier(identifier) => {
            set("identifier", json!(identifier.identifier));
            set("hasQuotes", json!(identifier.has_quotes()));
        }
        NodeRef::MemberReference(member) => set("hasDot", json!(member.has_dot)),
        NodeRef::MemberSelector(selector) => {
            set("selector", json!(selector.selector));
            set("selectorKind", json!(format!("{:?}", selector.selector_kind).to_lowercase()));
        }
        _ => {}
    }
    let children: Vec<Value> = node.child_nodes().into_iter().map(|child| self::node(context, utf16, child)).collect();
    if !children.is_empty() {
        item.insert("nodes".into(), Value::Array(children));
    }
    Value::Object(item)
}
