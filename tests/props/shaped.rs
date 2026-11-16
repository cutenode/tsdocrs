//! Properties of TSDoc-shaped comments, drawn the way the differential tests draw them but checked
//! without Node: the tree, tags, messages and emitted TSDoc hold together, and the configuration
//! and hand-built nodes do what they say.

use hegel::{TestCase, generators as gs};
use tsdocrs::{
    TSDocConfiguration, TSDocMessageId, TSDocParser, TSDocTagDefinition, TSDocTagSyntaxKind,
    emitters::plain_text_emitter::has_any_text_content,
    nodes::{
        DocBlockTag, DocCodeSpan, DocComment, DocDeclarationReference, DocExcerpt, DocFencedCode, DocHtmlAttribute,
        DocHtmlEndTag, DocHtmlStartTag, DocInlineTag, DocLinkTag, DocMemberIdentifier, DocMemberReference,
        DocMemberSelector, DocNode, DocParagraph, DocPlainText, DocSoftBreak, InlineTagBase, LinkTagExcerpts, NodeRef,
    },
    parser::TextRange,
    transforms::trim_spaces_in_paragraph,
};

use crate::common::{
    configuration,
    generate::{BLOCKS, PIECES, comment, config},
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

/// `node`, owned.
fn owned(node: NodeRef) -> DocNode {
    match node {
        NodeRef::Block(n) => n.clone().into(),
        NodeRef::BlockTag(n) => n.clone().into(),
        NodeRef::Excerpt(n) => (*n).into(),
        NodeRef::FencedCode(n) => n.clone().into(),
        NodeRef::CodeSpan(n) => n.clone().into(),
        NodeRef::Comment(n) => n.clone().into(),
        NodeRef::DeclarationReference(n) => n.clone().into(),
        NodeRef::ErrorText(n) => n.clone().into(),
        NodeRef::EscapedText(n) => n.clone().into(),
        NodeRef::HtmlAttribute(n) => n.clone().into(),
        NodeRef::HtmlEndTag(n) => n.clone().into(),
        NodeRef::HtmlStartTag(n) => n.clone().into(),
        NodeRef::InheritDocTag(n) => n.clone().into(),
        NodeRef::InlineTag(n) => n.clone().into(),
        NodeRef::LinkTag(n) => n.clone().into(),
        NodeRef::MemberIdentifier(n) => n.clone().into(),
        NodeRef::MemberReference(n) => n.clone().into(),
        NodeRef::MemberSelector(n) => n.clone().into(),
        NodeRef::MemberSymbol(n) => n.clone().into(),
        NodeRef::Paragraph(n) => n.clone().into(),
        NodeRef::ParamBlock(n) => n.clone().into(),
        NodeRef::ParamCollection(n) => n.clone().into(),
        NodeRef::PlainText(n) => n.clone().into(),
        NodeRef::Section(n) => n.clone().into(),
        NodeRef::SoftBreak(n) => n.clone().into(),
    }
}

/// The excerpts under `node`, in tree order.
fn excerpts<'a>(node: NodeRef<'a>) -> Vec<&'a DocExcerpt> {
    walk(node).into_iter().filter_map(|node| if let NodeRef::Excerpt(e) = node { Some(e) } else { None }).collect()
}

/// A drawn comment parses into a tree that holds together: its excerpts cover each token at most
/// once and run in source order within each block, each error text has its message, each tag read
/// is its name's tokens, and its references and HTML tags read back the same once emitted. Where
/// TSDoc throws, the error says why and keeps what it logged.
#[hegel::test(test_cases = 400)]
fn shaped_comments_hold_together(tc: TestCase) {
    let comment = comment(&tc);
    let Ok(configuration) = configuration(config(&tc).as_ref()) else { return };
    holds_together(&comment, &configuration);
}

/// Each construct the comments are drawn from holds together alone too, between a drawn opening
/// and closing, so the rare ones are read every time.
#[hegel::test(test_cases = 20)]
fn each_construct_holds_together(tc: TestCase) {
    let Ok(configuration) = configuration(config(&tc).as_ref()) else { return };
    let line_start = tc.draw(gs::sampled_from(vec![" * ", " ", "\n * "]));
    let closing = tc.draw(gs::sampled_from(vec![" */", "\n */", "*/"]));
    let constructs = PIECES.iter().map(|piece| vec![*piece]).chain(BLOCKS.iter().map(|block| block.to_vec()));
    for construct in constructs {
        let comment = format!("/**{line_start}{}{closing}", construct.join("\n * "));
        holds_together(&comment, &configuration);
    }
}

/// Checks [`shaped_comments_hold_together`]'s invariants of `comment`.
fn holds_together(comment: &str, configuration: &TSDocConfiguration) {
    let parser = TSDocParser::new(configuration);
    let context = match parser.parse_string(comment) {
        Ok(context) => context,
        Err(error) => {
            assert!(!error.message.is_empty() && error.to_string() == error.message);
            assert!(error.log.iter().all(|m| configuration.is_known_message_id(m.message_id.as_str())));
            return;
        }
    };
    let root = NodeRef::Comment(&context.doc_comment);
    let mut covered = vec![false; context.tokens.len()];
    for excerpt in excerpts(root) {
        assert!(!excerpt.excerpt_kind.as_str().is_empty());
        for slot in &mut covered[excerpt.content.start_index..excerpt.content.end_index] {
            assert!(!std::mem::replace(slot, true), "{comment:?}");
        }
        // An excerpt's range runs from its first token's start to its last token's end.
        let range = excerpt.content.containing_text_range(&context.tokens);
        match excerpt.content.tokens(&context.tokens) {
            [] => assert_eq!(range, TextRange::EMPTY),
            [first, .., last] | [first @ last] => assert_eq!((range.pos, range.end), (first.range.pos, last.range.end)),
        }
        assert_eq!(context.range_text(range).len(), range.len());
        assert_eq!(range.is_empty(), range.pos == range.end);
    }
    for node in walk(root) {
        assert!(!node.kind().as_str().is_empty());
        match node {
            // A block's excerpts run in source order: TSDoc only moves whole blocks and tags.
            NodeRef::Section(_) | NodeRef::Block(_) | NodeRef::ParamBlock(_) => {
                let starts: Vec<usize> = excerpts(node).iter().map(|e| e.content.start_index).collect();
                assert!(starts.is_sorted(), "{comment:?}");
            }
            // Each error text is logged, with its text's range.
            NodeRef::ErrorText(error) => {
                let range = error.text_excerpt.content.containing_text_range(&context.tokens);
                let logged = context.log.iter().any(|m| {
                    m.message_id == error.message_id
                        && m.unformatted_text == error.error_message
                        && m.text_range == range
                });
                assert!(logged, "{comment:?}: {}", error.error_message);
                assert_eq!(error.text, context.text(error.text_excerpt.content));
            }
            // A reference reads back the same once emitted.
            NodeRef::LinkTag(DocLinkTag { code_destination: Some(reference), .. }) => {
                let emitted = reference.emit_as_tsdoc();
                let again = parser.parse_string(&format!("/** {{@link {emitted}}} */")).unwrap();
                let link = walk(NodeRef::Comment(&again.doc_comment)).into_iter().find_map(|node| match node {
                    NodeRef::LinkTag(link) => link.code_destination.clone(),
                    _ => None,
                });
                assert_eq!(link.map(|again| again.emit_as_tsdoc()), Some(emitted), "{comment:?}");
            }
            // An HTML tag reads back the same once emitted.
            NodeRef::HtmlStartTag(tag) => {
                let again = parser.parse_string(&format!("/** {} */", tag.emit_as_html())).unwrap();
                let read = walk(NodeRef::Comment(&again.doc_comment)).into_iter().find_map(|node| match node {
                    NodeRef::HtmlStartTag(read) => Some(read.clone()),
                    _ => None,
                });
                let attributes = |tag: &DocHtmlStartTag| {
                    tag.html_attributes.iter().map(|a| (a.name.clone(), a.value.clone())).collect::<Vec<_>>()
                };
                let read = read.expect("the emitted tag reads as one");
                assert_eq!(
                    (&read.name, attributes(&read), read.self_closing_tag),
                    (&tag.name, attributes(tag), tag.self_closing_tag)
                );
            }
            _ => {}
        }
    }
    // A node held as a `DocNode` is the same node, of the same kind.
    for node in walk(root) {
        let owned = owned(node);
        assert_eq!((owned.kind(), owned.as_ref()), (node.kind(), node));
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
    // A parameter's block is the first with its name.
    for params in [&context.doc_comment.params, &context.doc_comment.type_params] {
        for block in &params.blocks {
            let first = params.blocks.iter().find(|b| b.parameter_name == block.parameter_name);
            assert_eq!(params.try_get_block_by_name(&block.parameter_name), first);
        }
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
    // Text with more than spaces counts as text, and so does code.
    let summary = NodeRef::Section(&context.doc_comment.summary_section);
    let texty = walk(summary).iter().any(|node| match node {
        NodeRef::PlainText(text) => text.text.trim() != "",
        NodeRef::FencedCode(code) => code.code.trim() != "",
        _ => false,
    });
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
        let again = parser.parse_string(&emitted);
        assert!(again.is_ok_and(|again| again.log.iter().all(|m| m.message_id != TSDocMessageId::CommentNotFound)));
    }
}

/// Ignoring undefined tags leaves out their messages and changes nothing else: the same tree, and
/// the same other messages, or the same error.
#[hegel::test(test_cases = 100)]
fn ignoring_undefined_tags_drops_only_their_messages(tc: TestCase) {
    let comment = comment(&tc);
    let Ok(mut configuration) = configuration(config(&tc).as_ref()) else { return };
    let reported = TSDocParser::new(&configuration).parse_string(&comment);
    configuration.validation.ignore_undefined_tags = true;
    let ignored = TSDocParser::new(&configuration).parse_string(&comment);
    match (reported, ignored) {
        (Ok(reported), Ok(ignored)) => {
            let others: Vec<_> =
                reported.log.iter().filter(|m| m.message_id != TSDocMessageId::UndefinedTag).cloned().collect();
            assert_eq!(ignored.log, others, "{comment:?}");
            assert_eq!(ignored.doc_comment, reported.doc_comment, "{comment:?}");
        }
        (Err(reported), Err(ignored)) => assert_eq!(ignored.message, reported.message, "{comment:?}"),
        (reported, ignored) => panic!("{comment:?}: {reported:?} but {ignored:?}"),
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
        let found = configuration.try_get_tag_definition(&name.to_uppercase()).unwrap().clone();
        assert_eq!(found.tag_name, *name);
        assert_eq!(configuration.is_tag_supported(&found), supported == Some(true));
        // The same definition again is fine; another of that name isn't.
        configuration.add_tag_definition(found.clone()).unwrap();
        let other = TSDocTagDefinition::new(&name.to_lowercase(), TSDocTagSyntaxKind::BlockTag, true).unwrap();
        assert!(other == found || configuration.add_tag_definition(other).is_err());
    }
    let supported_names: Vec<&str> =
        configuration.supported_tag_definitions().map(|tag| tag.tag_name.as_str()).collect();
    assert_eq!(supported_names.len(), if supported == Some(true) { names.len() } else { 0 });
    // Marking a tag supported or not turns on the report, as in TSDoc; with no tags nothing is marked.
    assert_eq!(configuration.validation.report_unsupported_tags, supported.is_some() && !names.is_empty());
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
    // A link with no destination writes its text alone, as TSDoc's emitter does.
    let text = tc.draw(gs::sampled_from(vec!["a", "two words"]));
    let bare = DocLinkTag { url_destination: None, link_text: Some(text.into()), ..link.clone() };
    let mut comment = DocComment::default();
    comment.summary_section.nodes.push(DocParagraph { nodes: vec![bare.into()] }.into());
    assert!(comment.emit_as_tsdoc().contains(&format!("{{@link | {text}}}")), "{}", comment.emit_as_tsdoc());
    // A member with neither an identifier nor a symbol writes nothing but its punctuation.
    let selector = tc.draw(gs::optional(gs::sampled_from(vec!["static", "1"])));
    let member =
        DocMemberReference { has_dot: true, selector: selector.map(DocMemberSelector::new), ..Default::default() };
    let reference = DocDeclarationReference { member_references: vec![member], ..Default::default() };
    let wanted = selector.map_or(".".to_owned(), |selector| format!(".(:{selector})"));
    assert_eq!(reference.emit_as_tsdoc(), wanted);
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
    // A tag built without spacing gets a space before each attribute, and reads back the same.
    let attribute =
        |name: &str, value: &str| DocHtmlAttribute { name: name.into(), value: value.into(), ..Default::default() };
    let attributes = vec![attribute("src", "\"x\""), attribute(word.split(' ').next().unwrap_or("a"), "'y'")];
    let tag = DocHtmlStartTag { name: "img".into(), html_attributes: attributes, ..Default::default() };
    let html = tag.emit_as_html();
    assert!(html.starts_with("<img src=\"x\" ") && html.ends_with("'y'>"), "{html}");
}
