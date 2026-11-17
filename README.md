# tsdocrs

tsdocrs parses [TSDoc](https://tsdoc.org) comments in Rust. It's a port of [`@microsoft/tsdoc`](https://github.com/microsoft/tsdoc)'s parser that gives the same tree of doc nodes and the same messages, with TSDoc's own message ids and texts, so a tool built on it reports what `eslint-plugin-tsdoc` does. It also reads tsdoc.json the way `@microsoft/tsdoc-config` does, and ships to npm as WebAssembly.

## Use it

```rust
use tsdocrs::{TSDocConfiguration, TSDocParser};

let configuration = TSDocConfiguration::new();
let context = TSDocParser::new(&configuration).parse_string("/**\n * Adds.\n * @bogus\n */").unwrap();
for message in &context.log {
    // tsdoc-undefined-tag: (3,4): The TSDoc tag "@bogus" is not defined in this configuration
    println!("{}: {}", message.message_id, message.text);
}
```

- `TSDocParser::parse_string` gives a `ParserContext`: the comment's lines, tokens, `doc_comment` tree and `log` of messages. It returns an error where `@microsoft/tsdoc` throws (it does on a few inputs, like `{@link (Foo:}`), with the same message.
- `nodes::NodeRef` views any node, with its `kind()` and its `child_nodes()` in TSDoc's order, excerpts included. Each node kind is a struct with TSDoc's fields, like `DocParamBlock::parameter_name`.
- `config_file::load` reads a tsdoc.json and the files it `extends`; `config_file::find` finds the one tsdoc-config would use for a source file. Turn off the `config-file` feature to leave out file reading; `TSDocConfigFile::configure_parser` still sets up a parser from a tsdoc.json's settings.
- `emitters` writes a tree back out as TSDoc, and `beta::declaration_reference` is TSDoc's beta DeclarationReference parser.

Offsets are bytes into the parsed `&str`. Where TSDoc counts UTF-16 code units, the line and column in a message's text do too, so the text matches.

## npm

```js
const { parse } = require("tsdocrs");
const { messages, docComment } = parse("/** @bogus */", { tagDefinitions: [{ tagName: "@mine", syntaxKind: "block" }] });
```

`parse(text, settings?)` takes a tsdoc.json's settings without `extends`, and gives the lines, the messages and the tree as plain objects, with offsets in UTF-16 code units as JavaScript strings index. TypeScript types ship with it.

It's built with [wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen), which is actively maintained, and runs as one WebAssembly file on every platform Node runs on. An editor extension or a linter plugin gets no native binary per platform to download, and a parse is one call with a string in and an object out.

## Testing

TSDoc's own test suite runs against tsdocrs. [tools/upstream/vendor.sh](tools/upstream/vendor.sh) runs microsoft/tsdoc's Jest tests at the commit in [tests/upstream/manifest.json](tests/upstream/manifest.json) and records each test's inputs and checked-in snapshots in [tests/upstream/cases.json](tests/upstream/cases.json). The harness in [tests/upstream/](tests/upstream/) makes each snapshot from tsdocrs's results and compares the text, and [tests/upstream/ports/](tests/upstream/ports/) ports the tests whose expectations aren't snapshots. A test that fails sits in [tests/upstream/allowlist.json](tests/upstream/allowlist.json) with why, and the allowlist is empty. A weekly workflow checks microsoft/tsdoc for a newer commit, re-vendors and runs the suite, and opens a pull request with the diff and the results.

Beyond that suite:

- Differential tests ([tests/differential.rs](tests/differential.rs)) draw TSDoc-shaped comments and tsdoc.json settings with [Hegel](https://crates.io/crates/hegeltest), run them through `@microsoft/tsdoc` ([tools/oracle/oracle.mjs](tools/oracle/oracle.mjs)) and tsdocrs, and require the same lines, messages and tree. They need Node and `npm ci`.
- [tests/fixtures/corpus.json](tests/fixtures/corpus.json) holds 260 comments with what `@microsoft/tsdoc` makes of each, checked without Node.
- Hegel properties ([tests/props/](tests/props/)) and cargo-fuzz targets ([fuzz/](fuzz/)) for the tokenizer, the parser, and the parser against `@microsoft/tsdoc`.
- The npm package's tests compare it with `@microsoft/tsdoc` in the same Node process.

## Benchmarks

With Node from [.node-version](.node-version) and rustup installed, from a fresh checkout:

```sh
npm ci
npm run bench
```

It times parsing, and parsing and emitting TSDoc again, of three corpora of real doc comments from pinned sources ([bench/corpus/](bench/corpus/SOURCES.md)): tsdocrs in Rust with [criterion](https://crates.io/crates/criterion), and tsdocrs's npm package and `@microsoft/tsdoc` in the same Node process with [tinybench](https://www.npmjs.com/package/tinybench). It writes target/bench/report.md and report.json: mean, p50 and p95 per pass, the spread, comments and MB per second, the speedup over `@microsoft/tsdoc`, and the versions and machine it ran on. rust-toolchain.toml, .node-version and the lockfiles pin what runs; the first run also builds the wasm-bindgen CLI the npm package pins, into target/. A weekly workflow runs it too, with the report in its summary.

## Scripts

```sh
tools/check.sh                      # everything CI checks
tools/build-npm.sh                  # the npm package, into npm/tsdocrs/pkg
npm run bench                       # the benchmarks, into target/bench/report.md
tools/bench/corpus.sh               # remake bench/corpus/ from its pinned sources
tools/upstream/vendor.sh [COMMIT]   # re-vendor TSDoc's suite
node tools/oracle/oracle.mjs --record tests/fixtures/corpus.json   # re-record the corpus
```

## Releases

[release.yml](.github/workflows/release.yml) runs releases with [uppt](https://github.com/danielroe/uppt). Each push to main opens or updates a draft release PR whose version and changelog come from the conventional commits since the last tag, with npm/tsdocrs/package.json, Cargo.toml and the lockfiles bumped to match. Merging it tags the commit, cuts a GitHub release, publishes the crate to crates.io, and stages the npm package with provenance, which goes live once it's approved on npmjs.com or with `npm stage approve`. Both registries trust the workflow directly, so there are no tokens; `tools/bootstrap-publishing.sh` sets that up once. npm expires a new trust config that hasn't published within 2 days, so merge the next release PR within 2 days of running it, or run it again with `RETRUST=1` first.

## License

MIT. tsdocrs is a port of Microsoft's MIT-licensed `@microsoft/tsdoc`; [LICENSE](LICENSE) carries both notices.
