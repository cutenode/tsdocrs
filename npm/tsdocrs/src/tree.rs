//! [`parse_trees`]: many comments' trees as numbers in two shared `Uint32Array`s, which index.js
//! reads through lazy objects, so JavaScript builds only the nodes it visits instead of parsing the
//! JSON of every one.
//!
//! `nodes` holds a record of [`RECORD`] numbers per node, each comment's in document order: its
//! kind, the start and end of an excerpt's text in UTF-16 code units (0 for the other kinds), its
//! first child and its next sibling ([`NONE`] when there's none), and where its fields start in
//! `fields`. There a count comes first, then four numbers per field: its name, how its value is
//! held, and the value: [`STRING`] and an index into `strings`, [`SLICE`] and the start and end of
//! the comment's text it is, or [`BOOL`] and 0 or 1. Kinds and names are indices into `strings`.

use serde::Serialize;
use tsdocrs::{
    ParserContext, TSDocParser,
    nodes::NodeRef,
    parser::{TextRange, TokenKind},
};
use wasm_bindgen::prelude::*;

use crate::{
    Lines, Messages, Utf16,
    fields::{self, Value},
    with_configuration,
};

const RECORD: usize = 6;
const NONE: u32 = u32::MAX;
const STRING: u32 = 0;
const SLICE: u32 = 1;
const BOOL: u32 = 2;

/// Parses each of `texts` as `parse` does, with one configuration, into one set of buffers. A
/// comment `@microsoft/tsdoc` throws on has its error message in place of its tree.
#[wasm_bindgen(js_name = parseTrees, unchecked_return_type = "Trees")]
pub fn parse_trees(
    texts: Vec<String>,
    #[wasm_bindgen(unchecked_param_type = "TSDocConfigFileJson | undefined")] configuration: JsValue,
) -> Result<JsValue, JsError> {
    let (json, writer) = with_configuration(configuration, |configuration| {
        let parser = TSDocParser::new(configuration);
        let mut writer = Writer::default();
        let mut json = String::from("{\"comments\":[");
        for (at, text) in texts.iter().enumerate() {
            if at > 0 {
                json.push(',');
            }
            match parser.parse_string(text) {
                Ok(context) => {
                    let utf16 = Utf16::new(context.buffer);
                    let root = writer.node(&context, &utf16, NodeRef::Comment(&context.doc_comment));
                    let comment = Comment {
                        lines: Lines(&context),
                        messages: Messages { context: &context, utf16: &utf16 },
                        root,
                    };
                    json += &serde_json::to_string(&comment)?;
                }
                Err(error) => json += &serde_json::to_string(&Failed { error: &error.message })?,
            }
        }
        json += "],\"strings\":";
        json += &serde_json::to_string(&writer.strings)?;
        json.push('}');
        Ok((json, writer))
    })?;
    let trees = js_sys::JSON::parse(&json).expect_throw("serde_json writes valid JSON");
    js_sys::Reflect::set(&trees, &"nodes".into(), &js_sys::Uint32Array::from(&writer.nodes[..]))
        .expect_throw("trees is an object");
    js_sys::Reflect::set(&trees, &"fields".into(), &js_sys::Uint32Array::from(&writer.fields[..]))
        .expect_throw("trees is an object");
    Ok(trees)
}

#[derive(Serialize)]
struct Comment<'a> {
    lines: Lines<'a>,
    messages: Messages<'a>,
    root: u32,
}

#[derive(Serialize)]
struct Failed<'a> {
    error: &'a str,
}

/// The buffers, as they're written.
#[derive(Default)]
struct Writer {
    nodes: Vec<u32>,
    fields: Vec<u32>,
    strings: Vec<String>,
    /// Each kind and field name written so far, with its index in `strings`. There are a few
    /// dozen, so a list is as quick as a map.
    names: Vec<(&'static str, u32)>,
}

impl Writer {
    /// Writes `node` and the nodes under it, and gives its index.
    fn node(&mut self, context: &ParserContext, utf16: &Utf16, node: NodeRef) -> u32 {
        let index = self.nodes.len() / RECORD;
        let kind = self.name(node.kind().as_str());
        let fields_at = self.fields.len();
        self.fields.push(0);
        let (mut pos, mut end) = (0, 0);
        if let NodeRef::Excerpt(excerpt) = node {
            (pos, end) = utf16.range(excerpt.content.containing_text_range(&context.tokens));
            self.field(context, utf16, "excerptKind", Value::Text(excerpt.excerpt_kind.as_str()));
            // An excerpt across lines has text of its own; the others' text is a slice.
            let tokens = excerpt.content.tokens(&context.tokens);
            if tokens.iter().any(|token| token.kind == TokenKind::Newline) {
                let content = context.text(excerpt.content);
                self.field(context, utf16, "content", Value::Text(&content));
            }
        }
        fields::for_each(node, |name, value| self.field(context, utf16, name, value));
        self.fields[fields_at] = ((self.fields.len() - fields_at - 1) / 4) as u32;
        self.nodes.extend([kind, pos, end, NONE, NONE, fields_at as u32]);
        let mut slot = index * RECORD + 3;
        for child in node.child_nodes() {
            let child = self.node(context, utf16, child);
            self.nodes[slot] = child;
            slot = child as usize * RECORD + 4;
        }
        index as u32
    }

    fn field(&mut self, context: &ParserContext, utf16: &Utf16, name: &'static str, value: Value) {
        let name = self.name(name);
        let (how, a, b) = match value {
            Value::Bool(value) => (BOOL, u32::from(value), 0),
            Value::Text(text) => match slice_of(context.buffer, text) {
                Some(range) => {
                    let (pos, end) = utf16.range(range);
                    (SLICE, pos, end)
                }
                None => {
                    self.strings.push(text.to_owned());
                    (STRING, (self.strings.len() - 1) as u32, 0)
                }
            },
        };
        self.fields.extend([name, how, a, b]);
    }

    fn name(&mut self, name: &'static str) -> u32 {
        if let Some(&(_, index)) = self.names.iter().find(|(each, _)| *each == name) {
            return index;
        }
        self.strings.push(name.to_owned());
        let index = (self.strings.len() - 1) as u32;
        self.names.push((name, index));
        index
    }
}

/// Where `text` is in `buffer`, when it's a slice of it, as the parser's text mostly is.
fn slice_of(buffer: &str, text: &str) -> Option<TextRange> {
    let pos = (text.as_ptr() as usize).checked_sub(buffer.as_ptr() as usize)?;
    let end = pos + text.len();
    (end <= buffer.len()).then(|| TextRange::new(pos, end))
}
