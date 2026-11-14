//! The node parser: `@microsoft/tsdoc`'s NodeParser, which reads the tokens into the comment's
//! tree. Each construct is read the way that parser reads it, so a mistake becomes the same
//! error text and message, and the text after it reads the same.

use super::{
    ParseError, ParsedTag, ParserContext, ParserMessage, TSDocMessageId as Id, TextRange, Token, TokenKind,
    TokenSequence, json_string, string_checks, token_reader::TokenReader, tokenizer,
};
use crate::{
    TSDocConfiguration, TSDocTagDefinition, TSDocTagSyntaxKind,
    emitters::plain_text_emitter::has_any_text_content,
    nodes::{
        DocBlock, DocBlockTag, DocCodeSpan, DocComment, DocDeclarationReference, DocErrorText, DocEscapedText,
        DocExcerpt, DocFencedCode, DocHtmlAttribute, DocHtmlEndTag, DocHtmlStartTag, DocInheritDocTag, DocInlineTag,
        DocLinkTag, DocMemberIdentifier, DocMemberReference, DocMemberSelector, DocMemberSymbol, DocNode,
        DocParamBlock, DocPlainText, DocSection, DocSoftBreak, EscapeStyle, ExcerptKind as Kind, FencedCodeExcerpts,
        HtmlAttributeExcerpts, HtmlTagExcerpts, InlineTagBase, LinkTagExcerpts, MemberReferenceExcerpts, NodeRef,
        ParamBlockExcerpts, paragraph_allows,
    },
};

type Result<T> = std::result::Result<T, ParseError>;

/// Why a construct couldn't be read, before it's reported.
struct Failure {
    id: Id,
    message: String,
    location: TokenSequence,
}

/// The section text goes into now.
#[derive(Clone, Copy)]
enum Target {
    Summary,
    Remarks,
    PrivateRemarks,
    Deprecated,
    Returns,
    Custom(usize),
    See(usize),
    Param(usize),
    TypeParam(usize),
}

/// What an inline tag's opening, name, spacing, content and closing were read as.
struct InlineTagParts {
    tag_name: String,
    opening_delimiter: TokenSequence,
    tag_name_excerpt: TokenSequence,
    spacing_after_tag_name: Option<TokenSequence>,
    tag_content: Option<TokenSequence>,
    closing_delimiter: TokenSequence,
}

/// Parses `context`'s tokens into its comment, logging what's wrong.
pub(super) fn parse(context: &mut ParserContext, configuration: &TSDocConfiguration) -> Result<()> {
    let ParserContext { buffer, tokens, doc_comment, log, comment_range, tags, .. } = context;
    let mut parser = NodeParser {
        buffer,
        tokens,
        configuration,
        log,
        tags,
        doc_comment,
        comment_range: *comment_range,
        target: Target::Summary,
    };
    parser.parse()
}

struct NodeParser<'a> {
    buffer: &'a str,
    tokens: &'a [Token],
    configuration: &'a TSDocConfiguration,
    log: &'a mut Vec<ParserMessage>,
    tags: &'a mut Vec<ParsedTag>,
    doc_comment: &'a mut DocComment,
    comment_range: TextRange,
    target: Target,
}

/// An excerpt of `kind`, of `content` if there is some.
fn excerpt(kind: Kind, content: Option<TokenSequence>) -> Option<DocExcerpt> {
    DocExcerpt::of(kind, content)
}

impl<'a> NodeParser<'a> {
    fn parse(&mut self) -> Result<()> {
        let mut reader = TokenReader::new(self.tokens, None);
        loop {
            match reader.peek_token_kind() {
                TokenKind::EndOfInput => break,
                TokenKind::Newline => {
                    self.push_accumulated_plain_text(&mut reader);
                    reader.read_token()?;
                    let soft_break = reader.extract_accumulated_sequence();
                    let excerpt = excerpt(Kind::SoftBreak, Some(soft_break));
                    self.push_node(DocSoftBreak { soft_break_excerpt: excerpt }.into());
                }
                TokenKind::Backslash => {
                    self.push_accumulated_plain_text(&mut reader);
                    let node = self.parse_backslash_escape(&mut reader)?;
                    self.push_node(node);
                }
                TokenKind::AtSign => {
                    self.push_accumulated_plain_text(&mut reader);
                    self.parse_and_push_block(&mut reader)?;
                }
                TokenKind::LeftCurlyBracket => {
                    self.push_accumulated_plain_text(&mut reader);
                    let marker = reader.create_marker();
                    match self.parse_inline_tag(&mut reader)? {
                        DocNode::InheritDocTag(tag) if self.doc_comment.inherit_doc_tag.is_none() => {
                            self.doc_comment.inherit_doc_tag = Some(tag);
                        }
                        DocNode::InheritDocTag(_) => {
                            let tag_end = reader.create_marker() - 1;
                            let text = "A doc comment cannot have more than one @inheritDoc tag";
                            let error = self.backtrack_and_create_error_range(
                                &mut reader,
                                marker,
                                tag_end,
                                Id::ExtraInheritDocTag,
                                text,
                            )?;
                            self.push_node(error);
                        }
                        node => self.push_node(node),
                    }
                }
                TokenKind::RightCurlyBracket => {
                    self.push_accumulated_plain_text(&mut reader);
                    let text = "The \"}\" character should be escaped using a backslash to avoid confusion with a TSDoc inline tag";
                    let error = self.create_error(&mut reader, Id::EscapeRightBrace, text.into())?;
                    self.push_node(error);
                }
                TokenKind::LessThan => {
                    self.push_accumulated_plain_text(&mut reader);
                    let node = if reader.peek_token_after_kind() == TokenKind::Slash {
                        self.parse_html_end_tag(&mut reader)?
                    } else {
                        self.parse_html_start_tag(&mut reader)?
                    };
                    self.push_node(node);
                }
                TokenKind::GreaterThan => {
                    self.push_accumulated_plain_text(&mut reader);
                    let text =
                        "The \">\" character should be escaped using a backslash to avoid confusion with an HTML tag";
                    let error = self.create_error(&mut reader, Id::EscapeGreaterThan, text.into())?;
                    self.push_node(error);
                }
                TokenKind::Backtick => {
                    self.push_accumulated_plain_text(&mut reader);
                    let fence = reader.peek_token_after_kind() == TokenKind::Backtick
                        && reader.peek_token_after_after_kind() == TokenKind::Backtick;
                    let node =
                        if fence { self.parse_fenced_code(&mut reader)? } else { self.parse_code_span(&mut reader)? };
                    self.push_node(node);
                }
                _ => {
                    reader.read_token()?;
                }
            }
        }
        self.push_accumulated_plain_text(&mut reader);
        self.perform_validation_checks();
        Ok(())
    }

    /// The checks of the whole comment, once it's read.
    fn perform_validation_checks(&mut self) {
        if let Some(block) = &self.doc_comment.deprecated_block
            && !has_any_text_content([NodeRef::Block(block)], 1)
        {
            let text = format!(
                "The {} block must include a deprecation message, e.g. describing the recommended alternative",
                block.block_tag.tag_name
            );
            let sequence = block.block_tag.token_sequence().unwrap_or_default();
            self.log_sequence(Id::MissingDeprecationMessage, text, sequence);
        }
        if self.doc_comment.inherit_doc_tag.is_none() {
            return;
        }
        if let Some(block) = &self.doc_comment.remarks_block {
            let text = format!(
                "A \"{}\" block must not be used, because that content is provided by the @inheritDoc tag",
                block.block_tag.tag_name
            );
            let sequence = block.block_tag.token_sequence().unwrap_or_default();
            self.log_sequence(Id::InheritDocIncompatibleTag, text, sequence);
        }
        if has_any_text_content([NodeRef::Section(&self.doc_comment.summary_section)], 1) {
            let text = "The summary section must not have any content, because that content is provided by the @inheritDoc tag";
            self.log_range(Id::InheritDocIncompatibleSummary, text.into(), self.comment_range);
        }
    }

    fn text(&self, sequence: TokenSequence) -> String {
        sequence.text(self.buffer, self.tokens)
    }

    fn log_sequence(&mut self, id: Id, text: String, sequence: TokenSequence) {
        let range = sequence.containing_text_range(self.tokens);
        self.log.push(ParserMessage::new(self.buffer, id, text, range, Some(sequence)));
    }

    fn log_range(&mut self, id: Id, text: String, range: TextRange) {
        self.log.push(ParserMessage::new(self.buffer, id, text, range, None));
    }

    fn validate_tag_definition(
        &mut self,
        definition: Option<&TSDocTagDefinition>,
        tag_name: &str,
        expecting_inline_tag: bool,
        sequence: TokenSequence,
    ) {
        self.tags.push(ParsedTag { tag_name: tag_name.into(), inline: expecting_inline_tag, token_sequence: sequence });
        let Some(definition) = definition else {
            if !self.configuration.validation.ignore_undefined_tags {
                let text = format!("The TSDoc tag \"{tag_name}\" is not defined in this configuration");
                self.log_sequence(Id::UndefinedTag, text, sequence);
            }
            return;
        };
        let is_inline_tag = definition.syntax_kind == TSDocTagSyntaxKind::InlineTag;
        if is_inline_tag != expecting_inline_tag {
            if expecting_inline_tag {
                let text = format!(
                    "The TSDoc tag \"{tag_name}\" is not an inline tag; it must not be enclosed in \"{{ }}\" braces"
                );
                self.log_sequence(Id::TagShouldNotHaveBraces, text, sequence);
            } else {
                let text =
                    format!("The TSDoc tag \"{tag_name}\" is an inline tag; it must be enclosed in \"{{ }}\" braces");
                self.log_sequence(Id::InlineTagMissingBraces, text, sequence);
            }
        } else if self.configuration.validation.report_unsupported_tags
            && !self.configuration.is_tag_supported(definition)
        {
            let text = format!("The TSDoc tag \"{tag_name}\" is not supported by this tool");
            self.log_sequence(Id::UnsupportedTag, text, sequence);
        }
    }

    fn push_accumulated_plain_text(&mut self, reader: &mut TokenReader) {
        if let Some(sequence) = reader.try_extract_accumulated_sequence() {
            let text_excerpt = excerpt(Kind::PlainText, Some(sequence));
            self.push_node(DocPlainText { text: self.text(sequence), text_excerpt }.into());
        }
    }

    fn section(&mut self) -> &mut DocSection {
        let comment = &mut *self.doc_comment;
        let block = match self.target {
            Target::Summary => return &mut comment.summary_section,
            Target::Param(at) => return &mut comment.params.blocks[at].content,
            Target::TypeParam(at) => return &mut comment.type_params.blocks[at].content,
            Target::Remarks => comment.remarks_block.as_mut(),
            Target::PrivateRemarks => comment.private_remarks.as_mut(),
            Target::Deprecated => comment.deprecated_block.as_mut(),
            Target::Returns => comment.returns_block.as_mut(),
            Target::Custom(at) => comment.custom_blocks.get_mut(at),
            Target::See(at) => comment.see_blocks.get_mut(at),
        };
        &mut block.expect("the target is a block the comment has").content
    }

    fn push_node(&mut self, node: DocNode) {
        let section = self.section();
        if paragraph_allows(node.kind()) {
            section.append_node_in_paragraph(node);
        } else {
            section.nodes.push(node);
        }
    }

    fn parse_and_push_block(&mut self, reader: &mut TokenReader) -> Result<()> {
        let tag = match self.parse_block_tag(reader)? {
            DocNode::BlockTag(tag) => tag,
            node => {
                self.push_node(node);
                return Ok(());
            }
        };
        let configuration = self.configuration;
        let definition = configuration.try_get_tag_definition_with_upper_case(&tag.tag_name_with_upper_case);
        let sequence = tag.token_sequence().unwrap_or_default();
        self.validate_tag_definition(definition, &tag.tag_name, false, sequence);
        match definition.map(|definition| definition.syntax_kind) {
            Some(TSDocTagSyntaxKind::BlockTag) => {
                let comment = &mut *self.doc_comment;
                match tag.tag_name_with_upper_case.as_str() {
                    "@PARAM" => {
                        let block = self.parse_param_block(reader, tag, "@param")?;
                        let params = &mut self.doc_comment.params.blocks;
                        params.push(block);
                        self.target = Target::Param(params.len() - 1);
                    }
                    "@TYPEPARAM" => {
                        let block = self.parse_param_block(reader, tag, "@typeParam")?;
                        let params = &mut self.doc_comment.type_params.blocks;
                        params.push(block);
                        self.target = Target::TypeParam(params.len() - 1);
                    }
                    upper => {
                        let (slot, target) = match upper {
                            "@REMARKS" => (&mut comment.remarks_block, Target::Remarks),
                            "@PRIVATEREMARKS" => (&mut comment.private_remarks, Target::PrivateRemarks),
                            "@DEPRECATED" => (&mut comment.deprecated_block, Target::Deprecated),
                            "@RETURNS" => (&mut comment.returns_block, Target::Returns),
                            "@SEE" => {
                                comment.see_blocks.push(DocBlock::new(tag));
                                self.target = Target::See(comment.see_blocks.len() - 1);
                                return Ok(());
                            }
                            _ => {
                                comment.custom_blocks.push(DocBlock::new(tag));
                                self.target = Target::Custom(comment.custom_blocks.len() - 1);
                                return Ok(());
                            }
                        };
                        *slot = Some(DocBlock::new(tag));
                        self.target = target;
                    }
                }
                Ok(())
            }
            Some(TSDocTagSyntaxKind::ModifierTag) => {
                self.doc_comment.modifier_tag_set.add_tag(tag);
                Ok(())
            }
            _ => {
                self.push_node(tag.into());
                Ok(())
            }
        }
    }

    /// The rest of a JSDoc type or default value after its opening `open_kind`, up to the
    /// `close_kind` that matches it, past nested pairs and quoted strings, or none when it isn't
    /// closed, back at `start_marker`.
    fn try_parse_jsdoc_type_or_value_rest(
        &mut self,
        reader: &mut TokenReader,
        open_kind: TokenKind,
        close_kind: TokenKind,
        start_marker: usize,
    ) -> Result<Option<TokenSequence>> {
        let mut quote_kind: Option<TokenKind> = None;
        let mut open_count = 1;
        while open_count > 0 {
            let mut kind = reader.peek_token_kind();
            match kind {
                _ if kind == open_kind => open_count += usize::from(quote_kind.is_none()),
                _ if kind == close_kind => open_count -= usize::from(quote_kind.is_none()),
                TokenKind::Backslash if quote_kind.is_some() => {
                    // The backslash and the character it escapes.
                    reader.read_token()?;
                    kind = reader.peek_token_kind();
                }
                TokenKind::DoubleQuote | TokenKind::SingleQuote | TokenKind::Backtick => {
                    if quote_kind == Some(kind) {
                        quote_kind = None;
                    } else if quote_kind.is_none() {
                        quote_kind = Some(kind);
                    }
                }
                _ => {}
            }
            if kind == TokenKind::EndOfInput {
                reader.backtrack_to_marker(start_marker);
                return Ok(None);
            }
            reader.read_token()?;
        }
        Ok(reader.try_extract_accumulated_sequence())
    }

    /// A JSDoc `{type}` in a `@param` block, which TSDoc reports and skips.
    fn try_parse_unsupported_jsdoc_type(
        &mut self,
        reader: &mut TokenReader,
        tag_name: &str,
    ) -> Result<Option<TokenSequence>> {
        reader.assert_accumulated_sequence_is_empty();
        // `{@` starts an inline tag, not a type.
        if reader.peek_token_kind() != TokenKind::LeftCurlyBracket
            || reader.peek_token_after_kind() == TokenKind::AtSign
        {
            return Ok(None);
        }
        let start_marker = reader.create_marker();
        reader.read_token()?;
        let rest = self.try_parse_jsdoc_type_or_value_rest(
            reader,
            TokenKind::LeftCurlyBracket,
            TokenKind::RightCurlyBracket,
            start_marker,
        )?;
        let Some(jsdoc_type) = rest else { return Ok(None) };
        let text = format!("The {tag_name} block should not include a JSDoc-style '{{type}}'");
        self.log_sequence(Id::ParamTagWithInvalidType, text, jsdoc_type);
        Ok(Some(match self.try_read_spacing_and_newlines(reader)? {
            Some(spacing) => TokenSequence::new(jsdoc_type.start_index, spacing.end_index),
            None => jsdoc_type,
        }))
    }

    /// The rest of a JSDoc optional name after its `[`, like `=[]]`.
    fn try_parse_jsdoc_optional_name_rest(&mut self, reader: &mut TokenReader) -> Result<Option<TokenSequence>> {
        reader.assert_accumulated_sequence_is_empty();
        if reader.peek_token_kind() == TokenKind::EndOfInput {
            return Ok(None);
        }
        let start_marker = reader.create_marker();
        self.try_parse_jsdoc_type_or_value_rest(
            reader,
            TokenKind::LeftSquareBracket,
            TokenKind::RightSquareBracket,
            start_marker,
        )
    }

    fn parse_param_block(
        &mut self,
        reader: &mut TokenReader,
        tag: DocBlockTag,
        tag_name: &str,
    ) -> Result<DocParamBlock> {
        let start_marker = reader.create_marker();
        let spacing_before_parameter_name = self.try_read_spacing_and_newlines(reader)?;
        let unsupported_jsdoc_type_before_parameter_name = self.try_parse_unsupported_jsdoc_type(reader, tag_name)?;
        let mut unsupported_jsdoc_optional_name_open_bracket = None;
        if reader.peek_token_kind() == TokenKind::LeftSquareBracket {
            reader.read_token()?;
            unsupported_jsdoc_optional_name_open_bracket = Some(reader.extract_accumulated_sequence());
        }
        let mut parameter_name = String::new();
        while matches!(reader.peek_token_kind(), TokenKind::AsciiWord | TokenKind::Period | TokenKind::DollarSign) {
            parameter_name += reader.read_token()?.text(self.buffer);
        }
        if let Some(explanation) = string_checks::explain_if_invalid_unquoted_identifier(&parameter_name) {
            reader.backtrack_to_marker(start_marker);
            let text = if parameter_name.is_empty() {
                format!("The {tag_name} block should be followed by a parameter name")
            } else {
                format!("The {tag_name} block should be followed by a valid parameter name: {explanation}")
            };
            self.log_sequence(Id::ParamTagWithInvalidName, text, tag.token_sequence().unwrap_or_default());
            return Ok(DocParamBlock::new(tag, ""));
        }
        let parameter_name_excerpt = reader.extract_accumulated_sequence();
        let mut unsupported_jsdoc_optional_name_rest = None;
        if let Some(open_bracket) = unsupported_jsdoc_optional_name_open_bracket {
            unsupported_jsdoc_optional_name_rest = self.try_parse_jsdoc_optional_name_rest(reader)?;
            let error = match unsupported_jsdoc_optional_name_rest {
                Some(rest) => TokenSequence::new(open_bracket.start_index, rest.end_index),
                None => open_bracket,
            };
            let text = format!(
                "The {tag_name} should not include a JSDoc-style optional name; it must not be enclosed in '[ ]' brackets."
            );
            self.log_sequence(Id::ParamTagWithInvalidOptionalName, text, error);
        }
        let spacing_after_parameter_name = self.try_read_spacing_and_newlines(reader)?;
        let unsupported_jsdoc_type_after_parameter_name = self.try_parse_unsupported_jsdoc_type(reader, tag_name)?;
        let (mut hyphen, mut spacing_after_hyphen, mut unsupported_jsdoc_type_after_hyphen) = (None, None, None);
        if reader.peek_token_kind() == TokenKind::Hyphen {
            reader.read_token()?;
            hyphen = Some(reader.extract_accumulated_sequence());
            spacing_after_hyphen = self.try_read_spacing_and_newlines(reader)?;
            unsupported_jsdoc_type_after_hyphen = self.try_parse_unsupported_jsdoc_type(reader, tag_name)?;
        } else {
            let text = format!("The {tag_name} block should be followed by a parameter name and then a hyphen");
            self.log_sequence(Id::ParamTagMissingHyphen, text, tag.token_sequence().unwrap_or_default());
        }
        let excerpts = ParamBlockExcerpts {
            spacing_before_parameter_name: excerpt(Kind::Spacing, spacing_before_parameter_name),
            unsupported_jsdoc_type_before_parameter_name: excerpt(
                Kind::NonstandardText,
                unsupported_jsdoc_type_before_parameter_name,
            ),
            unsupported_jsdoc_optional_name_open_bracket: excerpt(
                Kind::NonstandardText,
                unsupported_jsdoc_optional_name_open_bracket,
            ),
            parameter_name: excerpt(Kind::ParamBlock_ParameterName, Some(parameter_name_excerpt)),
            unsupported_jsdoc_optional_name_rest: excerpt(Kind::NonstandardText, unsupported_jsdoc_optional_name_rest),
            spacing_after_parameter_name: excerpt(Kind::Spacing, spacing_after_parameter_name),
            unsupported_jsdoc_type_after_parameter_name: excerpt(
                Kind::NonstandardText,
                unsupported_jsdoc_type_after_parameter_name,
            ),
            hyphen: excerpt(Kind::ParamBlock_Hyphen, hyphen),
            spacing_after_hyphen: excerpt(Kind::Spacing, spacing_after_hyphen),
            unsupported_jsdoc_type_after_hyphen: excerpt(Kind::NonstandardText, unsupported_jsdoc_type_after_hyphen),
        };
        Ok(DocParamBlock { block_tag: tag, parameter_name, content: DocSection::default(), excerpts })
    }

    fn parse_backslash_escape(&mut self, reader: &mut TokenReader) -> Result<DocNode> {
        reader.assert_accumulated_sequence_is_empty();
        let marker = reader.create_marker();
        reader.read_token()?;
        if reader.peek_token_kind() == TokenKind::EndOfInput {
            let text = "A backslash must precede another character that is being escaped";
            return self.backtrack_and_create_error(reader, marker, Id::UnnecessaryBackslash, text);
        }
        let escaped = reader.read_token()?;
        // CommonMark only escapes punctuation; anywhere else a backslash is a backslash.
        if !tokenizer::is_punctuation(escaped.kind) {
            let text = "A backslash can only be used to escape a punctuation character";
            return self.backtrack_and_create_error(reader, marker, Id::UnnecessaryBackslash, text);
        }
        let encoded = reader.extract_accumulated_sequence();
        Ok(DocEscapedText {
            escape_style: EscapeStyle::CommonMarkBackslash,
            encoded_text: self.text(encoded),
            encoded_text_excerpt: DocExcerpt::new(Kind::EscapedText, encoded),
            decoded_text: escaped.text(self.buffer).into(),
        }
        .into())
    }

    fn parse_block_tag(&mut self, reader: &mut TokenReader) -> Result<DocNode> {
        reader.assert_accumulated_sequence_is_empty();
        let marker = reader.create_marker();
        // "@one" starts a tag at the start of a line or after a space; "@one@two" is a mistake.
        if !matches!(reader.peek_previous_token_kind(), TokenKind::EndOfInput | TokenKind::Spacing | TokenKind::Newline)
        {
            let text = "The \"@\" character looks like part of a TSDoc tag; use a backslash to escape it";
            return self.backtrack_and_create_error(reader, marker, Id::AtSignInWord, text);
        }
        let mut tag_name = reader.read_token()?.text(self.buffer).to_owned();
        if reader.peek_token_kind() != TokenKind::AsciiWord {
            let text =
                "Expecting a TSDoc tag name after \"@\"; if it is not a tag, use a backslash to escape this character";
            return self.backtrack_and_create_error(reader, marker, Id::AtSignWithoutTagName, text);
        }
        let tag_name_marker = reader.create_marker();
        while reader.peek_token_kind() == TokenKind::AsciiWord {
            tag_name += reader.read_token()?.text(self.buffer);
        }
        if !matches!(reader.peek_token_kind(), TokenKind::Spacing | TokenKind::Newline | TokenKind::EndOfInput) {
            let bad = first_code_unit_json(reader.peek_token().range.text(self.buffer));
            let text = format!(
                "The token \"{tag_name}\" looks like a TSDoc tag but contains an invalid character {bad}; if it is not a tag, use a backslash to escape the \"@\""
            );
            return self.backtrack_and_create_error(reader, marker, Id::CharactersAfterBlockTag, &text);
        }
        if string_checks::explain_if_invalid_tsdoc_tag_name(&tag_name).is_some() {
            let failure = self.failure_for_tokens_since(reader, Id::MalformedTagName, TAG_NAME.into(), tag_name_marker);
            return self.backtrack_and_create_error_for_failure(reader, marker, "", failure);
        }
        let excerpt = reader.extract_accumulated_sequence();
        Ok(DocBlockTag::parsed(tag_name, Some(excerpt)).into())
    }

    fn parse_inline_tag(&mut self, reader: &mut TokenReader) -> Result<DocNode> {
        reader.assert_accumulated_sequence_is_empty();
        let marker = reader.create_marker();
        reader.read_token()?;
        let opening_delimiter = reader.extract_accumulated_sequence();
        // A mistake from here on takes the "{" and "@" both, or the main loop would read the "@" as
        // a block tag.
        let at_sign_marker = reader.create_marker();
        if reader.peek_token_kind() != TokenKind::AtSign {
            let text = "Expecting a TSDoc tag starting with \"{@\"";
            return self.backtrack_and_create_error(reader, marker, Id::MalformedInlineTag, text);
        }
        let mut tag_name = reader.read_token()?.text(self.buffer).to_owned();
        while reader.peek_token_kind() == TokenKind::AsciiWord {
            tag_name += reader.read_token()?.text(self.buffer);
        }
        if tag_name == "@" {
            let text = "Expecting a TSDoc inline tag name after the \"{@\" characters";
            let failure = self.failure_for_tokens_since(reader, Id::MalformedInlineTag, text.into(), at_sign_marker);
            return self.backtrack_and_create_error_range_for_failure(reader, marker, at_sign_marker, "", failure);
        }
        if string_checks::explain_if_invalid_tsdoc_tag_name(&tag_name).is_some() {
            let failure = self.failure_for_tokens_since(reader, Id::MalformedTagName, TAG_NAME.into(), at_sign_marker);
            return self.backtrack_and_create_error_range_for_failure(reader, marker, at_sign_marker, "", failure);
        }
        let tag_name_excerpt = reader.extract_accumulated_sequence();
        let spacing_after_tag_name = self.try_read_spacing_and_newlines(reader)?;
        // No space at all is a mistake, but in `{@tag}`.
        if spacing_after_tag_name.is_none() && reader.peek_token_kind() != TokenKind::RightCurlyBracket {
            let bad = first_code_unit_json(reader.peek_token().range.text(self.buffer));
            let text = format!("The character {bad} cannot appear after the TSDoc tag name; expecting a space");
            let failure = self.failure_for_token(reader, Id::CharactersAfterInlineTag, text, None);
            return self.backtrack_and_create_error_range_for_failure(reader, marker, at_sign_marker, "", failure);
        }
        loop {
            match reader.peek_token_kind() {
                TokenKind::EndOfInput => {
                    let text = "The TSDoc inline tag name is missing its closing \"}\"";
                    let id = Id::InlineTagMissingRightBrace;
                    return self.backtrack_and_create_error_range(reader, marker, at_sign_marker, id, text);
                }
                TokenKind::Backslash => {
                    // A backslash in an inline tag escapes a "}" (http://usejsdoc.org/about-block-inline-tags.html),
                    // and, as in CommonMark, only punctuation.
                    reader.read_token()?;
                    if !tokenizer::is_punctuation(reader.peek_token_kind()) {
                        let text = "A backslash can only be used to escape a punctuation character";
                        let failure = self.failure_for_token(reader, Id::UnnecessaryBackslash, text.into(), None);
                        let prefix = "Error reading inline TSDoc tag: ";
                        return self.backtrack_and_create_error_range_for_failure(
                            reader,
                            marker,
                            at_sign_marker,
                            prefix,
                            failure,
                        );
                    }
                    reader.read_token()?;
                }
                TokenKind::LeftCurlyBracket => {
                    let text =
                        "The \"{\" character must be escaped with a backslash when used inside a TSDoc inline tag";
                    let failure = self.failure_for_token(reader, Id::InlineTagUnescapedBrace, text.into(), None);
                    return self.backtrack_and_create_error_range_for_failure(
                        reader,
                        marker,
                        at_sign_marker,
                        "",
                        failure,
                    );
                }
                TokenKind::RightCurlyBracket => break,
                _ => {
                    reader.read_token()?;
                }
            }
        }
        let tag_content = reader.try_extract_accumulated_sequence();
        reader.read_token()?;
        let closing_delimiter = reader.extract_accumulated_sequence();
        let parts = InlineTagParts {
            tag_name,
            opening_delimiter,
            tag_name_excerpt,
            spacing_after_tag_name,
            tag_content,
            closing_delimiter,
        };
        let upper = parts.tag_name.to_uppercase();
        let mut embedded = TokenReader::new(self.tokens, Some(tag_content.unwrap_or_default()));
        let node = match upper.as_str() {
            "@INHERITDOC" => self.parse_inherit_doc_tag(&parts, &mut embedded)?,
            "@LINK" => self.parse_link_tag(&parts, &mut embedded)?,
            _ => self.inline_tag(&parts).into(),
        };
        let configuration = self.configuration;
        let definition = configuration.try_get_tag_definition_with_upper_case(&upper);
        self.validate_tag_definition(definition, &parts.tag_name, true, parts.tag_name_excerpt);
        Ok(node)
    }

    fn inline_tag_base(&self, parts: &InlineTagParts) -> InlineTagBase {
        InlineTagBase {
            tag_name: parts.tag_name.clone(),
            tag_name_with_upper_case: parts.tag_name.to_uppercase(),
            opening_delimiter_excerpt: excerpt(Kind::InlineTag_OpeningDelimiter, Some(parts.opening_delimiter)),
            tag_name_excerpt: excerpt(Kind::InlineTag_TagName, Some(parts.tag_name_excerpt)),
            spacing_after_tag_name_excerpt: excerpt(Kind::Spacing, parts.spacing_after_tag_name),
            closing_delimiter_excerpt: excerpt(Kind::InlineTag_ClosingDelimiter, Some(parts.closing_delimiter)),
        }
    }

    fn inline_tag(&self, parts: &InlineTagParts) -> DocInlineTag {
        DocInlineTag {
            base: self.inline_tag_base(parts),
            tag_content: parts.tag_content.map(|content| self.text(content)).unwrap_or_default(),
            tag_content_excerpt: excerpt(Kind::InlineTag_TagContent, parts.tag_content),
        }
    }

    fn parse_inherit_doc_tag(&mut self, parts: &InlineTagParts, embedded: &mut TokenReader) -> Result<DocNode> {
        let mut declaration_reference = None;
        if embedded.peek_token_kind() != TokenKind::EndOfInput {
            declaration_reference = self.parse_declaration_reference(embedded, parts.tag_name_excerpt)?;
            if declaration_reference.is_none() {
                return Ok(self.inline_tag(parts).into());
            }
            if embedded.peek_token_kind() != TokenKind::EndOfInput {
                embedded.read_token()?;
                let sequence = embedded.extract_accumulated_sequence();
                let text = "Unexpected character after declaration reference";
                self.log_sequence(Id::InheritDocTagSyntax, text.into(), sequence);
                return Ok(self.inline_tag(parts).into());
            }
        }
        Ok(DocInheritDocTag { base: self.inline_tag_base(parts), declaration_reference }.into())
    }

    fn parse_link_tag(&mut self, parts: &InlineTagParts, embedded: &mut TokenReader) -> Result<DocNode> {
        if parts.tag_content.is_none() {
            let text = "The @link tag content is missing";
            self.log_sequence(Id::LinkTagEmpty, text.into(), parts.tag_name_excerpt);
            return Ok(self.inline_tag(parts).into());
        }
        // A URL starts with "//", or with a scheme like "https:" and then "//".
        let slashes = |reader: &TokenReader| {
            reader.peek_token_kind() == TokenKind::Slash && reader.peek_token_after_kind() == TokenKind::Slash
        };
        let mut looks_like_url = slashes(embedded);
        let marker = embedded.create_marker();
        let mut done = looks_like_url;
        while !done {
            match embedded.peek_token_kind() {
                TokenKind::AsciiWord | TokenKind::Period | TokenKind::Hyphen | TokenKind::Plus => {
                    embedded.read_token()?;
                }
                TokenKind::Colon => {
                    embedded.read_token()?;
                    looks_like_url = slashes(embedded);
                    done = true;
                }
                _ => done = true,
            }
        }
        embedded.backtrack_to_marker(marker);
        let mut excerpts = LinkTagExcerpts::default();
        let (mut code_destination, mut url_destination) = (None, None);
        if looks_like_url {
            match self.parse_link_tag_url_destination(embedded)? {
                Some((url, spacing)) => {
                    url_destination = Some(url);
                    excerpts.spacing_after_destination = excerpt(Kind::Spacing, spacing);
                }
                None => return Ok(self.inline_tag(parts).into()),
            }
        } else {
            code_destination = self.parse_declaration_reference(embedded, parts.tag_name_excerpt)?;
            if code_destination.is_none() {
                return Ok(self.inline_tag(parts).into());
            }
        }
        if embedded.peek_token_kind() == TokenKind::Spacing {
            return Err(ParseError::new("Unconsumed spacing encountered after construct"));
        }
        let mut link_text = None;
        if embedded.peek_token_kind() == TokenKind::Pipe {
            embedded.read_token()?;
            excerpts.pipe = excerpt(Kind::LinkTag_Pipe, Some(embedded.extract_accumulated_sequence()));
            excerpts.spacing_after_pipe = excerpt(Kind::Spacing, self.try_read_spacing_and_newlines(embedded)?);
            let mut spacing_after_link_text_marker = None;
            loop {
                match embedded.peek_token_kind() {
                    TokenKind::EndOfInput => break,
                    TokenKind::Pipe | TokenKind::LeftCurlyBracket => {
                        let bad = embedded.read_token()?.text(self.buffer).to_owned();
                        let sequence = embedded.extract_accumulated_sequence();
                        let text =
                            format!("The \"{bad}\" character may not be used in the link text without escaping it");
                        self.log_sequence(Id::LinkTagUnescapedText, text, sequence);
                        return Ok(self.inline_tag(parts).into());
                    }
                    TokenKind::Spacing | TokenKind::Newline => {
                        embedded.read_token()?;
                    }
                    _ => {
                        spacing_after_link_text_marker = Some(embedded.create_marker() + 1);
                        embedded.read_token()?;
                    }
                }
            }
            if let Some(text_and_spacing) = embedded.try_extract_accumulated_sequence() {
                match spacing_after_link_text_marker {
                    None => excerpts.spacing_after_link_text = excerpt(Kind::Spacing, Some(text_and_spacing)),
                    Some(marker) if marker >= text_and_spacing.end_index => link_text = Some(text_and_spacing),
                    Some(marker) => {
                        link_text = Some(TokenSequence::new(text_and_spacing.start_index, marker));
                        let spacing = TokenSequence::new(marker, text_and_spacing.end_index);
                        excerpts.spacing_after_link_text = excerpt(Kind::Spacing, Some(spacing));
                    }
                }
            }
        } else if embedded.peek_token_kind() != TokenKind::EndOfInput {
            embedded.read_token()?;
            let sequence = embedded.extract_accumulated_sequence();
            self.log_sequence(
                Id::LinkTagDestinationSyntax,
                "Unexpected character after link destination".into(),
                sequence,
            );
            return Ok(self.inline_tag(parts).into());
        }
        excerpts.url_destination = excerpt(Kind::LinkTag_UrlDestination, url_destination);
        excerpts.link_text = excerpt(Kind::LinkTag_LinkText, link_text);
        Ok(DocLinkTag {
            base: self.inline_tag_base(parts),
            code_destination,
            url_destination: url_destination.map(|url| self.text(url)),
            link_text: link_text.map(|text| self.text(text)),
            excerpts,
        }
        .into())
    }

    /// A `{@link}`'s URL and the spacing after it, or none when it isn't a valid URL, which it
    /// reports.
    fn parse_link_tag_url_destination(
        &mut self,
        embedded: &mut TokenReader,
    ) -> Result<Option<(TokenSequence, Option<TokenSequence>)>> {
        // It starts with "//" or a scheme, so it's never empty.
        let mut url = String::new();
        while !matches!(
            embedded.peek_token_kind(),
            TokenKind::Spacing
                | TokenKind::Newline
                | TokenKind::EndOfInput
                | TokenKind::Pipe
                | TokenKind::RightCurlyBracket
        ) {
            url += embedded.read_token()?.text(self.buffer);
        }
        let sequence = embedded.extract_accumulated_sequence();
        if let Some(explanation) = string_checks::explain_if_invalid_link_url(&url) {
            self.log_sequence(Id::LinkTagInvalidUrl, explanation.to_string(), sequence);
            return Ok(None);
        }
        Ok(Some((sequence, self.try_read_spacing_and_newlines(embedded)?)))
    }

    fn parse_declaration_reference(
        &mut self,
        reader: &mut TokenReader,
        context: TokenSequence,
    ) -> Result<Option<DocDeclarationReference>> {
        reader.assert_accumulated_sequence_is_empty();
        // Whether it has a "#", after a package name or import path.
        let marker = reader.create_marker();
        let (mut has_hash, mut looking_for_import_characters, mut saw_import_characters) = (false, true, false);
        loop {
            match reader.peek_token_kind() {
                TokenKind::DoubleQuote
                | TokenKind::EndOfInput
                | TokenKind::LeftCurlyBracket
                | TokenKind::LeftParenthesis
                | TokenKind::LeftSquareBracket
                | TokenKind::Newline
                | TokenKind::Pipe
                | TokenKind::RightCurlyBracket
                | TokenKind::RightParenthesis
                | TokenKind::RightSquareBracket
                | TokenKind::SingleQuote
                | TokenKind::Spacing => break,
                TokenKind::PoundSymbol => {
                    has_hash = true;
                    break;
                }
                TokenKind::Slash | TokenKind::AtSign => {
                    saw_import_characters |= looking_for_import_characters;
                    reader.read_token()?;
                }
                TokenKind::AsciiWord | TokenKind::Period | TokenKind::Hyphen => {
                    reader.read_token()?;
                }
                _ => {
                    looking_for_import_characters = false;
                    reader.read_token()?;
                }
            }
        }
        if !has_hash && saw_import_characters {
            let text = "The declaration reference appears to contain a package name or import path, but it is missing the \"#\" delimiter";
            let sequence = reader.extract_accumulated_sequence();
            self.log_sequence(Id::ReferenceMissingHash, text.into(), sequence);
            return Ok(None);
        }
        reader.backtrack_to_marker(marker);
        let mut excerpts = crate::nodes::DeclarationReferenceExcerpts::default();
        let (mut package_name, mut import_path) = (None, None);
        // The "#" found above is ahead, so the loops below stop at it at the latest.
        if has_hash {
            if reader.peek_token_kind() != TokenKind::Period {
                let scoped_package_name = reader.peek_token_kind() == TokenKind::AtSign;
                let mut finished_scope = false;
                loop {
                    match reader.peek_token_kind() {
                        TokenKind::Slash if scoped_package_name && !finished_scope => {
                            reader.read_token()?;
                            finished_scope = true;
                        }
                        TokenKind::Slash | TokenKind::PoundSymbol => break,
                        _ => {
                            reader.read_token()?;
                        }
                    }
                }
                if let Some(sequence) = reader.try_extract_accumulated_sequence() {
                    let name = self.text(sequence);
                    if let Some(explanation) = string_checks::explain_if_invalid_package_name(&name) {
                        self.log_sequence(Id::ReferenceMalformedPackageName, explanation.to_string(), sequence);
                        return Ok(None);
                    }
                    package_name = Some(name);
                    excerpts.package_name = excerpt(Kind::DeclarationReference_PackageName, Some(sequence));
                }
            }
            while reader.peek_token_kind() != TokenKind::PoundSymbol {
                reader.read_token()?;
            }
            if let Some(sequence) = reader.try_extract_accumulated_sequence() {
                let path = self.text(sequence);
                if let Some(explanation) = string_checks::explain_if_invalid_import_path(&path, package_name.is_some())
                {
                    self.log_sequence(Id::ReferenceMalformedImportPath, explanation.to_string(), sequence);
                    return Ok(None);
                }
                import_path = Some(path);
                excerpts.import_path = excerpt(Kind::DeclarationReference_ImportPath, Some(sequence));
            }
            reader.read_token()?;
            let import_hash = reader.extract_accumulated_sequence();
            excerpts.import_hash = excerpt(Kind::DeclarationReference_ImportHash, Some(import_hash));
            excerpts.spacing_after_import_hash = excerpt(Kind::Spacing, self.try_read_spacing_and_newlines(reader)?);
            if package_name.is_none() && import_path.is_none() {
                let text = "The hash character must be preceded by a package name or import path";
                self.log_sequence(Id::ReferenceHashSyntax, text.into(), import_hash);
                return Ok(None);
            }
        }
        let mut member_references = Vec::new();
        while matches!(
            reader.peek_token_kind(),
            TokenKind::Period
                | TokenKind::LeftParenthesis
                | TokenKind::AsciiWord
                | TokenKind::Colon
                | TokenKind::LeftSquareBracket
                | TokenKind::DoubleQuote
        ) {
            let expecting_dot = !member_references.is_empty();
            match self.parse_member_reference(reader, expecting_dot, context)? {
                Some(member) => member_references.push(member),
                None => return Ok(None),
            }
        }
        if package_name.is_none() && import_path.is_none() && member_references.is_empty() {
            self.log_sequence(Id::MissingReference, "Expecting a declaration reference".into(), context);
            return Ok(None);
        }
        Ok(Some(DocDeclarationReference { package_name, import_path, member_references, excerpts }))
    }

    fn parse_member_reference(
        &mut self,
        reader: &mut TokenReader,
        expecting_dot: bool,
        context: TokenSequence,
    ) -> Result<Option<DocMemberReference>> {
        let mut member = DocMemberReference::default();
        let mut e = MemberReferenceExcerpts::default();
        if expecting_dot {
            if reader.peek_token_kind() != TokenKind::Period {
                let text = "Expecting a period before the next component of a declaration reference";
                self.log_sequence(Id::ReferenceMissingDot, text.into(), context);
                return Ok(None);
            }
            reader.read_token()?;
            e.dot = excerpt(Kind::MemberReference_Dot, Some(reader.extract_accumulated_sequence()));
            e.spacing_after_dot = excerpt(Kind::Spacing, self.try_read_spacing_and_newlines(reader)?);
        }
        member.has_dot = e.dot.is_some();
        let mut left_parenthesis = None;
        if reader.peek_token_kind() == TokenKind::LeftParenthesis {
            reader.read_token()?;
            left_parenthesis = Some(reader.extract_accumulated_sequence());
            e.left_parenthesis = excerpt(Kind::MemberReference_LeftParenthesis, left_parenthesis);
            e.spacing_after_left_parenthesis = excerpt(Kind::Spacing, self.try_read_spacing_and_newlines(reader)?);
        }
        if reader.peek_token_kind() == TokenKind::LeftSquareBracket {
            member.member_symbol = self.parse_member_symbol(reader)?;
            if member.member_symbol.is_none() {
                return Ok(None);
            }
        } else {
            member.member_identifier = self.parse_member_identifier(reader, context)?;
            if member.member_identifier.is_none() {
                return Ok(None);
            }
        }
        e.spacing_after_member = excerpt(Kind::Spacing, self.try_read_spacing_and_newlines(reader)?);
        if reader.peek_token_kind() == TokenKind::Colon {
            reader.read_token()?;
            let colon = reader.extract_accumulated_sequence();
            e.colon = excerpt(Kind::MemberReference_Colon, Some(colon));
            e.spacing_after_colon = excerpt(Kind::Spacing, self.try_read_spacing_and_newlines(reader)?);
            if left_parenthesis.is_none() {
                let text = "Syntax error in declaration reference: the member selector must be enclosed in parentheses";
                self.log_sequence(Id::ReferenceSelectorMissingParens, text.into(), colon);
                return Ok(None);
            }
            member.selector = self.parse_member_selector(reader, colon)?;
            if member.selector.is_none() {
                return Ok(None);
            }
            e.spacing_after_selector = excerpt(Kind::Spacing, self.try_read_spacing_and_newlines(reader)?);
        } else if let Some(left_parenthesis) = left_parenthesis {
            let text = "Expecting a colon after the identifier because the expression is in parentheses";
            self.log_sequence(Id::ReferenceMissingColon, text.into(), left_parenthesis);
            return Ok(None);
        }
        if let Some(left_parenthesis) = left_parenthesis {
            if reader.peek_token_kind() != TokenKind::RightParenthesis {
                let text = "Expecting a matching right parenthesis";
                self.log_sequence(Id::ReferenceMissingRightParen, text.into(), left_parenthesis);
                return Ok(None);
            }
            reader.read_token()?;
            e.right_parenthesis =
                excerpt(Kind::MemberReference_RightParenthesis, Some(reader.extract_accumulated_sequence()));
            e.spacing_after_right_parenthesis = excerpt(Kind::Spacing, self.try_read_spacing_and_newlines(reader)?);
        }
        member.excerpts = e;
        Ok(Some(member))
    }

    fn parse_member_symbol(&mut self, reader: &mut TokenReader) -> Result<Option<DocMemberSymbol>> {
        reader.read_token()?;
        let left_bracket = reader.extract_accumulated_sequence();
        let spacing_after_left_bracket = self.try_read_spacing_and_newlines(reader)?;
        let Some(symbol_reference) = self.parse_declaration_reference(reader, left_bracket)? else {
            let text = "Missing declaration reference in symbol reference";
            self.log_sequence(Id::ReferenceSymbolSyntax, text.into(), left_bracket);
            return Ok(None);
        };
        if reader.peek_token_kind() != TokenKind::RightSquareBracket {
            let text = "Missing closing square bracket for symbol reference";
            self.log_sequence(Id::ReferenceMissingRightBracket, text.into(), left_bracket);
            return Ok(None);
        }
        reader.read_token()?;
        let right_bracket = reader.extract_accumulated_sequence();
        Ok(Some(DocMemberSymbol {
            symbol_reference,
            excerpts: [
                excerpt(Kind::DocMemberSymbol_LeftBracket, Some(left_bracket)),
                excerpt(Kind::Spacing, spacing_after_left_bracket),
                excerpt(Kind::DocMemberSymbol_RightBracket, Some(right_bracket)),
            ],
        }))
    }

    fn parse_member_identifier(
        &mut self,
        reader: &mut TokenReader,
        context: TokenSequence,
    ) -> Result<Option<DocMemberIdentifier>> {
        if reader.peek_token_kind() == TokenKind::DoubleQuote {
            reader.read_token()?;
            let left_quote = reader.extract_accumulated_sequence();
            while reader.peek_token_kind() != TokenKind::DoubleQuote {
                if reader.peek_token_kind() == TokenKind::EndOfInput {
                    let text = "Unexpected end of input inside quoted member identifier";
                    self.log_sequence(Id::ReferenceMissingQuote, text.into(), left_quote);
                    return Ok(None);
                }
                reader.read_token()?;
            }
            let Some(identifier) = reader.try_extract_accumulated_sequence() else {
                let text = "The quoted identifier cannot be empty";
                self.log_sequence(Id::ReferenceEmptyIdentifier, text.into(), left_quote);
                return Ok(None);
            };
            reader.read_token()?;
            let right_quote = reader.extract_accumulated_sequence();
            return Ok(Some(DocMemberIdentifier {
                identifier: self.text(identifier),
                excerpts: [
                    excerpt(Kind::MemberIdentifier_LeftQuote, Some(left_quote)),
                    excerpt(Kind::MemberIdentifier_Identifier, Some(identifier)),
                    excerpt(Kind::MemberIdentifier_RightQuote, Some(right_quote)),
                ],
            }));
        }
        while matches!(reader.peek_token_kind(), TokenKind::AsciiWord | TokenKind::DollarSign) {
            reader.read_token()?;
        }
        let Some(sequence) = reader.try_extract_accumulated_sequence() else {
            let text = "Syntax error in declaration reference: expecting a member identifier";
            self.log_sequence(Id::ReferenceMissingIdentifier, text.into(), context);
            return Ok(None);
        };
        let identifier = self.text(sequence);
        if let Some(explanation) = string_checks::explain_if_invalid_unquoted_member_identifier(&identifier) {
            self.log_sequence(Id::ReferenceUnquotedIdentifier, explanation.to_string(), sequence);
            return Ok(None);
        }
        let excerpts = [None, excerpt(Kind::MemberIdentifier_Identifier, Some(sequence)), None];
        Ok(Some(DocMemberIdentifier { identifier, excerpts }))
    }

    fn parse_member_selector(
        &mut self,
        reader: &mut TokenReader,
        context: TokenSequence,
    ) -> Result<Option<DocMemberSelector>> {
        // TSDoc reports a missing label and reads on, so `(Foo:}` reads past the end.
        if reader.peek_token_kind() != TokenKind::AsciiWord {
            let text = "Expecting a selector label after the colon";
            self.log_sequence(Id::ReferenceMissingLabel, text.into(), context);
        }
        reader.read_token()?;
        let sequence = reader.extract_accumulated_sequence();
        let selector = DocMemberSelector::parsed(self.text(sequence), Some(sequence));
        if let Some(message) = &selector.error_message {
            self.log_sequence(Id::ReferenceSelectorSyntax, message.clone(), sequence);
            return Ok(None);
        }
        Ok(Some(selector))
    }

    fn parse_html_start_tag(&mut self, reader: &mut TokenReader) -> Result<DocNode> {
        reader.assert_accumulated_sequence_is_empty();
        let marker = reader.create_marker();
        reader.read_token()?;
        let opening_delimiter = reader.extract_accumulated_sequence();
        let name = match self.parse_html_name(reader)? {
            Ok(name) => name,
            Err(failure) => {
                return self.backtrack_and_create_error_for_failure(reader, marker, "Invalid HTML element: ", failure);
            }
        };
        let spacing_after_name = self.try_read_spacing_and_newlines(reader)?;
        let mut html_attributes = Vec::new();
        while reader.peek_token_kind() == TokenKind::AsciiWord {
            match self.parse_html_attribute(reader)? {
                Ok(attribute) => html_attributes.push(attribute),
                Err(failure) => {
                    let prefix = "The HTML element has an invalid attribute: ";
                    return self.backtrack_and_create_error_for_failure(reader, marker, prefix, failure);
                }
            }
        }
        reader.assert_accumulated_sequence_is_empty();
        let end_delimiter_marker = reader.create_marker();
        let mut self_closing_tag = false;
        if reader.peek_token_kind() == TokenKind::Slash {
            reader.read_token()?;
            self_closing_tag = true;
        }
        if reader.peek_token_kind() != TokenKind::GreaterThan {
            let text = "Expecting an attribute or \">\" or \"/>\"";
            let failure =
                self.failure_for_tokens_since(reader, Id::HtmlTagMissingGreaterThan, text.into(), end_delimiter_marker);
            let prefix = "The HTML tag has invalid syntax: ";
            return self.backtrack_and_create_error_for_failure(reader, marker, prefix, failure);
        }
        reader.read_token()?;
        let closing_delimiter = reader.extract_accumulated_sequence();
        Ok(DocHtmlStartTag {
            name: self.text(name),
            spacing_after_name: spacing_after_name.map(|spacing| self.text(spacing)),
            html_attributes,
            self_closing_tag,
            excerpts: HtmlTagExcerpts {
                opening_delimiter: excerpt(Kind::HtmlStartTag_OpeningDelimiter, Some(opening_delimiter)),
                name: excerpt(Kind::HtmlStartTag_Name, Some(name)),
                spacing_after_name: excerpt(Kind::Spacing, spacing_after_name),
                closing_delimiter: excerpt(Kind::HtmlStartTag_ClosingDelimiter, Some(closing_delimiter)),
            },
        }
        .into())
    }

    fn parse_html_attribute(
        &mut self,
        reader: &mut TokenReader,
    ) -> Result<std::result::Result<DocHtmlAttribute, Failure>> {
        reader.assert_accumulated_sequence_is_empty();
        let name = match self.parse_html_name(reader)? {
            Ok(name) => name,
            Err(failure) => return Ok(Err(failure)),
        };
        let spacing_after_name = self.try_read_spacing_and_newlines(reader)?;
        if reader.peek_token_kind() != TokenKind::Equals {
            let text = "Expecting \"=\" after HTML attribute name";
            return Ok(Err(self.failure_for_token(reader, Id::HtmlTagMissingEquals, text.into(), None)));
        }
        reader.read_token()?;
        let equals = reader.extract_accumulated_sequence();
        let spacing_after_equals = self.try_read_spacing_and_newlines(reader)?;
        if let Err(failure) = self.parse_html_string(reader)? {
            return Ok(Err(failure));
        }
        let value = reader.extract_accumulated_sequence();
        let spacing_after_value = self.try_read_spacing_and_newlines(reader)?;
        Ok(Ok(DocHtmlAttribute {
            name: self.text(name),
            spacing_after_name: spacing_after_name.map(|spacing| self.text(spacing)),
            spacing_after_equals: spacing_after_equals.map(|spacing| self.text(spacing)),
            value: self.text(value),
            spacing_after_value: spacing_after_value.map(|spacing| self.text(spacing)),
            excerpts: HtmlAttributeExcerpts {
                name: excerpt(Kind::HtmlAttribute_Name, Some(name)),
                spacing_after_name: excerpt(Kind::Spacing, spacing_after_name),
                equals: excerpt(Kind::HtmlAttribute_Equals, Some(equals)),
                spacing_after_equals: excerpt(Kind::Spacing, spacing_after_equals),
                value: excerpt(Kind::HtmlAttribute_Value, Some(value)),
                spacing_after_value: excerpt(Kind::Spacing, spacing_after_value),
            },
        }))
    }

    /// A quoted HTML attribute value, read up to and with its closing quote.
    fn parse_html_string(&mut self, reader: &mut TokenReader) -> Result<std::result::Result<(), Failure>> {
        let marker = reader.create_marker();
        let quote = reader.peek_token_kind();
        if quote != TokenKind::DoubleQuote && quote != TokenKind::SingleQuote {
            let text = "Expecting an HTML string starting with a single-quote or double-quote character";
            return Ok(Err(self.failure_for_token(reader, Id::HtmlTagMissingString, text.into(), None)));
        }
        reader.read_token()?;
        loop {
            match reader.peek_token_kind() {
                kind if kind == quote => {
                    reader.read_token()?;
                    break;
                }
                TokenKind::EndOfInput | TokenKind::Newline => {
                    let text = "The HTML string is missing its closing quote";
                    return Ok(Err(self.failure_for_token(
                        reader,
                        Id::HtmlStringMissingQuote,
                        text.into(),
                        Some(marker),
                    )));
                }
                _ => {
                    reader.read_token()?;
                }
            }
        }
        if reader.peek_token_kind() == TokenKind::AsciiWord {
            let text = "The next character after a closing quote must be spacing or punctuation";
            return Ok(Err(self.failure_for_token(reader, Id::TextAfterHtmlString, text.into(), None)));
        }
        Ok(Ok(()))
    }

    fn parse_html_end_tag(&mut self, reader: &mut TokenReader) -> Result<DocNode> {
        reader.assert_accumulated_sequence_is_empty();
        let marker = reader.create_marker();
        // The caller saw the "</".
        reader.read_token()?;
        reader.read_token()?;
        let opening_delimiter = reader.extract_accumulated_sequence();
        let name = match self.parse_html_name(reader)? {
            Ok(name) => name,
            Err(failure) => {
                let prefix = "Expecting an HTML element name: ";
                return self.backtrack_and_create_error_for_failure(reader, marker, prefix, failure);
            }
        };
        let spacing_after_name = self.try_read_spacing_and_newlines(reader)?;
        if reader.peek_token_kind() != TokenKind::GreaterThan {
            let text = "Expecting a closing \">\" for the HTML tag";
            let failure = self.failure_for_token(reader, Id::HtmlTagMissingGreaterThan, text.into(), None);
            return self.backtrack_and_create_error_for_failure(reader, marker, "", failure);
        }
        reader.read_token()?;
        let closing_delimiter = reader.extract_accumulated_sequence();
        Ok(DocHtmlEndTag {
            name: self.text(name),
            excerpts: HtmlTagExcerpts {
                opening_delimiter: excerpt(Kind::HtmlEndTag_OpeningDelimiter, Some(opening_delimiter)),
                name: excerpt(Kind::HtmlEndTag_Name, Some(name)),
                spacing_after_name: excerpt(Kind::Spacing, spacing_after_name),
                closing_delimiter: excerpt(Kind::HtmlEndTag_ClosingDelimiter, Some(closing_delimiter)),
            },
        }
        .into())
    }

    /// An HTML element or attribute name.
    fn parse_html_name(&mut self, reader: &mut TokenReader) -> Result<std::result::Result<TokenSequence, Failure>> {
        let marker = reader.create_marker();
        if reader.peek_token_kind() == TokenKind::Spacing {
            let text = "A space is not allowed here";
            return Ok(Err(self.failure_for_tokens_since(reader, Id::MalformedHtmlName, text.into(), marker)));
        }
        while matches!(reader.peek_token_kind(), TokenKind::Hyphen | TokenKind::Period | TokenKind::AsciiWord) {
            reader.read_token()?;
        }
        let Some(sequence) = reader.try_extract_accumulated_sequence() else {
            let text = "Expecting an HTML name";
            return Ok(Err(self.failure_for_token(reader, Id::MalformedHtmlName, text.into(), None)));
        };
        let name = self.text(sequence);
        if let Some(explanation) = string_checks::explain_if_invalid_html_name(&name) {
            let text = explanation.to_string();
            return Ok(Err(self.failure_for_tokens_since(reader, Id::MalformedHtmlName, text, marker)));
        }
        if self.configuration.validation.report_unsupported_html_elements
            && !self.configuration.is_html_element_supported(&name)
        {
            let text =
                format!("The HTML element name {} is not defined by your TSDoc configuration", json_string(&name));
            return Ok(Err(self.failure_for_token(reader, Id::UnsupportedHtmlElementName, text, Some(marker))));
        }
        Ok(Ok(sequence))
    }

    fn parse_fenced_code(&mut self, reader: &mut TokenReader) -> Result<DocNode> {
        reader.assert_accumulated_sequence_is_empty();
        let start_marker = reader.create_marker();
        let end_of_opening_delimiter_marker = start_marker + 2;
        if !matches!(reader.peek_previous_token_kind(), TokenKind::Newline | TokenKind::EndOfInput) {
            let text = "The opening backtick for a code fence must appear at the start of the line";
            let (id, end) = (Id::CodeFenceOpeningIndent, end_of_opening_delimiter_marker);
            return self.backtrack_and_create_error_range(reader, start_marker, end, id, text);
        }
        // The caller saw the three backticks.
        for _ in 0..3 {
            reader.read_token()?;
        }
        let opening_fence = reader.extract_accumulated_sequence();
        while reader.peek_token_kind() == TokenKind::Spacing {
            reader.read_token()?;
        }
        let spacing_after_opening_fence = reader.try_extract_accumulated_sequence();
        // The language, up to the spacing and newline that end its line.
        let mut start_of_padding_marker = None;
        let start_of_padding = loop {
            match reader.peek_token_kind() {
                kind @ (TokenKind::Spacing | TokenKind::Newline) => {
                    let start = *start_of_padding_marker.get_or_insert(reader.create_marker());
                    reader.read_token()?;
                    if kind == TokenKind::Newline {
                        break start;
                    }
                }
                TokenKind::Backtick => {
                    let text = "The language specifier cannot contain backtick characters";
                    let failure = self.failure_for_token(reader, Id::CodeFenceSpecifierSyntax, text.into(), None);
                    let (prefix, end) = ("Error parsing code fence: ", end_of_opening_delimiter_marker);
                    return self.backtrack_and_create_error_range_for_failure(
                        reader,
                        start_marker,
                        end,
                        prefix,
                        failure,
                    );
                }
                TokenKind::EndOfInput => {
                    let failure = self.failure_for_token(
                        reader,
                        Id::CodeFenceMissingDelimiter,
                        "Missing closing delimiter".into(),
                        None,
                    );
                    let (prefix, end) = ("Error parsing code fence: ", end_of_opening_delimiter_marker);
                    return self.backtrack_and_create_error_range_for_failure(
                        reader,
                        start_marker,
                        end,
                        prefix,
                        failure,
                    );
                }
                _ => {
                    start_of_padding_marker = None;
                    reader.read_token()?;
                }
            }
        };
        let rest_of_line = reader.extract_accumulated_sequence();
        let language = TokenSequence::new(rest_of_line.start_index, start_of_padding);
        let spacing_after_language = TokenSequence::new(start_of_padding, rest_of_line.end_index);
        // The code, up to a line that starts with three backticks after any spacing.
        let (code_end_marker, closing_fence_start_marker, token_before_delimiter) = loop {
            match reader.peek_token_kind() {
                TokenKind::EndOfInput => {
                    let failure = self.failure_for_token(
                        reader,
                        Id::CodeFenceMissingDelimiter,
                        "Missing closing delimiter".into(),
                        None,
                    );
                    let (prefix, end) = ("Error parsing code fence: ", end_of_opening_delimiter_marker);
                    return self.backtrack_and_create_error_range_for_failure(
                        reader,
                        start_marker,
                        end,
                        prefix,
                        failure,
                    );
                }
                TokenKind::Newline => {
                    let mut token_before_delimiter = reader.read_token()?;
                    let code_end_marker = reader.create_marker();
                    while reader.peek_token_kind() == TokenKind::Spacing {
                        token_before_delimiter = reader.read_token()?;
                    }
                    let closing_fence_start_marker = reader.create_marker();
                    let mut backticks = 0;
                    while backticks < 3 && reader.peek_token_kind() == TokenKind::Backtick {
                        reader.read_token()?;
                        backticks += 1;
                    }
                    if backticks == 3 {
                        break (code_end_marker, closing_fence_start_marker, token_before_delimiter);
                    }
                }
                _ => {
                    reader.read_token()?;
                }
            }
        };
        if token_before_delimiter.kind != TokenKind::Newline {
            let text = "The closing delimiter for a code fence must not be indented";
            self.log_range(Id::CodeFenceClosingIndent, text.into(), token_before_delimiter.range);
        }
        let code_and_delimiter = reader.extract_accumulated_sequence();
        let code = TokenSequence::new(code_and_delimiter.start_index, code_end_marker);
        let spacing_before_closing_fence = TokenSequence::new(code_end_marker, closing_fence_start_marker);
        let closing_fence = TokenSequence::new(closing_fence_start_marker, code_and_delimiter.end_index);
        loop {
            match reader.peek_token_kind() {
                TokenKind::Spacing => {
                    reader.read_token()?;
                }
                TokenKind::Newline => {
                    reader.read_token()?;
                    break;
                }
                TokenKind::EndOfInput => break,
                _ => {
                    let text = "Unexpected characters after closing delimiter for code fence";
                    self.log_range(Id::CodeFenceClosingSyntax, text.into(), reader.peek_token().range);
                    break;
                }
            }
        }
        let spacing_after_closing_fence = reader.try_extract_accumulated_sequence();
        Ok(DocFencedCode {
            language: self.text(language),
            code: self.text(code),
            excerpts: FencedCodeExcerpts {
                opening_fence: excerpt(Kind::FencedCode_OpeningFence, Some(opening_fence)),
                spacing_after_opening_fence: excerpt(Kind::Spacing, spacing_after_opening_fence),
                language: excerpt(Kind::FencedCode_Language, Some(language)),
                spacing_after_language: excerpt(Kind::Spacing, Some(spacing_after_language)),
                code: excerpt(Kind::FencedCode_Code, Some(code)),
                spacing_before_closing_fence: excerpt(Kind::Spacing, Some(spacing_before_closing_fence)),
                closing_fence: excerpt(Kind::FencedCode_ClosingFence, Some(closing_fence)),
                spacing_after_closing_fence: excerpt(Kind::Spacing, spacing_after_closing_fence),
            },
        }
        .into())
    }

    fn parse_code_span(&mut self, reader: &mut TokenReader) -> Result<DocNode> {
        reader.assert_accumulated_sequence_is_empty();
        let marker = reader.create_marker();
        reader.read_token()?;
        let opening_delimiter = reader.extract_accumulated_sequence();
        let (code, closing_delimiter) = loop {
            match reader.peek_token_kind() {
                TokenKind::Backtick => {
                    let Some(code) = reader.try_extract_accumulated_sequence() else {
                        let text = "A code span must contain at least one character between the backticks";
                        let (id, end) = (Id::CodeSpanEmpty, marker + 1);
                        return self.backtrack_and_create_error_range(reader, marker, end, id, text);
                    };
                    reader.read_token()?;
                    break (code, reader.extract_accumulated_sequence());
                }
                TokenKind::EndOfInput | TokenKind::Newline => {
                    let text = "The code span is missing its closing backtick";
                    return self.backtrack_and_create_error(reader, marker, Id::CodeSpanMissingDelimiter, text);
                }
                _ => {
                    reader.read_token()?;
                }
            }
        };
        Ok(DocCodeSpan {
            code: self.text(code),
            excerpts: Some([
                DocExcerpt::new(Kind::CodeSpan_OpeningDelimiter, opening_delimiter),
                DocExcerpt::new(Kind::CodeSpan_Code, code),
                DocExcerpt::new(Kind::CodeSpan_ClosingDelimiter, closing_delimiter),
            ]),
        }
        .into())
    }

    /// Reads spaces and line breaks, if there are any.
    fn try_read_spacing_and_newlines(&mut self, reader: &mut TokenReader) -> Result<Option<TokenSequence>> {
        while matches!(reader.peek_token_kind(), TokenKind::Spacing | TokenKind::Newline) {
            reader.read_token()?;
        }
        Ok(reader.try_extract_accumulated_sequence())
    }

    /// An error text for `sequence`, reported.
    fn error_text(&mut self, sequence: TokenSequence, id: Id, message: String, location: TokenSequence) -> DocNode {
        let error = DocErrorText {
            text: self.text(sequence),
            text_excerpt: DocExcerpt::new(Kind::ErrorText, sequence),
            message_id: id,
            error_message: message,
            error_location: location,
        };
        self.log_sequence(id, error.error_message.clone(), sequence);
        error.into()
    }

    /// Reads the next token as an error text.
    fn create_error(&mut self, reader: &mut TokenReader, id: Id, message: String) -> Result<DocNode> {
        reader.read_token()?;
        let sequence = reader.extract_accumulated_sequence();
        Ok(self.error_text(sequence, id, message, sequence))
    }

    /// Goes back to `marker` and reads the next token as an error text.
    fn backtrack_and_create_error(
        &mut self,
        reader: &mut TokenReader,
        marker: usize,
        id: Id,
        message: &str,
    ) -> Result<DocNode> {
        reader.backtrack_to_marker(marker);
        self.create_error(reader, id, message.into())
    }

    /// Goes back to `start` and reads up to and with `inclusive_end`, and the token after it unless
    /// that's the end of input.
    fn read_error_range(
        &mut self,
        reader: &mut TokenReader,
        start: usize,
        inclusive_end: usize,
    ) -> Result<TokenSequence> {
        reader.backtrack_to_marker(start);
        while reader.create_marker() != inclusive_end {
            reader.read_token()?;
        }
        if reader.peek_token_kind() != TokenKind::EndOfInput {
            reader.read_token()?;
        }
        Ok(reader.extract_accumulated_sequence())
    }

    fn backtrack_and_create_error_range(
        &mut self,
        reader: &mut TokenReader,
        start: usize,
        inclusive_end: usize,
        id: Id,
        message: &str,
    ) -> Result<DocNode> {
        let sequence = self.read_error_range(reader, start, inclusive_end)?;
        Ok(self.error_text(sequence, id, message.into(), sequence))
    }

    /// Goes back to `marker` and reads the next token as an error text, reporting `failure`.
    fn backtrack_and_create_error_for_failure(
        &mut self,
        reader: &mut TokenReader,
        marker: usize,
        prefix: &str,
        failure: Failure,
    ) -> Result<DocNode> {
        reader.backtrack_to_marker(marker);
        reader.read_token()?;
        let sequence = reader.extract_accumulated_sequence();
        Ok(self.error_text(sequence, failure.id, format!("{prefix}{}", failure.message), failure.location))
    }

    fn backtrack_and_create_error_range_for_failure(
        &mut self,
        reader: &mut TokenReader,
        start: usize,
        inclusive_end: usize,
        prefix: &str,
        failure: Failure,
    ) -> Result<DocNode> {
        let sequence = self.read_error_range(reader, start, inclusive_end)?;
        Ok(self.error_text(sequence, failure.id, format!("{prefix}{}", failure.message), failure.location))
    }

    /// A failure at the token at `marker`, or at the next token when there's no marker. As in
    /// TSDoc, a marker of 0 counts as none.
    fn failure_for_token(&self, reader: &TokenReader, id: Id, message: String, marker: Option<usize>) -> Failure {
        let at = marker.filter(|&marker| marker != 0).unwrap_or_else(|| reader.create_marker());
        Failure { id, message, location: TokenSequence::new(at, at + 1) }
    }

    /// A failure over the tokens read since `start_marker`, or the next token when there are none.
    fn failure_for_tokens_since(&self, reader: &TokenReader, id: Id, message: String, start_marker: usize) -> Failure {
        let end = reader.create_marker().max(start_marker + 1);
        Failure { id, message, location: TokenSequence::new(start_marker, end) }
    }
}

/// What a tag name that isn't valid is told.
const TAG_NAME: &str = "A TSDoc tag name must start with a letter and contain only letters and numbers";

/// The first UTF-16 code unit of `text` as JSON, as TSDoc's `JSON.stringify(text[0])` writes it,
/// or `undefined` when it's empty.
fn first_code_unit_json(text: &str) -> String {
    match text.chars().next() {
        None => "undefined".into(),
        Some(c) if c.len_utf16() == 2 => {
            let mut units = [0; 2];
            c.encode_utf16(&mut units);
            format!("\"\\u{:04x}\"", units[0])
        }
        Some(c) => json_string(c.encode_utf8(&mut [0; 4])),
    }
}
