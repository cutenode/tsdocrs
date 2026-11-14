//! Differential tests: Hegel draws TSDoc-shaped comments and tsdoc.json settings, and tsdocrs must
//! make of each exactly what `@microsoft/tsdoc` does, as tools/oracle/oracle.mjs reports it: the
//! same lines, messages (ids, texts, ranges) and tree, or the same error where it throws. The
//! oracle needs Node and `npm ci`; without them these fail rather than pass untested.

mod common;

use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{Mutex, OnceLock},
};

use hegel::{TestCase, generators as gs};
use serde_json::{Value, json};

/// The oracle, started once for the whole run.
struct Oracle {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

fn oracle() -> &'static Mutex<Oracle> {
    static ORACLE: OnceLock<Mutex<Oracle>> = OnceLock::new();
    ORACLE.get_or_init(|| {
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/oracle/oracle.mjs");
        let mut child = Command::new("node")
            .arg(script)
            .arg("--serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("the differential tests run @microsoft/tsdoc with node: install Node and run `npm ci`");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Mutex::new(Oracle { _child: child, stdin, stdout })
    })
}

/// What `@microsoft/tsdoc` makes of `comment` with the tsdoc.json keys `config`.
fn microsoft_tsdoc(comment: &str, config: Option<&Value>) -> Value {
    let mut oracle = oracle().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let request = json!({ "comment": comment, "config": config });
    writeln!(oracle.stdin, "{request}").unwrap();
    oracle.stdin.flush().unwrap();
    let mut line = String::new();
    oracle.stdout.read_line(&mut line).unwrap();
    assert!(!line.is_empty(), "the oracle exited; is @microsoft/tsdoc installed (`npm ci`)?");
    common::comparable(serde_json::from_str(&line).unwrap())
}

fn assert_same(comment: &str, config: Option<&Value>) {
    let expected = microsoft_tsdoc(comment, config);
    let actual = common::report(comment, config);
    assert!(
        actual == expected,
        "{comment:?} with {config:?}\n@microsoft/tsdoc: {}\ntsdocrs:          {}",
        serde_json::to_string_pretty(&expected).unwrap(),
        serde_json::to_string_pretty(&actual).unwrap()
    );
}

/// Pieces of TSDoc, and of the mistakes the parser reports, that comments are drawn from.
const PIECES: &[&str] = &[
    "text", "Foo", "x", "42", "é", "日本", "😀", " ", "  ", "\t", "-", " - ", ".", ",", ":", ";", "#", "$", "_", "=", "+",
    "|", "*", "/", "//", "'", "\"", "(", ")", "[", "]", "{", "}", "<", ">", "@", "\\", "`", "``", "```", "\\@",
    "\\{", "\\}", "\\a", "@param", "@param x -", "@param [x] -", "@param {string} x -", "@param x.y -",
    "@typeParam T -", "@remarks", "@privateRemarks", "@deprecated", "@returns", "@see", "@example", "@throws",
    "@defaultValue", "@public", "@beta", "@alpha", "@internal", "@sealed", "@virtual", "@override",
    "@packageDocumentation", "@custom", "@mine", "@bogus", "@REMARKS", "@a1", "@1a", "@x+", "{@link",
    "{@link Foo}", "{@link Foo.bar}", "{@link Foo | text}", "{@link pkg#Foo}", "{@link @scope/pkg#Foo.(bar:static)}",
    "{@link ./path#x}", "{@link #x}", "{@link (Foo:class)}", "{@link Foo.(bar:1)}", "{@link Foo.(bar:LABEL)}",
    "{@link [Symbol.iterator]}", "{@link \"quoted name\"}", "{@link http://example.com}",
    "{@link https://example.com | text}", "{@link a:b}", "{@link}", "{@link |}", "{@inheritDoc}",
    "{@inheritDoc Foo}", "{@inheritDoc pkg#Foo.bar}", "{@label X}", "{@custom x}", "{@mine}", "{@bogus}",
    "{@ x}", "{@x~}", "{@", "<a>", "</a>", "<b>", "</b>", "<br/>", "<a href=\"x\">", "<a href='x' title=\"y\">",
    "<a href=x>", "<a href=\"x>", "<my-el>", "<a.b>", "< a>", "</ a>", "<a", "</a", "`code`", "``", "```ts",
];

/// A comment drawn from [`PIECES`]: lines of them, with or without each line's `*`.
fn comment(tc: &TestCase) -> String {
    let mut comment = String::from(tc.draw(gs::sampled_from(vec!["/**", "/** ", "/**\n", " /**"])));
    for line in 0..tc.draw(gs::integers::<usize>().max_value(6)) {
        if line > 0 || tc.draw(gs::booleans()) {
            comment.push('\n');
            comment += tc.draw(gs::sampled_from(vec![" * ", " *", "*", "", "   ", " *   "]));
        }
        for _ in 0..tc.draw(gs::integers::<usize>().max_value(6)) {
            comment += tc.draw(gs::sampled_from(PIECES.to_vec()));
            if tc.draw(gs::booleans()) {
                comment.push(' ');
            }
        }
    }
    comment += tc.draw(gs::sampled_from(vec!["\n */", " */", "*/", "\n*/", "\r\n */", ""]));
    comment
}

/// tsdoc.json keys: tags it defines, which it supports, and its HTML elements.
fn config(tc: &TestCase) -> Option<Value> {
    if tc.draw(gs::booleans()) {
        return None;
    }
    let mut keys = serde_json::Map::new();
    if tc.draw(gs::booleans()) {
        keys.insert("noStandardTags".into(), json!(tc.draw(gs::booleans())));
    }
    let names = tc.draw(gs::vecs(gs::sampled_from(vec!["@custom", "@mine", "@Custom"])).max_size(2));
    let tags: Vec<Value> = names
        .iter()
        .map(|name| {
            let kind = tc.draw(gs::sampled_from(vec!["inline", "block", "modifier"]));
            json!({ "tagName": name, "syntaxKind": kind, "allowMultiple": tc.draw(gs::booleans()) })
        })
        .collect();
    if !tags.is_empty() {
        keys.insert("tagDefinitions".into(), Value::Array(tags));
    }
    let support = tc.draw(gs::vecs(gs::sampled_from(vec!["@custom", "@remarks", "@link", "@param", "@public"])).max_size(3));
    if !support.is_empty() {
        let support = support.into_iter().map(|name| (name.to_owned(), json!(tc.draw(gs::booleans()))));
        keys.insert("supportForTags".into(), Value::Object(support.collect()));
    }
    if tc.draw(gs::booleans()) {
        let html = tc.draw(gs::vecs(gs::sampled_from(vec!["a", "b", "br", "my-el"])).max_size(3));
        keys.insert("supportedHtmlElements".into(), json!(html));
    }
    if tc.draw(gs::booleans()) {
        keys.insert("reportUnsupportedHtmlElements".into(), json!(tc.draw(gs::booleans())));
    }
    Some(Value::Object(keys))
}

/// Comments built from TSDoc's syntax and its mistakes, read with drawn tsdoc.json settings.
#[hegel::test(test_cases = 500)]
fn tsdoc_shaped_comments_parse_as_microsoft_tsdoc_parses_them(tc: TestCase) {
    let comment = comment(&tc);
    let config = config(&tc);
    assert_same(&comment, config.as_ref());
}

/// Any text at all, inside a comment's delimiters or not.
#[hegel::test(test_cases = 300)]
fn any_text_parses_as_microsoft_tsdoc_parses_it(tc: TestCase) {
    let text = tc.draw(gs::text());
    let wrapped = if tc.draw(gs::booleans()) { format!("/**{text}*/") } else { text };
    assert_same(&wrapped, None);
}
