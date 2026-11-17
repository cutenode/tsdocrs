//! What tools/oracle/oracle.mjs reports for a comment, made from tsdocrs's parse of it, so the two
//! compare as JSON: the lines, the messages with UTF-8 offsets, the tree as TestHelpers snapshots
//! it, unescaped, and the comment emitted again as TSDoc.

// Each test target uses only some of this.
#![allow(dead_code)]

pub mod generate;

use std::fs;

use serde_json::{Map, Value, json};
use tsdocrs::{
    TSDocConfiguration, TSDocParser,
    config_file::{self, ConfigError},
    nodes::NodeRef,
    parser::ParserContext,
};

/// The configuration a tsdoc.json with `keys` sets up.
pub fn configuration(keys: Option<&Value>) -> Result<TSDocConfiguration, ConfigError> {
    let Some(Value::Object(keys)) = keys else { return Ok(TSDocConfiguration::new()) };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(config_file::FILE_NAME);
    let mut file = keys.clone();
    file.insert("$schema".into(), json!(config_file::SCHEMA));
    fs::write(&path, Value::Object(file).to_string()).unwrap();
    config_file::load(&path).map(|(configuration, _)| configuration)
}

/// What the oracle would report for `comment` read with the tsdoc.json keys `config`.
pub fn report(comment: &str, config: Option<&Value>) -> Value {
    let Ok(configuration) = configuration(config) else { return json!({ "configError": true }) };
    match TSDocParser::new(&configuration).parse_string(comment) {
        Ok(context) => json!({
            "lines": context.lines.iter().map(|&line| context.range_text(line)).collect::<Vec<_>>(),
            "messages": context.log.iter().map(|message| json!({
                "id": message.message_id.as_str(),
                "text": message.text,
                "pos": message.text_range.pos,
                "end": message.text_range.end,
            })).collect::<Vec<_>>(),
            "nodes": snapshot(&context, NodeRef::Comment(&context.doc_comment)),
            "emitted": context.doc_comment.emit_as_tsdoc(),
        }),
        Err(error) => json!({ "throws": error.message }),
    }
}

/// The oracle's report with a config error's text left out, which tsdocrs words its own way.
pub fn comparable(mut report: Value) -> Value {
    if let Some(error) = report.get_mut("configError") {
        *error = json!(true);
    }
    report
}

fn snapshot(context: &ParserContext, node: NodeRef) -> Value {
    let mut item = Map::new();
    item.insert("kind".into(), json!(node.kind().as_str()));
    match node {
        NodeRef::Excerpt(excerpt) => {
            item.insert("kind".into(), json!(format!("Excerpt: {}", excerpt.excerpt_kind.as_str())));
            item.insert("excerpt".into(), json!(context.text(excerpt.content)));
        }
        NodeRef::PlainText(text) if text.text_excerpt.is_none() => {
            item.insert("plainText".into(), json!(text.text));
        }
        NodeRef::ErrorText(error) => {
            item.insert("errorMessage".into(), json!(error.error_message));
            item.insert("errorLocation".into(), json!(context.text(error.error_location)));
            if let Some(before) = error.error_location.start_index.checked_sub(1) {
                item.insert("errorLocationPrecedingToken".into(), json!(context.tokens[before].text(context.buffer)));
            }
        }
        _ => {}
    }
    let children = node.child_nodes();
    if !children.is_empty() {
        let nodes: Vec<Value> = children.into_iter().map(|child| snapshot(context, child)).collect();
        item.insert("nodes".into(), Value::Array(nodes));
    }
    Value::Object(item)
}
