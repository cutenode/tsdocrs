//! The snapshots the harness makes itself: TestHelpers' two, and the ones the Tokenizer,
//! LineExtractor, TextRange and TSDocEmitter tests build, from the input the test parsed.

use tsdocrs::{
    nodes::NodeRef,
    parser::{TextRange, TokenKind},
};

use crate::{
    harness::{Event, Test},
    snapshot::{Value, compare, coverage_gaps, escaped, format_line_span, lines, log_messages, node_snapshot, parse},
};

/// Checks the snapshot at `index` of `test`'s events, or none when only a port can.
pub fn snapshot(path: &str, test: &Test, index: usize) -> Option<Result<(), String>> {
    let before = &test.events[..index];
    let expected = match &test.events[index] {
        Event::Snapshot { expected, .. } | Event::InlineSnapshot { expected } => expected,
        _ => unreachable!("only snapshots are checked here"),
    };
    let helper = match &test.events[index] {
        Event::Snapshot { helper, .. } => helper.as_deref(),
        _ => None,
    };
    let file = path.rsplit('/').next().unwrap_or(path);
    let last_parse = before.iter().rev().find_map(|event| match event {
        Event::Parse { text, configuration } => Some((text.as_str(), configuration)),
        _ => None,
    });
    let value = match (helper, file) {
        (Some("parseAndMatchNodeParserSnapshot"), _) => node_parser(last_parse?),
        (Some("parseAndMatchDocCommentSnapshot"), _) => doc_comment(last_parse?),
        (Some(_), _) => return None,
        (None, "LineExtractor.test.ts") => line_extractor(last_parse?),
        (None, "Tokenizer.test.ts") => tokenizer(last_parse?),
        (None, "TextRange.test.ts") if test.template != "construction scenarios" => text_range(before),
        (None, "TSDocEmitter.test.ts") if parsed_since_last_snapshot(before) => emitted(last_parse?),
        _ => return None,
    };
    Some(value.and_then(|value| compare(expected, &value)))
}

type Input<'a> = (&'a str, &'a serde_json::Value);

/// Whether the test parsed something after its last snapshot.
fn parsed_since_last_snapshot(before: &[Event]) -> bool {
    let last = |f: fn(&Event) -> bool| before.iter().rposition(f);
    let parse = last(|event| matches!(event, Event::Parse { .. }));
    let snapshot = last(|event| matches!(event, Event::Snapshot { .. } | Event::InlineSnapshot { .. }));
    parse.is_some_and(|parse| snapshot.is_none_or(|snapshot| parse > snapshot))
}

/// TSDocEmitter.test.ts's createSnapshot: the messages, and the comment emitted again.
fn emitted((text, configuration): Input) -> Result<Value, String> {
    let context = parse(text, configuration)?;
    Ok(Value::Object(vec![
        ("errors", log_messages(&context)),
        ("output", format!("\n{}", context.doc_comment.emit_as_tsdoc()).into()),
    ]))
}

/// TestHelpers.parseAndMatchNodeParserSnapshot.
fn node_parser((text, configuration): Input) -> Result<Value, String> {
    let context = parse(text, configuration)?;
    let comment = NodeRef::Comment(&context.doc_comment);
    Ok(Value::Object(vec![
        ("buffer", escaped(text).into()),
        ("lines", lines(&context)),
        ("logMessages", log_messages(&context)),
        ("nodes", node_snapshot(Some(&context), comment)),
        ("gaps", coverage_gaps(&context, comment)?.into()),
    ]))
}

/// TestHelpers.parseAndMatchDocCommentSnapshot.
fn doc_comment((text, configuration): Input) -> Result<Value, String> {
    let context = parse(text, configuration)?;
    let comment = &context.doc_comment;
    let snapshot = |node: NodeRef| node_snapshot(Some(&context), node);
    let block = |block: &Option<tsdocrs::nodes::DocBlock>| block.as_ref().map(|block| snapshot(NodeRef::Block(block)));
    let params = |blocks: &[tsdocrs::nodes::DocParamBlock]| {
        Value::Array(blocks.iter().map(|block| snapshot(NodeRef::ParamBlock(block))).collect())
    };
    Ok(Value::Object(vec![
        ("s00_lines", lines(&context)),
        ("s01_gaps", coverage_gaps(&context, NodeRef::Comment(comment))?.into()),
        ("s02_summarySection", snapshot(NodeRef::Section(&comment.summary_section))),
        ("s03_remarksBlock", block(&comment.remarks_block).into()),
        ("s04_privateRemarksBlock", block(&comment.private_remarks).into()),
        ("s05_deprecatedBlock", block(&comment.deprecated_block).into()),
        ("s06_paramBlocks", params(&comment.params.blocks)),
        ("s07_typeParamBlocks", params(&comment.type_params.blocks)),
        ("s08_returnsBlock", block(&comment.returns_block).into()),
        (
            "s09_customBlocks",
            Value::Array(comment.custom_blocks.iter().map(|block| snapshot(NodeRef::Block(block))).collect()),
        ),
        ("s10_inheritDocTag", comment.inherit_doc_tag.as_ref().map(|tag| snapshot(NodeRef::InheritDocTag(tag))).into()),
        (
            "s11_modifierTags",
            Value::Array(comment.modifier_tag_set.nodes.iter().map(|tag| snapshot(NodeRef::BlockTag(tag))).collect()),
        ),
        ("s12_logMessages", log_messages(&context)),
    ]))
}

/// LineExtractor.test.ts's parseAndMatchSnapshot.
fn line_extractor((text, configuration): Input) -> Result<Value, String> {
    let context = parse(text, configuration)?;
    Ok(Value::Object(vec![
        ("buffer", escaped(text).into()),
        ("comment", escaped(context.range_text(context.comment_range)).into()),
        ("lines", lines(&context)),
        ("logMessages", log_messages(&context)),
    ]))
}

/// Tokenizer.test.ts's matchSnapshot.
fn tokenizer((text, configuration): Input) -> Result<Value, String> {
    let context = parse(text, configuration)?;
    let mut tokens = Vec::new();
    for token in &context.tokens {
        let line = token.line(&context.lines);
        let index_of_line = context.lines.iter().position(|&each| each == line).map_or(-1, |at| at as i64);
        tokens.push(Value::Object(vec![
            ("indexOfLine", Value::Number(index_of_line)),
            ("line", format!(">{}<", escaped(context.range_text(line))).into()),
            ("span", format_line_span(context.buffer, line, token.range()).into()),
            ("tokenKind", token.kind.as_str().into()),
        ]));
        if token.kind == TokenKind::EndOfInput {
            break;
        }
    }
    Ok(Value::Object(vec![("buffer", escaped(text).into()), ("tokens", Value::Array(tokens))]))
}

/// TextRange.test.ts's matchSnapshot, which snapshots each index from -1 to one past the end in
/// turn: this is the one for the snapshots already taken since `TextRange.fromString`.
fn text_range(before: &[Event]) -> Result<Value, String> {
    let (start, buffer) = before
        .iter()
        .enumerate()
        .rev()
        .find_map(|(at, event)| match event {
            Event::TextRange { text } => Some((at, text)),
            _ => None,
        })
        .ok_or("no TextRange.fromString before the snapshot")?;
    let taken = before[start..].iter().filter(|event| matches!(event, Event::Snapshot { .. })).count();
    let i = taken as isize - 1;
    let from = i.max(0) as usize;
    let substr = |length: usize| buffer.get(from..).map_or("", |rest| &rest[..rest.len().min(length)]);
    let location = TextRange::location(buffer, i);
    Ok(Value::Object(vec![
        ("c", escaped(substr(1)).into()),
        ("context", escaped(substr(10)).into()),
        ("i", Value::Number(i as i64)),
        (
            "location",
            Value::Object(vec![
                ("column", Value::Number(location.column as i64)),
                ("line", Value::Number(location.line as i64)),
            ]),
        ),
    ]))
}
