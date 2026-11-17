//! What upstream's snapshots hold, made from tsdocrs's results: Jest's pretty-format, TestHelpers'
//! doc node snapshots, escaped text and token coverage gaps.

use serde_json::Value as Json;
use tsdocrs::{
    TSDocConfiguration, TSDocParser, TSDocTagDefinition, TSDocTagSyntaxKind,
    nodes::NodeRef,
    parser::{ParserContext, TextRange, TokenKind, TokenSequence},
};

/// A value as Jest serializes it in a snapshot.
#[derive(Clone, Debug)]
pub enum Value {
    Undefined,
    Number(i64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(&'static str, Value)>),
    /// An instance of a class, which prints with the class's name.
    Instance(&'static str, Vec<(&'static str, Value)>),
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::String(s.into())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Undefined, Into::into)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(values: Vec<T>) -> Self {
        Self::Array(values.into_iter().map(Into::into).collect())
    }
}

/// `value` as `toMatchSnapshot()` stores it: pretty-format with `escapeString` and
/// `printBasicPrototype`, with a line break before and after when it takes more than one line.
pub fn serialize(value: &Value) -> String {
    let mut out = String::new();
    print(value, "", &mut out);
    if out.contains('\n') { format!("\n{out}\n") } else { out }
}

fn print(value: &Value, indent: &str, out: &mut String) {
    let inner = format!("{indent}  ");
    match value {
        Value::Undefined => out.push_str("undefined"),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => print_string(s, out),
        Value::Array(items) if items.is_empty() => out.push_str("Array []"),
        Value::Array(items) => {
            out.push_str("Array [\n");
            for item in items {
                out.push_str(&inner);
                print(item, &inner, out);
                out.push_str(",\n");
            }
            out.push_str(indent);
            out.push(']');
        }
        Value::Object(entries) => print(&Value::Instance("Object", entries.clone()), indent, out),
        Value::Instance(name, entries) if entries.is_empty() => out.push_str(&format!("{name} {{}}")),
        Value::Instance(name, entries) => {
            let mut entries: Vec<_> = entries.iter().collect();
            entries.sort_by_key(|(key, _)| *key);
            out.push_str(&format!("{name} {{\n"));
            for (key, value) in entries {
                out.push_str(&inner);
                print_string(key, out);
                out.push_str(": ");
                print(value, &inner, out);
                out.push_str(",\n");
            }
            out.push_str(indent);
            out.push('}');
        }
    }
}

fn print_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
}

/// TestHelpers.getEscaped: `s` with the characters that escape badly in a snapshot spelled out.
pub fn escaped(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\n' => out.push_str("[n]"),
            '\r' => out.push_str("[r]"),
            '\t' => out.push_str("[t]"),
            '\u{c}' => out.push_str("[f]"),
            '\\' => out.push_str("[b]"),
            '"' => out.push_str("[q]"),
            '`' => out.push_str("[c]"),
            '<' => out.push_str("[<]"),
            '>' => out.push_str("[>]"),
            c => out.push(c),
        }
    }
    out
}

/// TestHelpers.getDocNodeSnapshot: `node`, its excerpts' text and its children.
pub fn node_snapshot(context: Option<&ParserContext>, node: NodeRef) -> Value {
    let mut entries = vec![("kind", Value::from(node.kind().as_str()))];
    match node {
        NodeRef::Excerpt(excerpt) => {
            entries[0].1 = format!("Excerpt: {}", excerpt.excerpt_kind.as_str()).into();
            let text = context.map(|context| context.text(excerpt.content)).unwrap_or_default();
            entries.push(("nodeExcerpt", escaped(&text).into()));
        }
        NodeRef::PlainText(text) if text.text_excerpt.is_none() => {
            entries.push(("nodePlainText", escaped(&text.text).into()));
        }
        NodeRef::ErrorText(error) => {
            entries.push(("errorMessage", escaped(&error.error_message).into()));
            let context = context.expect("an error text comes from a parse");
            entries.push(("errorLocation", escaped(&context.text(error.error_location)).into()));
            if error.error_location.start_index > 0 {
                let preceding = context.tokens[error.error_location.start_index - 1];
                entries.push(("errorLocationPrecedingToken", preceding.text(context.buffer).into()));
            }
        }
        _ => {}
    }
    let children = node.child_nodes();
    if !children.is_empty() {
        entries
            .push(("nodes", Value::Array(children.into_iter().map(|child| node_snapshot(context, child)).collect())));
    }
    Value::Object(entries)
}

/// TokenCoverageChecker.getGaps: the runs of tokens no excerpt under `root` covers, or the two
/// excerpts that overlap.
pub fn coverage_gaps(context: &ParserContext, root: NodeRef) -> Result<Vec<String>, String> {
    let mut covered: Vec<Option<(&'static str, TokenSequence)>> = vec![None; context.tokens.len()];
    let format = |(kind, sequence): (&str, TokenSequence)| format!("{kind} ({:?})", context.text(sequence));
    let mut stack = vec![root];
    let mut order = Vec::new();
    while let Some(node) = stack.pop() {
        order.push(node);
        stack.extend(node.child_nodes().into_iter().rev());
    }
    for node in order {
        if let NodeRef::Excerpt(excerpt) = node {
            let association = ("Excerpt", excerpt.content);
            for slot in &mut covered[excerpt.content.start_index..excerpt.content.end_index] {
                if let Some(existing) = *slot {
                    return Err(format!(
                        "Overlapping content encountered between {} and {}",
                        format(existing),
                        format(association)
                    ));
                }
                *slot = Some(association);
            }
        }
    }
    let last = context.tokens.len() - 1;
    assert_eq!(context.tokens[last].kind, TokenKind::EndOfInput, "Missing EndOfInput token");
    let mut gaps = Vec::new();
    let mut gap_start: Option<usize> = None;
    for (index, slot) in covered.iter().enumerate().take(last) {
        match (gap_start, slot) {
            (None, None) => gap_start = Some(index),
            (Some(start), Some(_)) => {
                gaps.push(TokenSequence::new(start, index));
                gap_start = None;
            }
            _ => {}
        }
    }
    // As in TSDoc, a gap from the first token isn't checked for here.
    if let Some(start) = gap_start.filter(|&start| start != 0) {
        gaps.push(TokenSequence::new(start, context.tokens.len()));
    }
    Ok(gaps.into_iter().map(|gap| context.text(gap).into_owned()).collect())
}

/// The configuration a recorded `parse` event's `configuration` describes.
pub fn configuration(json: &Json) -> TSDocConfiguration {
    let mut configuration = TSDocConfiguration::new();
    configuration.clear(true);
    for tag in json["tagDefinitions"].as_array().expect("tagDefinitions") {
        let kind = match tag["syntaxKind"].as_str() {
            Some("inline") => TSDocTagSyntaxKind::InlineTag,
            Some("block") => TSDocTagSyntaxKind::BlockTag,
            _ => TSDocTagSyntaxKind::ModifierTag,
        };
        let name = tag["tagName"].as_str().expect("tagName");
        let definition = TSDocTagDefinition::new(name, kind, tag["allowMultiple"].as_bool() == Some(true)).unwrap();
        configuration.add_tag_definition(definition).unwrap();
    }
    for name in json["supportedTagDefinitions"].as_array().expect("supportedTagDefinitions") {
        configuration.set_support_for_tag(name.as_str().unwrap(), true).unwrap();
    }
    let html = json["supportedHtmlElements"].as_array().expect("supportedHtmlElements");
    configuration.set_supported_html_elements(html.iter().map(|name| name.as_str().unwrap().to_owned()));
    let validation = &json["validation"];
    configuration.validation.ignore_undefined_tags = validation["ignoreUndefinedTags"].as_bool().unwrap();
    configuration.validation.report_unsupported_tags = validation["reportUnsupportedTags"].as_bool().unwrap();
    configuration.validation.report_unsupported_html_elements =
        validation["reportUnsupportedHtmlElements"].as_bool().unwrap();
    configuration
}

/// Parses `text` with the configuration `json` describes.
pub fn parse<'a>(text: &'a str, json: &Json) -> Result<ParserContext<'a>, String> {
    TSDocParser::new(&configuration(json)).parse_string(text).map_err(|e| format!("tsdocrs failed: {e}"))
}

/// The text of each of `context`'s lines, escaped.
pub fn lines(context: &ParserContext) -> Value {
    context.lines.iter().map(|&line| escaped(context.range_text(line))).collect::<Vec<_>>().into()
}

/// The text of each of `context`'s messages, with its line and column.
pub fn log_messages(context: &ParserContext) -> Value {
    context.log.iter().map(|message| message.text.clone()).collect::<Vec<_>>().into()
}

/// TestHelpers.formatLineSpan: `range` within `line`, marked with `>` and `<` under the line as
/// [`escaped`] spells it.
pub fn format_line_span(buffer: &str, line: TextRange, range: TextRange) -> String {
    const SPACE: [&str; 5] = ["", " ", "  ", "   ", "    "];
    const LEFT: [&str; 5] = ["", ">", " >", "  >", "   >"];
    const RIGHT: [&str; 5] = ["", "<", "< ", "<  ", "<   "];
    // Each character's escaped width, by its index in the buffer; TSDoc indexes UTF-16 code
    // units, and the suite's buffers are ASCII.
    let width = |at: usize| buffer.get(at..at + 1).map_or(0, |c| escaped(c).len());
    let mut span = String::new();
    if line.end == 0 {
        return span;
    }
    let mut i = line.pos as isize - 1;
    while i < range.pos as isize - 1 {
        span += SPACE[width(i as usize)];
        i += 1;
    }
    span += LEFT[usize::try_from(i).map_or(0, width)];
    i += 1;
    while i < range.end as isize {
        span += SPACE[width(i as usize)];
        i += 1;
    }
    if i == line.end as isize {
        span += "<";
    } else {
        span += RIGHT[width(i as usize)];
        i += 1;
        while i < line.end as isize {
            span += SPACE[width(i as usize)];
            i += 1;
        }
    }
    span
}

/// The `expected` of `test`'s snapshot or inline snapshot number `n`, from 0.
pub fn expected(test: &crate::harness::Test, n: usize) -> Result<&str, String> {
    use crate::harness::Event;
    test.events
        .iter()
        .filter_map(|event| match event {
            Event::Snapshot { expected, .. } | Event::InlineSnapshot { expected } => Some(expected.as_str()),
            _ => None,
        })
        .nth(n)
        .ok_or_else(|| format!("no snapshot {n}"))
}

/// The text and configuration of `test`'s parse number `n`, from 0.
pub fn parse_event(test: &crate::harness::Test, n: usize) -> Result<(&str, &Json), String> {
    use crate::harness::Event;
    test.events
        .iter()
        .filter_map(|event| match event {
            Event::Parse { text, configuration } => Some((text.as_str(), configuration)),
            _ => None,
        })
        .nth(n)
        .ok_or_else(|| format!("no parse {n}"))
}

/// Fails with `what` unless `ok`.
pub fn check(ok: bool, what: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(format!("expected {what}")) }
}

/// Compares a snapshot tsdocrs made with upstream's.
pub fn compare(expected: &str, actual: &Value) -> Result<(), String> {
    let actual = serialize(actual);
    if actual == expected {
        return Ok(());
    }
    let first = expected.lines().zip(actual.lines()).position(|(e, a)| e != a).unwrap_or(0);
    let context = |s: &str| s.lines().skip(first.saturating_sub(3)).take(8).collect::<Vec<_>>().join("\n");
    Err(format!(
        "snapshot differs at line {}:\n--- upstream\n{}\n--- tsdocrs\n{}",
        first + 1,
        context(expected),
        context(&actual)
    ))
}
