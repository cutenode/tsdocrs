//! The doc nodes: the tree a comment parses into, `@microsoft/tsdoc`'s DocNode classes. Each node
//! the parser makes keeps excerpts, the runs of tokens it was read from, as children, so a tree's
//! excerpts cover the comment's tokens in order. A node built by hand has none.
//!
//! [`NodeRef`] views any node, with its [`kind`](NodeRef::kind) and its
//! [`child_nodes`](NodeRef::child_nodes) in TSDoc's order. [`DocNode`] owns the nodes a section or
//! paragraph holds.

mod comment;
mod html;
mod inline;
mod reference;
mod text;

pub(crate) use self::comment::paragraph_allows;
pub use self::{comment::*, html::*, inline::*, reference::*, text::*};
use crate::parser::TokenSequence;

/// The type a [`DocNode`] variant holds its node as: boxed for the kinds marked `boxed`, so a
/// `DocNode` is as small as the largest of the rest, and moving one is cheap.
macro_rules! held {
    ([$($node:tt)*]) => { $($node)* };
    ([$($node:tt)*] boxed) => { Box<$($node)*> };
}

/// `node`, as its [`DocNode`] variant holds it.
macro_rules! hold {
    ($node:ident) => {
        $node
    };
    ($node:ident boxed) => {
        Box::new($node)
    };
}

/// Declares the node kinds: [`DocNodeKind`], and a variant of [`DocNode`] and [`NodeRef`] for each.
/// `$a` is the lifetime of the text the nodes borrow.
macro_rules! nodes {
    ($a:lifetime; $($kind:ident($node:ident $(<$l:lifetime>)? $($boxed:ident)?),)*) => {
        /// What a node is.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum DocNodeKind {
            $(
                #[doc = concat!("A [`", stringify!($node), "`].")]
                $kind,
            )*
        }

        impl DocNodeKind {
            /// Its name, as `@microsoft/tsdoc`'s `kind` has it.
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$kind => stringify!($kind),)*
                }
            }
        }

        /// A node, owned. Sections and paragraphs hold these; every kind has a variant, so any
        /// node can be held as one, the large comment included. Plain text, soft breaks,
        /// paragraphs, sections and excerpts are held as they are and the other kinds boxed, so a
        /// `DocNode` is no larger than plain text.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum DocNode<$a> {
            $(
                #[doc = concat!("A [`", stringify!($node), "`].")]
                $kind(held!([$node $(<$l>)?] $($boxed)?)),
            )*
        }

        /// A node, borrowed.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum NodeRef<$a> {
            $(
                #[doc = concat!("A [`", stringify!($node), "`].")]
                $kind(&$a $node $(<$l>)?),
            )*
        }

        impl<$a> DocNode<$a> {
            /// What it is.
            pub fn kind(&self) -> DocNodeKind {
                self.as_ref().kind()
            }

            /// It, borrowed.
            pub fn as_ref(&self) -> NodeRef<'_> {
                match self {
                    $(Self::$kind(node) => NodeRef::$kind(node),)*
                }
            }
        }

        impl<$a> NodeRef<$a> {
            /// What it is.
            pub fn kind(self) -> DocNodeKind {
                match self {
                    $(Self::$kind(_) => DocNodeKind::$kind,)*
                }
            }

            /// Its children, excerpts included, in `@microsoft/tsdoc`'s order.
            pub fn child_nodes(self) -> Vec<NodeRef<$a>> {
                let mut children = Vec::new();
                match self {
                    $(Self::$kind(node) => node.push_children(&mut children),)*
                }
                children
            }
        }

        $(
            impl<$a> From<&$a $node $(<$l>)?> for NodeRef<$a> {
                fn from(node: &$a $node $(<$l>)?) -> Self {
                    Self::$kind(node)
                }
            }

            impl<$a> From<$node $(<$l>)?> for DocNode<$a> {
                fn from(node: $node $(<$l>)?) -> Self {
                    Self::$kind(hold!(node $($boxed)?))
                }
            }
        )*
    };
}

nodes! {
    'a;
    Block(DocBlock<'a> boxed),
    BlockTag(DocBlockTag<'a> boxed),
    Excerpt(DocExcerpt),
    FencedCode(DocFencedCode<'a> boxed),
    CodeSpan(DocCodeSpan<'a> boxed),
    Comment(DocComment<'a> boxed),
    DeclarationReference(DocDeclarationReference<'a> boxed),
    ErrorText(DocErrorText<'a> boxed),
    EscapedText(DocEscapedText<'a> boxed),
    HtmlAttribute(DocHtmlAttribute<'a> boxed),
    HtmlEndTag(DocHtmlEndTag<'a> boxed),
    HtmlStartTag(DocHtmlStartTag<'a> boxed),
    InheritDocTag(DocInheritDocTag<'a> boxed),
    InlineTag(DocInlineTag<'a> boxed),
    LinkTag(DocLinkTag<'a> boxed),
    MemberIdentifier(DocMemberIdentifier<'a> boxed),
    MemberReference(DocMemberReference<'a> boxed),
    MemberSelector(DocMemberSelector<'a> boxed),
    MemberSymbol(DocMemberSymbol<'a> boxed),
    Paragraph(DocParagraph<'a>),
    ParamBlock(DocParamBlock<'a> boxed),
    ParamCollection(DocParamCollection<'a> boxed),
    PlainText(DocPlainText<'a>),
    Section(DocSection<'a>),
    SoftBreak(DocSoftBreak),
}

/// Adds `node` to `children`, if there is one.
pub(crate) fn push<'a, T>(children: &mut Vec<NodeRef<'a>>, node: &'a Option<T>)
where
    &'a T: Into<NodeRef<'a>>,
{
    if let Some(node) = node {
        children.push(node.into());
    }
}

/// What part of a node an excerpt is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types, missing_docs)]
pub enum ExcerptKind {
    Spacing,
    BlockTag,
    CodeSpan_OpeningDelimiter,
    CodeSpan_Code,
    CodeSpan_ClosingDelimiter,
    DeclarationReference_PackageName,
    DeclarationReference_ImportPath,
    DeclarationReference_ImportHash,
    ErrorText,
    NonstandardText,
    EscapedText,
    FencedCode_OpeningFence,
    FencedCode_Language,
    FencedCode_Code,
    FencedCode_ClosingFence,
    HtmlAttribute_Name,
    HtmlAttribute_Equals,
    HtmlAttribute_Value,
    HtmlEndTag_OpeningDelimiter,
    HtmlEndTag_Name,
    HtmlEndTag_ClosingDelimiter,
    HtmlStartTag_OpeningDelimiter,
    HtmlStartTag_Name,
    HtmlStartTag_ClosingDelimiter,
    InlineTag_OpeningDelimiter,
    InlineTag_TagName,
    InlineTag_TagContent,
    InlineTag_ClosingDelimiter,
    LinkTag_UrlDestination,
    LinkTag_Pipe,
    LinkTag_LinkText,
    MemberIdentifier_LeftQuote,
    MemberIdentifier_Identifier,
    MemberIdentifier_RightQuote,
    MemberReference_Dot,
    MemberReference_LeftParenthesis,
    MemberReference_Colon,
    MemberReference_RightParenthesis,
    MemberSelector,
    DocMemberSymbol_LeftBracket,
    DocMemberSymbol_RightBracket,
    ParamBlock_ParameterName,
    ParamBlock_Hyphen,
    PlainText,
    SoftBreak,
}

impl ExcerptKind {
    /// Its name, as `@microsoft/tsdoc`'s `excerptKind` has it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Spacing => "Spacing",
            Self::BlockTag => "BlockTag",
            Self::CodeSpan_OpeningDelimiter => "CodeSpan_OpeningDelimiter",
            Self::CodeSpan_Code => "CodeSpan_Code",
            Self::CodeSpan_ClosingDelimiter => "CodeSpan_ClosingDelimiter",
            Self::DeclarationReference_PackageName => "DeclarationReference_PackageName",
            Self::DeclarationReference_ImportPath => "DeclarationReference_ImportPath",
            Self::DeclarationReference_ImportHash => "DeclarationReference_ImportHash",
            Self::ErrorText => "ErrorText",
            Self::NonstandardText => "NonstandardText",
            Self::EscapedText => "EscapedText",
            Self::FencedCode_OpeningFence => "FencedCode_OpeningFence",
            Self::FencedCode_Language => "FencedCode_Language",
            Self::FencedCode_Code => "FencedCode_Code",
            Self::FencedCode_ClosingFence => "FencedCode_ClosingFence",
            Self::HtmlAttribute_Name => "HtmlAttribute_Name",
            Self::HtmlAttribute_Equals => "HtmlAttribute_Equals",
            Self::HtmlAttribute_Value => "HtmlAttribute_Value",
            Self::HtmlEndTag_OpeningDelimiter => "HtmlEndTag_OpeningDelimiter",
            Self::HtmlEndTag_Name => "HtmlEndTag_Name",
            Self::HtmlEndTag_ClosingDelimiter => "HtmlEndTag_ClosingDelimiter",
            Self::HtmlStartTag_OpeningDelimiter => "HtmlStartTag_OpeningDelimiter",
            Self::HtmlStartTag_Name => "HtmlStartTag_Name",
            Self::HtmlStartTag_ClosingDelimiter => "HtmlStartTag_ClosingDelimiter",
            Self::InlineTag_OpeningDelimiter => "InlineTag_OpeningDelimiter",
            Self::InlineTag_TagName => "InlineTag_TagName",
            Self::InlineTag_TagContent => "InlineTag_TagContent",
            Self::InlineTag_ClosingDelimiter => "InlineTag_ClosingDelimiter",
            Self::LinkTag_UrlDestination => "LinkTag_UrlDestination",
            Self::LinkTag_Pipe => "LinkTag_Pipe",
            Self::LinkTag_LinkText => "LinkTag_LinkText",
            Self::MemberIdentifier_LeftQuote => "MemberIdentifier_LeftQuote",
            Self::MemberIdentifier_Identifier => "MemberIdentifier_Identifier",
            Self::MemberIdentifier_RightQuote => "MemberIdentifier_RightQuote",
            Self::MemberReference_Dot => "MemberReference_Dot",
            Self::MemberReference_LeftParenthesis => "MemberReference_LeftParenthesis",
            Self::MemberReference_Colon => "MemberReference_Colon",
            Self::MemberReference_RightParenthesis => "MemberReference_RightParenthesis",
            Self::MemberSelector => "MemberSelector",
            Self::DocMemberSymbol_LeftBracket => "DocMemberSymbol_LeftBracket",
            Self::DocMemberSymbol_RightBracket => "DocMemberSymbol_RightBracket",
            Self::ParamBlock_ParameterName => "ParamBlock_ParameterName",
            Self::ParamBlock_Hyphen => "ParamBlock_Hyphen",
            Self::PlainText => "PlainText",
            Self::SoftBreak => "SoftBreak",
        }
    }
}

/// A run of tokens a parsed node was read from, as a child of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DocExcerpt {
    /// What part of the node it is.
    pub excerpt_kind: ExcerptKind,
    /// Its tokens.
    pub content: TokenSequence,
}

impl DocExcerpt {
    pub(crate) fn new(excerpt_kind: ExcerptKind, content: TokenSequence) -> Self {
        Self { excerpt_kind, content }
    }

    /// An excerpt of `content`, if there is one.
    pub(crate) fn of(excerpt_kind: ExcerptKind, content: Option<TokenSequence>) -> Option<Self> {
        content.map(|content| Self::new(excerpt_kind, content))
    }

    fn push_children<'a>(&'a self, _: &mut Vec<NodeRef<'a>>) {}
}
