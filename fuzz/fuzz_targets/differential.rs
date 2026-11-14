//! tsdocrs against `@microsoft/tsdoc` on any comment: the same lines, messages and tree, or the
//! same error. It runs tools/oracle/oracle.mjs, so it needs Node and `npm ci` in the crate.
#![no_main]

#[path = "../../tests/common/mod.rs"]
mod common;

use std::{
    io::{BufRead, BufReader, Write},
    process::{ChildStdin, ChildStdout, Command, Stdio},
    sync::{Mutex, OnceLock},
};

use libfuzzer_sys::fuzz_target;

fn oracle() -> &'static Mutex<(ChildStdin, BufReader<ChildStdout>)> {
    static ORACLE: OnceLock<Mutex<(ChildStdin, BufReader<ChildStdout>)>> = OnceLock::new();
    ORACLE.get_or_init(|| {
        let script = concat!(env!("CARGO_MANIFEST_DIR"), "/../tools/oracle/oracle.mjs");
        let mut child = Command::new("node")
            .args([script, "--serve"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("node");
        Mutex::new((child.stdin.take().unwrap(), BufReader::new(child.stdout.take().unwrap())))
    })
}

fuzz_target!(|text: &str| {
    let comment = format!("/**{text}*/");
    let mut oracle = oracle().lock().unwrap();
    writeln!(oracle.0, "{}", serde_json::json!({ "comment": comment })).unwrap();
    let mut line = String::new();
    oracle.1.read_line(&mut line).unwrap();
    let expected = common::comparable(serde_json::from_str(&line).unwrap());
    assert_eq!(common::report(&comment, None), expected, "{comment:?}");
});
