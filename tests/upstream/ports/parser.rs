//! Ports of the parser tests whose expectations aren't snapshots the harness makes by itself.

use tsdocrs::{
    TSDocConfiguration, TSDocMessageId, TSDocParser,
    nodes::{
        DocBlockTag, DocDeclarationReference, DocHtmlAttribute, DocHtmlStartTag, DocMemberIdentifier,
        DocMemberReference, DocMemberSelector, DocNode, DocParagraph, DocPlainText, DocSection, DocSoftBreak, NodeRef,
    },
    parser::{TextRange, TokenKind, split_paragraphs_for_section, tokenizer},
    transforms::trim_spaces_in_paragraph,
};

use crate::{
    harness::{Port, Test},
    snapshot::{Value, check, compare, expected, node_snapshot, parse, parse_event},
};

const PARSING_BASICS: &str = "tsdoc/src/__tests__/ParsingBasics.test.ts";

fn simple_beta_and_internal(test: &Test) -> Result<(), String> {
    let (text, configuration) = parse_event(test, 0)?;
    let tags = parse(text, configuration)?.doc_comment.modifier_tag_set;
    check(!tags.is_alpha(), "not @alpha")?;
    check(tags.is_beta(), "@beta")?;
    check(tags.is_internal(), "@internal")
}

fn has_custom_modifier(test: &Test) -> Result<(), String> {
    let (text, configuration) = parse_event(test, 0)?;
    check(parse(text, configuration)?.doc_comment.modifier_tag_set.has_tag_name("@customModifier"), "@customModifier")
}

inventory::submit! { Port { file: PARSING_BASICS, template: "01 Simple @beta and @internal extraction", run: simple_beta_and_internal } }
inventory::submit! { Port { file: PARSING_BASICS, template: "02 A basic TSDoc comment with common components", run: has_custom_modifier } }
inventory::submit! { Port { file: PARSING_BASICS, template: "03 Jumbled order", run: has_custom_modifier } }

fn every_message_id_is_known(_: &Test) -> Result<(), String> {
    let configuration = TSDocConfiguration::new();
    let missing: Vec<_> = TSDocMessageId::ALL.iter().filter(|id| !configuration.is_known_message_id(id.as_str())).collect();
    check(missing.is_empty(), &format!("no missing ids, not {missing:?}"))
}

fn unsupported_html_has_a_known_id(_: &Test) -> Result<(), String> {
    let mut configuration = TSDocConfiguration::new();
    configuration.set_supported_html_elements([]);
    configuration.validation.report_unsupported_html_elements = true;
    let context = TSDocParser::new(&configuration).parse_string("/**\n * <b>\n */").map_err(|e| e.to_string())?;
    let unsupported = context.log.iter().filter(|m| m.message_id == TSDocMessageId::UnsupportedHtmlElementName);
    check(unsupported.count() > 0, "an unsupported HTML element message")?;
    check(context.log.iter().all(|m| configuration.is_known_message_id(m.message_id.as_str())), "known ids")
}

const MESSAGE_IDS: &str = "tsdoc/src/parser/__tests__/TSDocMessageId.test.ts";
inventory::submit! { Port { file: MESSAGE_IDS, template: "allTsdocMessageIds includes every TSDocMessageId enum value", run: every_message_id_is_known } }
inventory::submit! { Port { file: MESSAGE_IDS, template: "unsupported HTML elements report a known message id", run: unsupported_html_has_a_known_id } }

fn is_punctuation(_: &Test) -> Result<(), String> {
    for kind in [TokenKind::OtherPunctuation, TokenKind::DoubleQuote, TokenKind::Slash] {
        check(tokenizer::is_punctuation(kind), &format!("{kind:?} to be punctuation"))?;
    }
    for kind in [TokenKind::EndOfInput, TokenKind::Spacing, TokenKind::AsciiWord] {
        check(!tokenizer::is_punctuation(kind), &format!("{kind:?} not to be punctuation"))?;
    }
    Ok(())
}

inventory::submit! { Port { file: "tsdoc/src/parser/__tests__/Tokenizer.test.ts", template: "Tokenizer.isPunctuation()", run: is_punctuation } }

fn construction_scenarios(test: &Test) -> Result<(), String> {
    let buffer = "0123456789";
    let range = TextRange::new(0, buffer.len());
    check(range.text(buffer) == buffer, "toString() to be the buffer")?;
    let sub_range = TextRange::new(3, 6);
    let snapshot = Value::Instance(
        "TextRange",
        vec![
            ("buffer", buffer.into()),
            ("end", Value::Number(sub_range.end as i64)),
            ("pos", Value::Number(sub_range.pos as i64)),
        ],
    );
    compare(expected(test, 0)?, &snapshot)
}

inventory::submit! { Port { file: "tsdoc/src/parser/__tests__/TextRange.test.ts", template: "construction scenarios", run: construction_scenarios } }

fn trim_spaces(test: &Test) -> Result<(), String> {
    let (text, configuration) = parse_event(test, 0)?;
    let context = parse(text, configuration)?;
    let Some(DocNode::Paragraph(paragraph)) = context.doc_comment.summary_section.nodes.first() else {
        return Err("expected the summary to start with a paragraph".into());
    };
    let trimmed = trim_spaces_in_paragraph(paragraph);
    compare(expected(test, 0)?, &node_snapshot(Some(&context), NodeRef::Paragraph(&trimmed)))
}

inventory::submit! { Port { file: "tsdoc/src/__tests__/DocNodeTransforms.test.ts", template: "01 trimSpacesInParagraphNodes()", run: trim_spaces } }

fn degenerate_manual_nodes(test: &Test) -> Result<(), String> {
    let text = |s: &str| DocNode::from(DocPlainText::new(s).unwrap());
    let soft_break = || DocNode::from(DocSoftBreak::default());
    let mut section = DocSection {
        nodes: vec![
            DocParagraph {
                nodes: vec![
                    text("  para 1 "),
                    soft_break(),
                    text("   "),
                    soft_break(),
                    text(" \t  "),
                    text("   "),
                    DocBlockTag::new("@public").unwrap().into(),
                    text("  para 2 "),
                    soft_break(),
                    soft_break(),
                    text("  para 3  "),
                ],
            }
            .into(),
            // Upstream keeps empty paragraphs.
            DocParagraph::default().into(),
        ],
    };
    split_paragraphs_for_section(&mut section);
    compare(expected(test, 0)?, &node_snapshot(None, NodeRef::Section(&section)))
}

inventory::submit! { Port { file: "tsdoc/src/__tests__/ParagraphSplitter.test.ts", template: "04 Degenerate manually constructed nodes", run: degenerate_manual_nodes } }

fn render_html_tag(test: &Test) -> Result<(), String> {
    let attribute = DocHtmlAttribute { name: "src".into(), value: "\"http://example.com\"".into(), ..Default::default() };
    let tag = DocHtmlStartTag { name: "img".into(), html_attributes: vec![attribute], ..Default::default() };
    compare(expected(test, 0)?, &tag.emit_as_html().into())
}

fn render_declaration_reference(test: &Test) -> Result<(), String> {
    let member = DocMemberReference {
        has_dot: false,
        member_identifier: Some(DocMemberIdentifier::new("MyClass")),
        selector: Some(DocMemberSelector::new("class")),
        ..Default::default()
    };
    let reference =
        DocDeclarationReference { package_name: Some("my-package".into()), member_references: vec![member], ..Default::default() };
    compare(expected(test, 0)?, &reference.emit_as_tsdoc().into())
}

const EMITTER: &str = "tsdoc/src/emitters/__tests__/TSDocEmitter.test.ts";
inventory::submit! { Port { file: EMITTER, template: "03 TSDocEmitter.renderHtmlTag()", run: render_html_tag } }
inventory::submit! { Port { file: EMITTER, template: "04 TSDocEmitter.renderDeclarationReference()", run: render_declaration_reference } }
