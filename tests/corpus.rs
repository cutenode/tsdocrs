//! tests/fixtures/corpus.json: comments from docov's TSDoc checks and `@microsoft/tsdoc`'s own
//! tests, with what that parser makes of each (tools/oracle/oracle.mjs --record). tsdocrs makes
//! the same of every one, tree and messages.

mod common;

use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Entry {
    comment: String,
    config: Option<Value>,
    expected: Value,
}

#[test]
fn every_comment_parses_as_microsoft_tsdoc_parses_it() {
    let entries: Vec<Entry> = serde_json::from_str(include_str!("fixtures/corpus.json")).unwrap();
    let wrong: Vec<String> = entries
        .iter()
        .filter(|entry| {
            common::report(&entry.comment, entry.config.as_ref()) != common::comparable(entry.expected.clone())
        })
        .map(|entry| format!("{:?}", entry.comment))
        .collect();
    assert!(wrong.is_empty(), "{} of {} differ:\n{}", wrong.len(), entries.len(), wrong.join("\n"));
}
