//! tsdocrs's side of `npm run bench`: parsing each corpus in bench/corpus/, and parsing and emitting
//! it again, with the default configuration, as tools/bench/node.mjs does with `@microsoft/tsdoc`.
//! tools/bench/run.mjs reads criterion's samples from target/criterion/ for the report.

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use tsdocrs::{TSDocConfiguration, TSDocParser};

/// The corpora, smallest first.
const CORPORA: [(&str, &str); 3] = [
    ("small", include_str!("../bench/corpus/small.json")),
    ("medium", include_str!("../bench/corpus/medium.json")),
    ("large", include_str!("../bench/corpus/large.json")),
];

fn bench(c: &mut Criterion) {
    let configuration = TSDocConfiguration::new();
    let parser = TSDocParser::new(&configuration);
    for (name, json) in CORPORA {
        let comments: Vec<String> = serde_json::from_str(json).expect("a corpus is a JSON array of strings");
        let bytes: usize = comments.iter().map(String::len).sum();
        let mut group = c.benchmark_group(name);
        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_function("parse", |b| {
            b.iter(|| {
                for comment in &comments {
                    black_box(parser.parse_string(black_box(comment)).ok());
                }
            });
        });
        group.bench_function("emit", |b| {
            b.iter(|| {
                for comment in &comments {
                    if let Ok(context) = parser.parse_string(black_box(comment)) {
                        black_box(context.doc_comment.emit_as_tsdoc());
                    }
                }
            });
        });
        group.finish();
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
