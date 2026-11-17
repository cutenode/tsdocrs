//! HTML tags.

use std::borrow::Cow;

use super::{DocExcerpt, NodeRef, push};

/// An HTML start tag, like `<a href="#">` or `<br/>`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocHtmlStartTag<'a> {
    /// The element's name.
    pub name: Cow<'a, str>,
    /// The spacing after its name.
    pub spacing_after_name: Option<Cow<'a, str>>,
    /// Its attributes.
    pub html_attributes: Vec<DocHtmlAttribute<'a>>,
    /// Whether it ends in `/>`.
    pub self_closing_tag: bool,
    /// Its `<`, name, spacing and closing excerpts, when it was parsed.
    pub excerpts: HtmlTagExcerpts,
}

/// A parsed HTML tag's excerpts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct HtmlTagExcerpts {
    pub opening_delimiter: Option<DocExcerpt>,
    pub name: Option<DocExcerpt>,
    pub spacing_after_name: Option<DocExcerpt>,
    pub closing_delimiter: Option<DocExcerpt>,
}

impl<'a> DocHtmlStartTag<'a> {
    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        let e = &self.excerpts;
        push(children, &e.opening_delimiter);
        push(children, &e.name);
        push(children, &e.spacing_after_name);
        children.extend(self.html_attributes.iter().map(NodeRef::HtmlAttribute));
        push(children, &e.closing_delimiter);
    }
}

/// An HTML end tag, like `</a>`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocHtmlEndTag<'a> {
    /// The element's name.
    pub name: Cow<'a, str>,
    /// Its `</`, name, spacing and `>` excerpts, when it was parsed.
    pub excerpts: HtmlTagExcerpts,
}

impl<'a> DocHtmlEndTag<'a> {
    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        let e = &self.excerpts;
        push(children, &e.opening_delimiter);
        push(children, &e.name);
        push(children, &e.spacing_after_name);
        push(children, &e.closing_delimiter);
    }
}

/// An HTML attribute, like `href="#"`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocHtmlAttribute<'a> {
    /// Its name.
    pub name: Cow<'a, str>,
    /// The spacing after its name.
    pub spacing_after_name: Option<Cow<'a, str>>,
    /// The spacing after its `=`.
    pub spacing_after_equals: Option<Cow<'a, str>>,
    /// Its value, quotes and all.
    pub value: Cow<'a, str>,
    /// The spacing after its value.
    pub spacing_after_value: Option<Cow<'a, str>>,
    /// Its excerpts, when it was parsed.
    pub excerpts: HtmlAttributeExcerpts,
}

/// A parsed HTML attribute's excerpts, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct HtmlAttributeExcerpts {
    pub name: Option<DocExcerpt>,
    pub spacing_after_name: Option<DocExcerpt>,
    pub equals: Option<DocExcerpt>,
    pub spacing_after_equals: Option<DocExcerpt>,
    pub value: Option<DocExcerpt>,
    pub spacing_after_value: Option<DocExcerpt>,
}

impl<'a> DocHtmlAttribute<'a> {
    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        let e = &self.excerpts;
        for excerpt in
            [&e.name, &e.spacing_after_name, &e.equals, &e.spacing_after_equals, &e.value, &e.spacing_after_value]
        {
            push(children, excerpt);
        }
    }
}
