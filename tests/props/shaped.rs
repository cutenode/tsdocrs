//! Properties of TSDoc-shaped comments, drawn the way the differential tests draw them but checked
//! without Node: the tree, tags, messages and emitted TSDoc hold together, and the configuration
//! and hand-built nodes do what they say.

use hegel::{TestCase, generators as gs};
use tsdocrs::{
    TSDocConfiguration, TSDocMessageId, TSDocParser, TSDocTagDefinition, TSDocTagSyntaxKind,
    emitters::plain_text_emitter::has_any_text_content,
    nodes::{
        DocBlockTag, DocCodeSpan, DocComment, DocFencedCode, DocHtmlEndTag, DocInlineTag, DocLinkTag,
        DocMemberIdentifier, DocNode, DocParagraph, DocPlainText, DocSoftBreak, InlineTagBase, LinkTagExcerpts,
        NodeRef,
    },
    transforms::trim_spaces_in_paragraph,
};

use crate::common::{
    configuration,
    generate::{comment, config},
};

/// Each node under `root`, parents first.
fn walk(root: NodeRef) -> Vec<NodeRef> {
    let (mut stack, mut nodes) = (vec![root], Vec::new());
    while let Some(node) = stack.pop() {
        nodes.push(node);
        stack.extend(node.child_nodes().into_iter().rev());
    }
    nodes
}

#[hegel::test(test_cases = 300)]
fn shaped_comments_hold_together(tc: TestCase) {
    let comment = comment(&tc);
    let Ok(configuration) = configuration(config(&tc).as_ref()) else { return };
    let context = match TSDocParser::new(&configuration).parse_string(&comment) {
        Ok(context) => context,
        Err(error) => {
            assert!(!error.message.is_empty() && error.to_string() == error.message);
            return;
        }
    };
    let root = NodeRef::Comment(&context.doc_comment);
    // Excerpts cover each token at most once.
    let mut covered = vec![false; context.tokens.len()];
    for node in walk(root) {
        assert!(!node.kind().as_str().is_empty());
        if let NodeRef::Excerpt(excerpt) = node {
            assert!(!excerpt.excerpt_kind.as_str().is_empty());
            for slot in &mut covered[excerpt.content.start_index..excerpt.content.end_index] {
                assert!(!std::mem::replace(slot, true), "{comment:?}");
            }
        }
    }
    // Each tag read is its name's tokens.
    for tag in &context.tags {
        assert_eq!(context.text(tag.token_sequence), tag.tag_name);
    }
    for message in &context.log {
        assert_eq!(message.to_string(), message.text);
        assert_eq!(TSDocMessageId::from_id(message.message_id.as_str()), Some(message.message_id));
        assert!(configuration.is_known_message_id(&message.message_id.to_string()));
    }
    let tags = &context.doc_comment.modifier_tag_set;
    let modifiers = [
        ("@alpha", tags.is_alpha()),
        ("@beta", tags.is_beta()),
        ("@eventProperty", tags.is_event_property()),
        ("@experimental", tags.is_experimental()),
        ("@internal", tags.is_internal()),
        ("@override", tags.is_override()),
        ("@packageDocumentation", tags.is_package_documentation()),
        ("@public", tags.is_public()),
        ("@readonly", tags.is_readonly()),
        ("@sealed", tags.is_sealed()),
        ("@virtual", tags.is_virtual()),
    ];
    for (name, is) in modifiers {
        assert_eq!(tags.has_tag_name(name), is, "{name}");
    }
    // Text with more than spaces counts as text.
    let summary = NodeRef::Section(&context.doc_comment.summary_section);
    let texty = walk(summary).iter().any(|node| matches!(node, NodeRef::PlainText(text) if text.text.trim() != ""));
    assert!(!texty || has_any_text_content([summary], 1));
    // A trimmed paragraph's text has no runs of spaces and none at its ends.
    for node in walk(root) {
        if let NodeRef::Paragraph(paragraph) = node {
            let trimmed = trim_spaces_in_paragraph(paragraph);
            for node in &trimmed.nodes {
                if let DocNode::PlainText(text) = node {
                    assert!(!text.text.contains("  ") && !text.text.is_empty());
                }
            }
            if let Some(DocNode::PlainText(first)) = trimmed.nodes.first() {
                assert!(!first.text.starts_with(' '));
            }
        }
    }
    // The emitted comment is a comment, and reads again.
    let emitted = context.doc_comment.emit_as_tsdoc();
    if !emitted.is_empty() {
        assert!(emitted.starts_with("/**\n") && emitted.ends_with("*/\n"), "{emitted:?}");
        let again = TSDocParser::new(&configuration).parse_string(&emitted);
        assert!(again.is_ok_and(|again| again.log.iter().all(|m| m.message_id != TSDocMessageId::CommentNotFound)));
    }
}

/// Tags are found in any case, defined once, marked supported or not, and HTML elements are
/// allowed once each.
#[hegel::test(test_cases = 100)]
fn the_configuration_keeps_what_it_is_told(tc: TestCase) {
    let mut configuration = TSDocConfiguration::new();
    configuration.clear(tc.draw(gs::booleans()));
    let names = tc.draw(gs::vecs(gs::sampled_from(vec!["@one", "@Two", "@three"])).unique(true).max_size(3));
    let supported = tc.draw(gs::optional(gs::booleans()));
    let kind = tc.draw(gs::sampled_from(vec![
        TSDocTagSyntaxKind::BlockTag,
        TSDocTagSyntaxKind::InlineTag,
        TSDocTagSyntaxKind::ModifierTag,
    ]));
    let tags = names.iter().map(|name| TSDocTagDefinition::new(name, kind, false).unwrap());
    configuration.add_tag_definitions(tags, supported).unwrap();
    for name in &names {
        let found = configuration.try_get_tag_definition(&name.to_uppercase()).unwrap();
        assert_eq!(found.tag_name, *name);
        assert_eq!(configuration.is_tag_supported(found), supported == Some(true));
        // The same definition again is fine; another of that name isn't.
        configuration.add_tag_definition(found.clone()).unwrap();
        let other = TSDocTagDefinition::new(&name.to_lowercase(), TSDocTagSyntaxKind::BlockTag, true).unwrap();
        assert!(other == *found || configuration.add_tag_definition(other).is_err());
    }
    let supported_names: Vec<&str> =
        configuration.supported_tag_definitions().map(|tag| tag.tag_name.as_str()).collect();
    assert_eq!(supported_names.len(), if supported == Some(true) { names.len() } else { 0 });
    assert_eq!(configuration.validation.report_unsupported_tags, supported.is_some());
    assert!(configuration.set_support_for_tag("@nowhere", true).is_err());
    assert!(TSDocTagDefinition::new("nope", kind, false).is_err());
    let html = tc.draw(gs::vecs(gs::sampled_from(vec!["a", "b", "b"])).max_size(4));
    configuration.set_supported_html_elements(html.iter().map(|name| (*name).to_owned()));
    assert!(configuration.supported_html_elements().len() <= html.len());
    assert!(html.iter().all(|name| configuration.is_html_element_supported(name)));
    assert!(configuration.validation.report_unsupported_html_elements);
    configuration.clear(false);
    assert_eq!(configuration.tag_definitions().len(), tsdocrs::standard_tags().len());
    assert!(TSDocMessageId::ALL.iter().all(|id| configuration.is_known_message_id(id.as_str())));
}

/// Nodes built by hand emit as TSDoc writes them.
#[hegel::test(test_cases = 50)]
fn hand_built_nodes_emit(tc: TestCase) {
    let word = tc.draw(gs::sampled_from(vec!["x", "two words", "a.b"]));
    let identifier = DocMemberIdentifier::new(word);
    assert_eq!(identifier.has_quotes(), !DocMemberIdentifier::is_valid_identifier(word));
    let link = DocLinkTag {
        base: InlineTagBase::new("@link").unwrap(),
        code_destination: None,
        url_destination: Some("https://example.com".into()),
        link_text: tc.draw(gs::optional(gs::sampled_from(vec!["text".to_owned()]))),
        excerpts: LinkTagExcerpts::default(),
    };
    let label = DocInlineTag {
        base: InlineTagBase::new("@label").unwrap(),
        tag_content: word.into(),
        tag_content_excerpt: None,
    };
    assert!(InlineTagBase::new("label").is_err() && DocBlockTag::new("@").is_err());
    assert!(DocPlainText::new("two\nlines").is_err());
    let paragraph = DocParagraph {
        nodes: vec![
            DocPlainText::new(format!("  {word}  ")).unwrap().into(),
            DocCodeSpan::new("code()").into(),
            DocSoftBreak::default().into(),
            link.clone().into(),
            label.into(),
        ],
    };
    let mut comment = DocComment::default();
    comment.summary_section.nodes.push(paragraph.into());
    comment.summary_section.nodes.push(DocFencedCode::new("ts", "let x = 1;\n").into());
    let emitted = comment.emit_as_tsdoc();
    assert!(emitted.contains(&format!(" * {word} `code()`")), "{emitted}");
    assert!(emitted.contains("{@link https://example.com"), "{emitted}");
    assert!(emitted.contains(" * ```ts\n * let x = 1;\n * ```\n"), "{emitted}");
    assert_eq!(DocHtmlEndTag { name: "b".into(), ..Default::default() }.emit_as_html(), "</b>");
}
