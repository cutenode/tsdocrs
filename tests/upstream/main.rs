//! `@microsoft/tsdoc`'s own tests, run against tsdocrs: each upstream test is a test here, named by
//! its file and Jest name. Its snapshots are compared with what tsdocrs makes of the same input,
//! and a port in ports/ checks the rest of its expectations. A test with an expectation nothing
//! here can check fails, and so does one that fails while allowlist.json says it's expected to, if
//! it passes: the allowlist only holds tests that fail.
//!
//! With UPSTREAM_DIR set, it runs the suite tools/upstream/vendor.sh wrote there instead of the one
//! checked in, against the same ports and allowlist: `npm run bench -- --live` runs TSDoc's
//! newest tests that way.

mod harness;
mod shapes;
mod snapshot;
mod ports {
    automod::dir!("tests/upstream/ports");
}

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use harness::{Event, Port, Suite, Test};
use libtest_mimic::{Arguments, Failed, Trial};
use serde::Deserialize;

/// tests/upstream/allowlist.json: the upstream tests expected to fail, by file and name, each with
/// why.
#[derive(Deserialize)]
struct Allowed {
    file: String,
    name: String,
    #[allow(dead_code)]
    reason: String,
}

fn main() {
    let dir = std::env::var_os("UPSTREAM_DIR")
        .map_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/upstream"), PathBuf::from);
    let cases = dir.join("cases.json");
    let suite: Suite = serde_json::from_str(
        &std::fs::read_to_string(&cases).unwrap_or_else(|error| panic!("{}: {error}", cases.display())),
    )
    .unwrap_or_else(|error| panic!("{}: {error}", cases.display()));
    let allowed: Vec<Allowed> = serde_json::from_str(include_str!("allowlist.json")).expect("allowlist.json");
    let trials = suite.files.into_iter().flat_map(|file| {
        let path = file.path;
        let allowed = &allowed;
        file.tests.into_iter().map(move |test| {
            let expected_to_fail = allowed.iter().any(|a| a.file == path && a.name == test.name);
            let name = format!("{path}::{}", test.name);
            let path = path.clone();
            Trial::test(name, move || {
                let result = catch_unwind(AssertUnwindSafe(|| check(&path, &test)))
                    .unwrap_or_else(|panic| Err(panic_message(&panic)));
                match (result, expected_to_fail) {
                    (Ok(()), false) | (Err(_), true) => Ok(()),
                    (Ok(()), true) => Err(Failed::from("passes now: take it out of allowlist.json")),
                    (Err(error), false) => Err(Failed::from(error)),
                }
            })
        })
    });
    libtest_mimic::run(&Arguments::from_args(), trials.collect()).exit();
}

/// Checks `test` from the upstream file at `path`: each snapshot the harness knows how to make,
/// and the ports of the test, which it needs when it has other expectations.
fn check(path: &str, test: &Test) -> Result<(), String> {
    let ports: Vec<&Port> = inventory::iter::<Port>
        .into_iter()
        .filter(|port| port.file == path && port.template == test.template)
        .collect();
    let mut unchecked = Vec::new();
    for (index, event) in test.events.iter().enumerate() {
        let checked = match event {
            Event::Parse { .. } | Event::TextRange { .. } => true,
            Event::Snapshot { key, .. } => match shapes::snapshot(path, test, index) {
                Some(result) => {
                    result.map_err(|error| format!("{key}: {error}"))?;
                    true
                }
                None => !ports.is_empty(),
            },
            Event::InlineSnapshot { .. } => match shapes::snapshot(path, test, index) {
                Some(result) => {
                    result.map_err(|error| format!("inline snapshot: {error}"))?;
                    true
                }
                None => !ports.is_empty(),
            },
            Event::Assertion { .. } => !ports.is_empty(),
        };
        if !checked {
            unchecked.push(match event {
                Event::Assertion { matcher } => format!("expect().{matcher}"),
                Event::Snapshot { key, .. } => format!("snapshot {key:?}"),
                _ => "an inline snapshot".into(),
            });
        }
    }
    if !unchecked.is_empty() {
        return Err(format!("nothing here checks {}: add a port to tests/upstream/ports/", unchecked.join(", ")));
    }
    ports.iter().try_for_each(|port| (port.run)(test))
}

fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_else(|| "panicked".into())
}
