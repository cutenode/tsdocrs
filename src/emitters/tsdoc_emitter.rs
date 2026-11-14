//! `@microsoft/tsdoc`'s TSDocEmitter: a tree written back out as TSDoc, normalized: one blank line
//! between paragraphs and blocks, and spacing collapsed.

use crate::{
    nodes::{DocComment, DocDeclarationReference, DocHtmlEndTag, DocHtmlStartTag, DocNode, InlineTagBase, NodeRef},
    transforms::trim_spaces_in_paragraph,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum LineState {
    Closed,
    StartOfLine,
    MiddleOfLine,
}

/// Writes trees as TSDoc.
struct TSDocEmitter {
    output: String,
    emit_comment_framing: bool,
    line_state: LineState,
    previous_line_had_content: bool,
    hanging_paragraph: bool,
}

const EOL: &str = "\n";

impl DocComment {
    /// It as a `/** */` comment.
    pub fn emit_as_tsdoc(&self) -> String {
        TSDocEmitter::render(NodeRef::Comment(self), true)
    }
}

impl DocDeclarationReference {
    /// It as TSDoc writes a declaration reference.
    pub fn emit_as_tsdoc(&self) -> String {
        TSDocEmitter::render(NodeRef::DeclarationReference(self), false)
    }
}

impl DocHtmlStartTag {
    /// It as HTML.
    pub fn emit_as_html(&self) -> String {
        TSDocEmitter::render(NodeRef::HtmlStartTag(self), false)
    }
}

impl DocHtmlEndTag {
    /// It as HTML.
    pub fn emit_as_html(&self) -> String {
        TSDocEmitter::render(NodeRef::HtmlEndTag(self), false)
    }
}

impl TSDocEmitter {
    fn render(node: NodeRef, emit_comment_framing: bool) -> String {
        let mut emitter = Self {
            output: String::new(),
            emit_comment_framing,
            line_state: LineState::Closed,
            previous_line_had_content: false,
            hanging_paragraph: false,
        };
        emitter.render_node(node);
        emitter.write_end();
        emitter.output
    }

    fn render_node(&mut self, node: NodeRef) {
        match node {
            NodeRef::Block(block) => {
                self.ensure_line_skipped();
                self.render_node(NodeRef::BlockTag(&block.block_tag));
                if matches!(block.block_tag.tag_name_with_upper_case.as_str(), "@RETURNS" | "@DEFAULTVALUE") {
                    self.write_content(" ");
                    self.hanging_paragraph = true;
                }
                self.render_node(NodeRef::Section(&block.content));
            }
            NodeRef::BlockTag(tag) => {
                if self.line_state == LineState::MiddleOfLine {
                    self.write_content(" ");
                }
                self.write_content(&tag.tag_name);
            }
            NodeRef::CodeSpan(code) => {
                self.write_content("`");
                self.write_content(&code.code);
                self.write_content("`");
            }
            NodeRef::Comment(comment) => {
                let sections = [Some(NodeRef::Section(&comment.summary_section))];
                let blocks = [&comment.remarks_block, &comment.private_remarks, &comment.deprecated_block];
                let nodes = sections
                    .into_iter()
                    .chain(blocks.map(|block| block.as_ref().map(NodeRef::Block)))
                    .chain([Some(NodeRef::ParamCollection(&comment.params))])
                    .chain([Some(NodeRef::ParamCollection(&comment.type_params))])
                    .chain([comment.returns_block.as_ref().map(NodeRef::Block)])
                    .flatten()
                    .chain(comment.custom_blocks.iter().map(NodeRef::Block))
                    .chain(comment.see_blocks.iter().map(NodeRef::Block))
                    .chain(comment.inherit_doc_tag.as_ref().map(NodeRef::InheritDocTag));
                nodes.for_each(|node| self.render_node(node));
                if !comment.modifier_tag_set.nodes.is_empty() {
                    self.ensure_line_skipped();
                    comment.modifier_tag_set.nodes.iter().for_each(|tag| self.render_node(NodeRef::BlockTag(tag)));
                }
            }
            NodeRef::DeclarationReference(reference) => {
                self.write_optional(reference.package_name.as_deref());
                self.write_optional(reference.import_path.as_deref());
                if reference.package_name.is_some() || reference.import_path.is_some() {
                    self.write_content("#");
                }
                reference
                    .member_references
                    .iter()
                    .for_each(|member| self.render_node(NodeRef::MemberReference(member)));
            }
            NodeRef::ErrorText(error) => self.write_content(&error.text),
            NodeRef::EscapedText(escaped) => self.write_content(&escaped.encoded_text),
            NodeRef::FencedCode(code) => {
                self.ensure_at_start_of_line();
                self.write_content("```");
                self.write_content(&code.language);
                self.write_newline();
                self.write_content(&code.code);
                self.write_content("```");
                self.write_newline();
                self.write_newline();
            }
            NodeRef::HtmlAttribute(attribute) => {
                self.write_content(&attribute.name);
                self.write_optional(attribute.spacing_after_name.as_deref());
                self.write_content("=");
                self.write_optional(attribute.spacing_after_equals.as_deref());
                self.write_content(&attribute.value);
                self.write_optional(attribute.spacing_after_value.as_deref());
            }
            NodeRef::HtmlEndTag(tag) => {
                self.write_content("</");
                self.write_content(&tag.name);
                self.write_content(">");
            }
            NodeRef::HtmlStartTag(tag) => {
                self.write_content("<");
                self.write_content(&tag.name);
                self.write_optional(tag.spacing_after_name.as_deref());
                let mut needs_space = tag.spacing_after_name.as_deref().is_none_or(str::is_empty);
                for attribute in &tag.html_attributes {
                    if needs_space {
                        self.write_content(" ");
                    }
                    self.render_node(NodeRef::HtmlAttribute(attribute));
                    needs_space = attribute.spacing_after_value.as_deref().is_none_or(str::is_empty);
                }
                self.write_content(if tag.self_closing_tag { "/>" } else { ">" });
            }
            NodeRef::InheritDocTag(tag) => self.render_inline_tag(&tag.base, |emitter| {
                if let Some(reference) = &tag.declaration_reference {
                    emitter.write_content(" ");
                    emitter.render_node(NodeRef::DeclarationReference(reference));
                }
            }),
            NodeRef::InlineTag(tag) => self.render_inline_tag(&tag.base, |emitter| {
                if !tag.tag_content.is_empty() {
                    emitter.write_content(" ");
                    emitter.write_content(&tag.tag_content);
                }
            }),
            NodeRef::LinkTag(link) => self.render_inline_tag(&link.base, |emitter| {
                if let Some(url) = &link.url_destination {
                    emitter.write_content(" ");
                    emitter.write_content(url);
                } else if let Some(reference) = &link.code_destination {
                    emitter.write_content(" ");
                    emitter.render_node(NodeRef::DeclarationReference(reference));
                }
                if let Some(text) = &link.link_text {
                    emitter.write_content(" ");
                    emitter.write_content("|");
                    emitter.write_content(" ");
                    emitter.write_content(text);
                }
            }),
            NodeRef::MemberIdentifier(identifier) => {
                if identifier.has_quotes() {
                    self.write_content("\"");
                    self.write_content(&identifier.identifier);
                    self.write_content("\"");
                } else {
                    self.write_content(&identifier.identifier);
                }
            }
            NodeRef::MemberReference(member) => {
                if member.has_dot {
                    self.write_content(".");
                }
                if member.selector.is_some() {
                    self.write_content("(");
                }
                if let Some(symbol) = &member.member_symbol {
                    self.render_node(NodeRef::MemberSymbol(symbol));
                } else if let Some(identifier) = &member.member_identifier {
                    self.render_node(NodeRef::MemberIdentifier(identifier));
                }
                if let Some(selector) = &member.selector {
                    self.write_content(":");
                    self.render_node(NodeRef::MemberSelector(selector));
                    self.write_content(")");
                }
            }
            NodeRef::MemberSelector(selector) => self.write_content(&selector.selector),
            NodeRef::MemberSymbol(symbol) => {
                self.write_content("[");
                self.render_node(NodeRef::DeclarationReference(&symbol.symbol_reference));
                self.write_content("]");
            }
            NodeRef::Section(section) => section.nodes.iter().for_each(|node| self.render_node(node.as_ref())),
            NodeRef::SoftBreak(_) => self.ensure_at_start_of_line(),
            NodeRef::Paragraph(paragraph) => {
                let trimmed = trim_spaces_in_paragraph(paragraph);
                if !trimmed.nodes.is_empty() {
                    if self.hanging_paragraph {
                        self.hanging_paragraph = false;
                    } else {
                        self.ensure_line_skipped();
                    }
                    trimmed.nodes.iter().for_each(|node: &DocNode| self.render_node(node.as_ref()));
                    self.write_newline();
                }
            }
            NodeRef::ParamBlock(block) => {
                self.ensure_line_skipped();
                self.render_node(NodeRef::BlockTag(&block.block_tag));
                self.write_content(" ");
                self.write_content(&block.parameter_name);
                self.write_content(" - ");
                self.hanging_paragraph = true;
                self.render_node(NodeRef::Section(&block.content));
                self.hanging_paragraph = false;
            }
            NodeRef::ParamCollection(params) => {
                params.blocks.iter().for_each(|block| self.render_node(NodeRef::ParamBlock(block)));
            }
            NodeRef::PlainText(text) => self.write_content(&text.text),
            NodeRef::Excerpt(_) => {}
        }
    }

    fn render_inline_tag(&mut self, base: &InlineTagBase, write_content: impl FnOnce(&mut Self)) {
        self.write_content("{");
        self.write_content(&base.tag_name);
        write_content(self);
        self.write_content("}");
    }

    fn ensure_at_start_of_line(&mut self) {
        if self.line_state == LineState::MiddleOfLine {
            self.write_newline();
        }
    }

    fn ensure_line_skipped(&mut self) {
        self.ensure_at_start_of_line();
        if self.previous_line_had_content {
            self.write_newline();
        }
    }

    fn write_optional(&mut self, content: Option<&str>) {
        if let Some(content) = content {
            self.write_content(content);
        }
    }

    fn write_content(&mut self, content: &str) {
        if content.is_empty() {
            return;
        }
        if content.contains('\n') {
            // Lines end in `\n` or `\r\n`.
            let lines: Vec<&str> = content.split('\n').collect();
            for (i, line) in lines.iter().enumerate() {
                if i > 0 {
                    self.write_newline();
                }
                let ends_line = i + 1 < lines.len();
                self.write_content(line.strip_suffix('\r').filter(|_| ends_line).unwrap_or(line));
            }
            return;
        }
        self.open();
        if self.line_state == LineState::StartOfLine && self.emit_comment_framing {
            self.output.push(' ');
        }
        self.output += content;
        self.line_state = LineState::MiddleOfLine;
        self.previous_line_had_content = true;
    }

    /// Starts the comment, if it hasn't started.
    fn open(&mut self) {
        if self.line_state == LineState::Closed {
            if self.emit_comment_framing {
                self.output += "/**";
                self.output += EOL;
                self.output += " *";
            }
            self.line_state = LineState::StartOfLine;
        }
    }

    fn write_newline(&mut self) {
        self.open();
        self.previous_line_had_content = self.line_state == LineState::MiddleOfLine;
        self.output += EOL;
        if self.emit_comment_framing {
            self.output += " *";
        }
        self.line_state = LineState::StartOfLine;
        self.hanging_paragraph = false;
    }

    fn write_end(&mut self) {
        if self.line_state == LineState::MiddleOfLine && self.emit_comment_framing {
            self.write_newline();
        }
        if self.line_state != LineState::Closed {
            if self.emit_comment_framing {
                self.output += "/";
                self.output += EOL;
            }
            self.line_state = LineState::Closed;
        }
    }
}
