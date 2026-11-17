//! Inline tags: `{@link}`, `{@inheritDoc}` and the rest.

use std::borrow::Cow;

use super::{DocDeclarationReference, DocExcerpt, NodeRef, push};

/// What every inline tag has: its name and the excerpts around its content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineTagBase<'a> {
    /// Its name, with its `@`.
    pub tag_name: Cow<'a, str>,
    /// The same, upper-cased.
    pub tag_name_with_upper_case: String,
    /// Its `{`, when it was parsed.
    pub opening_delimiter_excerpt: Option<DocExcerpt>,
    /// Its name, when it was parsed.
    pub tag_name_excerpt: Option<DocExcerpt>,
    /// The spacing after its name, when it was parsed.
    pub spacing_after_tag_name_excerpt: Option<DocExcerpt>,
    /// Its `}`, when it was parsed.
    pub closing_delimiter_excerpt: Option<DocExcerpt>,
}

impl<'a> InlineTagBase<'a> {
    /// A tag named `tag_name`, built by hand, or why it can't be.
    pub fn new(tag_name: &str) -> Result<Self, String> {
        crate::parser::string_checks::validate_tsdoc_tag_name(tag_name)?;
        Ok(Self {
            tag_name: Cow::Owned(tag_name.to_owned()),
            tag_name_with_upper_case: tag_name.to_uppercase(),
            opening_delimiter_excerpt: None,
            tag_name_excerpt: None,
            spacing_after_tag_name_excerpt: None,
            closing_delimiter_excerpt: None,
        })
    }

    /// Adds its children around `content`'s.
    fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>, content: impl FnOnce(&mut Vec<NodeRef<'r>>)) {
        push(children, &self.opening_delimiter_excerpt);
        push(children, &self.tag_name_excerpt);
        push(children, &self.spacing_after_tag_name_excerpt);
        content(children);
        push(children, &self.closing_delimiter_excerpt);
    }
}

/// An inline tag TSDoc has no syntax of its own for, like `{@label}` or a custom one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocInlineTag<'a> {
    /// Its name and delimiters.
    pub base: InlineTagBase<'a>,
    /// Its content, or empty.
    pub tag_content: Cow<'a, str>,
    /// Its content's excerpt, when it was parsed and has content.
    pub tag_content_excerpt: Option<DocExcerpt>,
}

impl<'a> DocInlineTag<'a> {
    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        self.base.push_children(children, |children| push(children, &self.tag_content_excerpt));
    }
}

/// A `{@link}` tag, to a declaration or a URL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocLinkTag<'a> {
    /// Its name and delimiters.
    pub base: InlineTagBase<'a>,
    /// The declaration it links to.
    pub code_destination: Option<DocDeclarationReference<'a>>,
    /// The URL it links to.
    pub url_destination: Option<Cow<'a, str>>,
    /// The text after its `|`.
    pub link_text: Option<Cow<'a, str>>,
    /// Its excerpts, when it was parsed.
    pub excerpts: LinkTagExcerpts,
}

/// A parsed `{@link}`'s excerpts, in order after its declaration reference.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct LinkTagExcerpts {
    pub url_destination: Option<DocExcerpt>,
    pub spacing_after_destination: Option<DocExcerpt>,
    pub pipe: Option<DocExcerpt>,
    pub spacing_after_pipe: Option<DocExcerpt>,
    pub link_text: Option<DocExcerpt>,
    pub spacing_after_link_text: Option<DocExcerpt>,
}

impl<'a> DocLinkTag<'a> {
    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        self.base.push_children(children, |children| {
            let e = &self.excerpts;
            push(children, &self.code_destination);
            for excerpt in [
                &e.url_destination,
                &e.spacing_after_destination,
                &e.pipe,
                &e.spacing_after_pipe,
                &e.link_text,
                &e.spacing_after_link_text,
            ] {
                push(children, excerpt);
            }
        });
    }
}

/// An `{@inheritDoc}` tag, which takes the docs of the declaration it names, or of what the item
/// overrides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocInheritDocTag<'a> {
    /// Its name and delimiters.
    pub base: InlineTagBase<'a>,
    /// The declaration it takes the docs of.
    pub declaration_reference: Option<DocDeclarationReference<'a>>,
}

impl<'a> DocInheritDocTag<'a> {
    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        self.base.push_children(children, |children| push(children, &self.declaration_reference));
    }
}
