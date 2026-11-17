//! The comment and its parts: sections, paragraphs, blocks and `@param` blocks.

use std::borrow::Cow;

use super::{DocBlockTag, DocExcerpt, DocInheritDocTag, DocNode, DocNodeKind, NodeRef, push};

/// A whole doc comment.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocComment<'a> {
    /// The text before the first block tag.
    pub summary_section: DocSection<'a>,
    /// The last `@remarks` block.
    pub remarks_block: Option<DocBlock<'a>>,
    /// The last `@privateRemarks` block.
    pub private_remarks: Option<DocBlock<'a>>,
    /// The last `@deprecated` block.
    pub deprecated_block: Option<DocBlock<'a>>,
    /// The `@param` blocks.
    pub params: DocParamCollection<'a>,
    /// The `@typeParam` blocks.
    pub type_params: DocParamCollection<'a>,
    /// The last `@returns` block.
    pub returns_block: Option<DocBlock<'a>>,
    /// The `@see` blocks.
    pub see_blocks: Vec<DocBlock<'a>>,
    /// The other blocks, like `@example`.
    pub custom_blocks: Vec<DocBlock<'a>>,
    /// Its `{@inheritDoc}`, the first if it has more.
    pub inherit_doc_tag: Option<DocInheritDocTag<'a>>,
    /// Its modifier tags, like `@public`.
    pub modifier_tag_set: ModifierTagSet<'a>,
}

impl<'a> DocComment<'a> {
    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        children.push(NodeRef::Section(&self.summary_section));
        push(children, &self.remarks_block);
        push(children, &self.private_remarks);
        push(children, &self.deprecated_block);
        if !self.params.blocks.is_empty() {
            children.push(NodeRef::ParamCollection(&self.params));
        }
        if !self.type_params.blocks.is_empty() {
            children.push(NodeRef::ParamCollection(&self.type_params));
        }
        push(children, &self.returns_block);
        children.extend(self.custom_blocks.iter().map(NodeRef::Block));
        children.extend(self.see_blocks.iter().map(NodeRef::Block));
        push(children, &self.inherit_doc_tag);
        children.extend(self.modifier_tag_set.nodes.iter().map(NodeRef::BlockTag));
    }
}

/// A comment's modifier tags, each once.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModifierTagSet<'a> {
    /// The tags, in the order they came.
    pub nodes: Vec<DocBlockTag<'a>>,
}

impl<'a> ModifierTagSet<'a> {
    /// Whether it has the tag named `tag_name`, in any case.
    pub fn has_tag_name(&self, tag_name: &str) -> bool {
        // Every tag the parser reads has an ASCII name, which upper-cases byte by byte without a
        // copy. Other text can upper-case into ASCII, as `ı` does into `I`, so it takes the full
        // mapping.
        if tag_name.is_ascii() {
            let upper = || tag_name.bytes().map(|b| b.to_ascii_uppercase());
            return self.nodes.iter().any(|tag| tag.tag_name_with_upper_case.bytes().eq(upper()));
        }
        let upper = tag_name.to_uppercase();
        self.nodes.iter().any(|tag| tag.tag_name_with_upper_case == upper)
    }

    /// Adds `tag`, unless it has a tag of that name already, and says whether it did.
    pub fn add_tag(&mut self, tag: DocBlockTag<'a>) -> bool {
        if self.has_tag_name(&tag.tag_name) {
            return false;
        }
        self.nodes.push(tag);
        true
    }

    /// Whether it has `@alpha`.
    pub fn is_alpha(&self) -> bool {
        self.has_tag_name("@alpha")
    }

    /// Whether it has `@beta`.
    pub fn is_beta(&self) -> bool {
        self.has_tag_name("@beta")
    }

    /// Whether it has `@eventProperty`.
    pub fn is_event_property(&self) -> bool {
        self.has_tag_name("@eventProperty")
    }

    /// Whether it has `@experimental`.
    pub fn is_experimental(&self) -> bool {
        self.has_tag_name("@experimental")
    }

    /// Whether it has `@internal`.
    pub fn is_internal(&self) -> bool {
        self.has_tag_name("@internal")
    }

    /// Whether it has `@override`.
    pub fn is_override(&self) -> bool {
        self.has_tag_name("@override")
    }

    /// Whether it has `@packageDocumentation`.
    pub fn is_package_documentation(&self) -> bool {
        self.has_tag_name("@packageDocumentation")
    }

    /// Whether it has `@public`.
    pub fn is_public(&self) -> bool {
        self.has_tag_name("@public")
    }

    /// Whether it has `@readonly`.
    pub fn is_readonly(&self) -> bool {
        self.has_tag_name("@readonly")
    }

    /// Whether it has `@sealed`.
    pub fn is_sealed(&self) -> bool {
        self.has_tag_name("@sealed")
    }

    /// Whether it has `@virtual`.
    pub fn is_virtual(&self) -> bool {
        self.has_tag_name("@virtual")
    }
}

/// A section: paragraphs, and the fenced code and HTML tags between them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocSection<'a> {
    /// What it holds.
    pub nodes: Vec<DocNode<'a>>,
}

impl<'a> DocSection<'a> {
    /// Adds `node` to its last paragraph, starting one if its last node isn't one.
    pub fn append_node_in_paragraph(&mut self, node: DocNode<'a>) {
        if let Some(DocNode::Paragraph(paragraph)) = self.nodes.last_mut() {
            paragraph.nodes.push(node);
            return;
        }
        self.nodes.push(DocNode::Paragraph(DocParagraph { nodes: vec![node] }));
    }

    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        children.extend(self.nodes.iter().map(DocNode::as_ref));
    }
}

/// A paragraph.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocParagraph<'a> {
    /// What it holds.
    pub nodes: Vec<DocNode<'a>>,
}

impl<'a> DocParagraph<'a> {
    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        children.extend(self.nodes.iter().map(DocNode::as_ref));
    }
}

/// Whether a paragraph may hold a node of `kind`; the others go in its section.
pub(crate) fn paragraph_allows(kind: DocNodeKind) -> bool {
    matches!(
        kind,
        DocNodeKind::BlockTag
            | DocNodeKind::CodeSpan
            | DocNodeKind::ErrorText
            | DocNodeKind::EscapedText
            | DocNodeKind::HtmlStartTag
            | DocNodeKind::HtmlEndTag
            | DocNodeKind::InlineTag
            | DocNodeKind::LinkTag
            | DocNodeKind::PlainText
            | DocNodeKind::SoftBreak
    )
}

/// A block: a block tag and the section after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocBlock<'a> {
    /// Its tag.
    pub block_tag: DocBlockTag<'a>,
    /// What follows it.
    pub content: DocSection<'a>,
}

impl<'a> DocBlock<'a> {
    /// A block for `block_tag`, empty.
    pub fn new(block_tag: DocBlockTag<'a>) -> Self {
        Self { block_tag, content: DocSection::default() }
    }

    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        children.push(NodeRef::BlockTag(&self.block_tag));
        children.push(NodeRef::Section(&self.content));
    }
}

/// A `@param` or `@typeParam` block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocParamBlock<'a> {
    /// Its tag.
    pub block_tag: DocBlockTag<'a>,
    /// The parameter's name, or empty when it was missing.
    pub parameter_name: Cow<'a, str>,
    /// What follows the name and hyphen.
    pub content: DocSection<'a>,
    /// Its excerpts, when it was parsed.
    pub excerpts: ParamBlockExcerpts,
}

/// A parsed `@param` block's excerpts, in order. The nonstandard ones are JSDoc syntax TSDoc
/// reports, like a `{type}` or a `[name]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct ParamBlockExcerpts {
    pub spacing_before_parameter_name: Option<DocExcerpt>,
    pub unsupported_jsdoc_type_before_parameter_name: Option<DocExcerpt>,
    pub unsupported_jsdoc_optional_name_open_bracket: Option<DocExcerpt>,
    pub parameter_name: Option<DocExcerpt>,
    pub unsupported_jsdoc_optional_name_rest: Option<DocExcerpt>,
    pub spacing_after_parameter_name: Option<DocExcerpt>,
    pub unsupported_jsdoc_type_after_parameter_name: Option<DocExcerpt>,
    pub hyphen: Option<DocExcerpt>,
    pub spacing_after_hyphen: Option<DocExcerpt>,
    pub unsupported_jsdoc_type_after_hyphen: Option<DocExcerpt>,
}

impl<'a> DocParamBlock<'a> {
    /// A block for `block_tag` naming `parameter_name`, empty.
    pub fn new(block_tag: DocBlockTag<'a>, parameter_name: impl Into<Cow<'a, str>>) -> Self {
        let parameter_name = parameter_name.into();
        Self { block_tag, parameter_name, content: DocSection::default(), excerpts: ParamBlockExcerpts::default() }
    }

    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        let e = &self.excerpts;
        children.push(NodeRef::BlockTag(&self.block_tag));
        for excerpt in [
            &e.spacing_before_parameter_name,
            &e.unsupported_jsdoc_type_before_parameter_name,
            &e.unsupported_jsdoc_optional_name_open_bracket,
            &e.parameter_name,
            &e.unsupported_jsdoc_optional_name_rest,
            &e.spacing_after_parameter_name,
            &e.unsupported_jsdoc_type_after_parameter_name,
            &e.hyphen,
            &e.spacing_after_hyphen,
            &e.unsupported_jsdoc_type_after_hyphen,
        ] {
            push(children, excerpt);
        }
        children.push(NodeRef::Section(&self.content));
    }
}

/// A comment's `@param` or `@typeParam` blocks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocParamCollection<'a> {
    /// The blocks, in order.
    pub blocks: Vec<DocParamBlock<'a>>,
}

impl<'a> DocParamCollection<'a> {
    /// The first block for the parameter named `name`.
    pub fn try_get_block_by_name(&self, name: &str) -> Option<&DocParamBlock<'a>> {
        self.blocks.iter().find(|block| block.parameter_name == name)
    }

    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        children.extend(self.blocks.iter().map(NodeRef::ParamBlock));
    }
}
