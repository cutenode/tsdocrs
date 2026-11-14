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

use common::generate::{comment, config};
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
