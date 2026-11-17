//! The comment and its parts: sections, paragraphs, blocks and `@param` blocks.

use super::{DocBlockTag, DocExcerpt, DocInheritDocTag, DocNode, DocNodeKind, NodeRef, push};

/// A whole doc comment.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocComment {
    /// The text before the first block tag.
    pub summary_section: DocSection,
    /// The last `@remarks` block.
    pub remarks_block: Option<DocBlock>,
    /// The last `@privateRemarks` block.
    pub private_remarks: Option<DocBlock>,
    /// The last `@deprecated` block.
    pub deprecated_block: Option<DocBlock>,
    /// The `@param` blocks.
    pub params: DocParamCollection,
    /// The `@typeParam` blocks.
    pub type_params: DocParamCollection,
    /// The last `@returns` block.
    pub returns_block: Option<DocBlock>,
    /// The `@see` blocks.
    pub see_blocks: Vec<DocBlock>,
    /// The other blocks, like `@example`.
    pub custom_blocks: Vec<DocBlock>,
    /// Its `{@inheritDoc}`, the first if it has more.
    pub inherit_doc_tag: Option<DocInheritDocTag>,
    /// Its modifier tags, like `@public`.
    pub modifier_tag_set: ModifierTagSet,
}

impl DocComment {
    pub(super) fn push_children<'a>(&'a self, children: &mut Vec<NodeRef<'a>>) {
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
pub struct ModifierTagSet {
    /// The tags, in the order they came.
    pub nodes: Vec<DocBlockTag>,
}

impl ModifierTagSet {
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
    pub fn add_tag(&mut self, tag: DocBlockTag) -> bool {
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
pub struct DocSection {
    /// What it holds.
    pub nodes: Vec<DocNode>,
}

impl DocSection {
    /// Adds `node` to its last paragraph, starting one if its last node isn't one.
    pub fn append_node_in_paragraph(&mut self, node: DocNode) {
        if let Some(DocNode::Paragraph(paragraph)) = self.nodes.last_mut() {
            paragraph.nodes.push(node);
            return;
        }
        self.nodes.push(DocNode::Paragraph(DocParagraph { nodes: vec![node] }));
    }

    pub(super) fn push_children<'a>(&'a self, children: &mut Vec<NodeRef<'a>>) {
        children.extend(self.nodes.iter().map(DocNode::as_ref));
    }
}

/// A paragraph.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocParagraph {
    /// What it holds.
    pub nodes: Vec<DocNode>,
}

impl DocParagraph {
    pub(super) fn push_children<'a>(&'a self, children: &mut Vec<NodeRef<'a>>) {
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
pub struct DocBlock {
    /// Its tag.
    pub block_tag: DocBlockTag,
    /// What follows it.
    pub content: DocSection,
}

impl DocBlock {
    /// A block for `block_tag`, empty.
    pub fn new(block_tag: DocBlockTag) -> Self {
        Self { block_tag, content: DocSection::default() }
    }

    pub(super) fn push_children<'a>(&'a self, children: &mut Vec<NodeRef<'a>>) {
        children.push(NodeRef::BlockTag(&self.block_tag));
        children.push(NodeRef::Section(&self.content));
    }
}

/// A `@param` or `@typeParam` block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocParamBlock {
    /// Its tag.
    pub block_tag: DocBlockTag,
    /// The parameter's name, or empty when it was missing.
    pub parameter_name: String,
    /// What follows the name and hyphen.
    pub content: DocSection,
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

impl DocParamBlock {
    /// A block for `block_tag` naming `parameter_name`, empty.
    pub fn new(block_tag: DocBlockTag, parameter_name: impl Into<String>) -> Self {
        let parameter_name = parameter_name.into();
        Self { block_tag, parameter_name, content: DocSection::default(), excerpts: ParamBlockExcerpts::default() }
    }

    pub(super) fn push_children<'a>(&'a self, children: &mut Vec<NodeRef<'a>>) {
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
pub struct DocParamCollection {
    /// The blocks, in order.
    pub blocks: Vec<DocParamBlock>,
}

impl DocParamCollection {
    /// The first block for the parameter named `name`.
    pub fn try_get_block_by_name(&self, name: &str) -> Option<&DocParamBlock> {
        self.blocks.iter().find(|block| block.parameter_name == name)
    }

    pub(super) fn push_children<'a>(&'a self, children: &mut Vec<NodeRef<'a>>) {
        children.extend(self.blocks.iter().map(NodeRef::ParamBlock));
    }
}
