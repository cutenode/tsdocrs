//! TSDoc-shaped comments and tsdoc.json settings for Hegel to draw, which the differential tests
//! and the properties share.

use hegel::{TestCase, generators as gs};
use serde_json::{Value, json};

/// Pieces of TSDoc, and of the mistakes the parser reports, that comments are drawn from.
const PIECES: &[&str] = &[
    "text",
    "Foo",
    "x",
    "42",
    "é",
    "日本",
    "😀",
    " ",
    "  ",
    "\t",
    "-",
    " - ",
    ".",
    ",",
    ":",
    ";",
    "#",
    "$",
    "_",
    "=",
    "+",
    "|",
    "*",
    "/",
    "//",
    "'",
    "\"",
    "(",
    ")",
    "[",
    "]",
    "{",
    "}",
    "<",
    ">",
    "@",
    "\\",
    "`",
    "``",
    "```",
    "\\@",
    "\\{",
    "\\}",
    "\\a",
    "@param",
    "@param x -",
    "@param [x] -",
    "@param {string} x -",
    "@param x.y -",
    "@typeParam T -",
    "@remarks",
    "@privateRemarks",
    "@deprecated",
    "@returns",
    "@see",
    "@example",
    "@throws",
    "@defaultValue",
    "@public",
    "@beta",
    "@alpha",
    "@internal",
    "@sealed",
    "@virtual",
    "@override",
    "@packageDocumentation",
    "@custom",
    "@mine",
    "@bogus",
    "@REMARKS",
    "@a1",
    "@1a",
    "@x+",
    "{@link",
    "{@link Foo}",
    "{@link Foo.bar}",
    "{@link Foo | text}",
    "{@link pkg#Foo}",
    "{@link @scope/pkg#Foo.(bar:static)}",
    "{@link ./path#x}",
    "{@link #x}",
    "{@link (Foo:class)}",
    "{@link Foo.(bar:1)}",
    "{@link Foo.(bar:LABEL)}",
    "{@link [Symbol.iterator]}",
    "{@link \"quoted name\"}",
    "{@link http://example.com}",
    "{@link https://example.com | text}",
    "{@link a:b}",
    "{@link}",
    "{@link |}",
    "{@inheritDoc}",
    "{@inheritDoc Foo}",
    "{@inheritDoc pkg#Foo.bar}",
    "{@label X}",
    "{@custom x}",
    "{@mine}",
    "{@bogus}",
    "{@ x}",
    "{@x~}",
    "{@",
    "<a>",
    "</a>",
    "<b>",
    "</b>",
    "<br/>",
    "<a href=\"x\">",
    "<a href='x' title=\"y\">",
    "<a href=x>",
    "<a href=\"x>",
    "<my-el>",
    "<a.b>",
    "< a>",
    "</ a>",
    "<a",
    "</a",
    "`code`",
    "``",
    "```ts",
];

/// A comment drawn from [`PIECES`]: lines of them, with or without each line's `*`.
pub fn comment(tc: &TestCase) -> String {
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
pub fn config(tc: &TestCase) -> Option<Value> {
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
    let support =
        tc.draw(gs::vecs(gs::sampled_from(vec!["@custom", "@remarks", "@link", "@param", "@public"])).max_size(3));
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
