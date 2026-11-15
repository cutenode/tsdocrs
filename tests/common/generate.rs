//! TSDoc-shaped comments and tsdoc.json settings for Hegel to draw, which the differential tests
//! and the properties share.

use hegel::{TestCase, generators as gs};
use serde_json::{Value, json};

/// Pieces of TSDoc, and of the mistakes the parser reports, that comments are drawn from.
pub const PIECES: &[&str] = &[
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
    "@link",
    "{@remarks}",
    "{@link (Foo:}",
    "{@link Foo.(bar:01)}",
    "{@link Foo.(bar:Lab)}",
    "{@link Foo.(bar:x1)}",
    "{@link Foo.(bar:nope)}",
    "{@link Foo.(bar:static) | text  }",
    "{@link Foo.(bar)}",
    "{@link Foo:bar}",
    "{@link Foo.bar baz}",
    "{@link Foo |}",
    "{@link Foo | a | b}",
    "{@link Foo | {x}",
    "{@link @scope/pkg/sub/path#Foo}",
    "{@link a/b#}",
    "{@link /x#Foo}",
    "{@link x//y#Foo}",
    "{@link x/#Foo}",
    "{@link a b#Foo}",
    "{@link #}",
    "{@link Foo..bar}",
    "{@link Foo.[x}",
    "{@link Foo.[]}",
    "{@link [x}",
    "{@link (Foo:static}",
    "{@link Foo.\"\"}",
    "{@link Foo.\"a}",
    "{@link Foo.static}",
    "{@link Foo.1x}",
    "{@link Foo.$x}",
    "{@link ftp:}",
    "{@link ht+tp://x}",
    "{@link 1http://x}",
    "{@link http://}",
    "{@inheritDoc Foo bar}",
    "{@inheritDoc Foo#}",
    "{@inheritDoc a/b}",
    "{@a \\b}",
    "{@a \\}}",
    "{@a \\x}",
    "@param {\"}\" x} y -",
    "@param {'a\\'b'} x -",
    "@param {`x`} x -",
    "@param [x=\"]\"] -",
    "@param [x=[1, [2]]] -",
    "@param [x -",
    "@param {x -",
    "@param 1x -",
    "@param x.y$z -",
    "@param x - {type}",
    "@param x {type} -",
    "@param x",
    "@a\"",
    "@a\\",
    "@a\u{1}",
    "@a\r",
    "@a\u{c}",
    "@a\u{8}",
    "@a😀",
    "@a\t",
    "<a b=\"c\"d>",
    "<a b=\"c\" >",
    "<a b = \"c\" />",
    "<a b>",
    "<a b=c>",
    "<a b=\"c>",
    "<a/x>",
    "<1a>",
    "</1a>",
    "</a b>",
    "<-a>",
    "<",
    "</",
    "`a`b`",
    "` `",
    "\\",
    "{@1a}",
    "{@inheritDoc Foo|x}",
    "{@link Foo]x}",
    "@param {x}y -",
    "@a\u{c}b",
];

/// Constructs that take whole lines: fenced code, and the blocks whose rules span lines.
pub const BLOCKS: &[&[&str]] = &[
    &["```ts", "let x = 1;", "```"],
    &["```", "code", "  ```"],
    &["```ts  ", "a", "", "``` trailing"],
    &["```ts", "a", "```  "],
    &["```t`s", "a", "```"],
    &["```", "never closed"],
    &["```"],
    &["text ```", "a", "```"],
    &["```", "a", "``", "```"],
    &["@deprecated"],
    &["@deprecated", "Use the other one."],
    &["@deprecated `code`"],
    &["@deprecated", "```", "only code", "```"],
    &["@deprecated \\@"],
    &["@deprecated {@link Foo | text}"],
    &["{@inheritDoc}", "{@inheritDoc Foo}"],
    &["Summary.", "@remarks", "{@inheritDoc}"],
    &["@param x - one", "@param x - two", "@typeParam T - three"],
    &["@returns", "@see Foo", "@see Bar", "@privateRemarks no", "@example", "x"],
    &["{@link Foo |", "  multi line text", "}"],
    &["<a", "  href=\"x\"", ">"],
];

/// A comment drawn from [`PIECES`] and [`BLOCKS`]: lines of them, with or without each line's `*`.
pub fn comment(tc: &TestCase) -> String {
    let opening = vec!["/**", "/** ", "/**\n", " /**", "/*x", "x /**"];
    let mut comment = String::from(tc.draw(gs::sampled_from(opening)));
    let line_start = vec![" * ", " *", "*", "", "   ", " *   "];
    for line in 0..tc.draw(gs::integers::<usize>().max_value(6)) {
        if line > 0 || tc.draw(gs::booleans()) {
            comment.push('\n');
            comment += tc.draw(gs::sampled_from(line_start.clone()));
        }
        if tc.draw(gs::weighted_booleans(0.2)) {
            let block = tc.draw(gs::sampled_from(BLOCKS.to_vec()));
            for (i, text) in block.iter().enumerate() {
                if i > 0 {
                    comment.push('\n');
                    comment += tc.draw(gs::sampled_from(vec![" * ", " *  ", "   "]));
                }
                comment += text;
            }
            continue;
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
    let names =
        tc.draw(gs::vecs(gs::sampled_from(vec!["@custom", "@mine", "@Custom", "custom", "@remarks"])).max_size(2));
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
        let html = tc.draw(gs::vecs(gs::sampled_from(vec!["a", "b", "br", "my-el", "b c"])).max_size(3));
        keys.insert("supportedHtmlElements".into(), json!(html));
    }
    if tc.draw(gs::booleans()) {
        keys.insert("reportUnsupportedHtmlElements".into(), json!(tc.draw(gs::booleans())));
    }
    Some(Value::Object(keys))
}
