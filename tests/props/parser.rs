//! Properties of the parser on any text: it never panics, its tokens spell out each line, its
//! excerpts never overlap and stay in order, and its messages point inside the text.

use hegel::{Generator, TestCase, generators as gs};
use tsdocrs::{
    TSDocParser,
    nodes::NodeRef,
    parser::{TextRange, TokenKind, tokenizer},
};

/// Text that's usually a comment, with TSDoc's punctuation over-represented.
fn text(tc: &TestCase) -> String {
    let body = tc.draw(
        gs::vecs(gs::one_of(vec![
            gs::text().max_size(4).boxed(),
            gs::sampled_from(vec![
                "@", "{", "}", "<", ">", "`", "\\", "\n", " * ", "*/", "\"", "#", ".", "(", ":", "|",
            ])
            .map(str::to_owned)
            .boxed(),
        ]))
        .max_size(30),
    );
    let body = body.concat();
    if tc.draw(gs::booleans()) { format!("/**{body}*/") } else { body }
}

#[hegel::test(test_cases = 300)]
fn tokens_spell_out_each_line(tc: TestCase) {
    let text = text(&tc);
    // Without a comment there are no lines, and nothing to tokenize.
    let Ok(context) = TSDocParser::default().parse_string(&text) else { return };
    if context.tokens.is_empty() {
        assert!(context.lines.is_empty());
        return;
    }
    let mut tokens = context.tokens.iter().peekable();
    for &line in &context.lines {
        let mut spelled = String::new();
        while let Some(token) = tokens.next_if(|token| token.kind != TokenKind::Newline) {
            assert_eq!(token.line, line);
            spelled += token.text(&text);
        }
        assert_eq!(spelled, line.text(&text));
        assert_eq!(tokens.next().map(|token| token.kind), Some(TokenKind::Newline));
    }
    assert_eq!(tokens.next().map(|token| token.kind), Some(TokenKind::EndOfInput));
    assert!(tokens.next().is_none());
}

#[hegel::test(test_cases = 300)]
fn excerpts_cover_tokens_once_in_order(tc: TestCase) {
    let text = text(&tc);
    let Ok(context) = TSDocParser::default().parse_string(&text) else { return };
    let mut stack = vec![NodeRef::Comment(&context.doc_comment)];
    let mut covered = vec![false; context.tokens.len()];
    while let Some(node) = stack.pop() {
        if let NodeRef::Excerpt(excerpt) = node {
            for slot in &mut covered[excerpt.content.start_index..excerpt.content.end_index] {
                assert!(!*slot, "two excerpts cover one token in {text:?}");
                *slot = true;
            }
        }
        stack.extend(node.child_nodes().into_iter().rev());
    }
}

#[hegel::test(test_cases = 300)]
fn messages_point_inside_the_text(tc: TestCase) {
    let text = text(&tc);
    let Ok(context) = TSDocParser::default().parse_string(&text) else { return };
    for message in &context.log {
        assert!(message.text_range.end <= text.len());
        assert!(text.is_char_boundary(message.text_range.pos) && text.is_char_boundary(message.text_range.end));
        assert!(message.text.ends_with(&message.unformatted_text));
        let location = TextRange::location(&text, message.text_range.pos as isize);
        if message.text_range != TextRange::EMPTY {
            assert!(message.text.starts_with(&format!("({},{}): ", location.line, location.column)));
        }
    }
}

#[hegel::test(test_cases = 100)]
fn punctuation_is_what_a_backslash_escapes(tc: TestCase) {
    let c = tc.draw(gs::characters());
    let text = format!("/** {c} */");
    let context = TSDocParser::default().parse_string(&text).unwrap();
    if let Some(token) = context.tokens.first().filter(|token| token.kind != TokenKind::Newline) {
        assert_eq!(tokenizer::is_punctuation(token.kind), c.is_ascii_punctuation() && c != '_', "{c:?}");
    }
}
