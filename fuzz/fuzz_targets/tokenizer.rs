//! The tokenizer on any lines of any text: its tokens spell the lines back, each line ending in a
//! newline and the whole in one end of input.
#![no_main]

use libfuzzer_sys::fuzz_target;
use tsdocrs::parser::{TextRange, TokenKind, tokenizer};

fuzz_target!(|text: &str| {
    let mut lines = Vec::new();
    let mut start = 0;
    for (at, c) in text.char_indices().filter(|&(_, c)| c == '\n') {
        lines.push(TextRange::new(start, at));
        start = at + c.len_utf8();
    }
    let tokens = tokenizer::read_tokens(text, &lines);
    let spelled: String = tokens.iter().map(|token| token.text(text)).collect();
    let expected: String = lines.iter().map(|line| format!("{}\n", line.text(text))).collect();
    assert_eq!(spelled, expected);
    assert_eq!(tokens.last().map(|token| token.kind), Some(TokenKind::EndOfInput));
});
