//! The vendored suite's shape: tests/upstream/cases.json, which tools/upstream/vendor.sh writes
//! from a run of `@microsoft/tsdoc`'s own Jest tests, and the ports of the tests whose expectations
//! aren't snapshots.

use serde::Deserialize;
use serde_json::Value;

/// tests/upstream/cases.json.
#[derive(Deserialize)]
pub struct Suite {
    pub files: Vec<File>,
}

/// An upstream test file and its tests, in the order Jest ran them.
#[derive(Deserialize)]
pub struct File {
    /// Its path in microsoft/tsdoc, like `tsdoc/src/parser/__tests__/NodeParserTags.test.ts`.
    pub path: String,
    pub tests: Vec<Test>,
}

/// One Jest test: one `test()` or `it()`, or one row of a `.each` table.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Test {
    /// Jest's full name for it: the `describe` names and its own, joined by spaces, with a `.each`
    /// row's values filled in. Snapshot keys start with it.
    pub name: String,
    /// The same before a `.each` row fills it in, which is what a [`Port`] names.
    pub template: String,
    /// A `.each` row: an array of its arguments, or an object for a tagged-template table.
    pub each: Option<Value>,
    /// What it did, in order.
    pub events: Vec<Event>,
}

/// Something an upstream test did that the harness replays or checks.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    /// `TSDocParser.parseString(text)`, with its configuration as it was at the call.
    Parse { text: String, configuration: Value },
    /// `TextRange.fromString(text)`.
    TextRange { text: String },
    /// `toMatchSnapshot()`: the snapshot's key in its .snap file and its checked-in text, plus the
    /// TestHelpers function that made it, if one did.
    Snapshot { key: String, helper: Option<String>, expected: String },
    /// `toMatchInlineSnapshot()`, with its checked-in text.
    InlineSnapshot { expected: String },
    /// Any other `expect()` matcher, which only a port can check.
    Assertion { matcher: String },
}

/// A Rust port of an upstream test, for the tests a snapshot can't check by itself. It runs once
/// for each of the template's tests, which have its `.each` row.
pub struct Port {
    /// The upstream file's path, as in [`File::path`].
    pub file: &'static str,
    /// The test's [`Test::template`].
    pub template: &'static str,
    /// Checks it, with an error saying what differs.
    pub run: fn(&Test) -> Result<(), String>,
}

inventory::collect!(Port);
