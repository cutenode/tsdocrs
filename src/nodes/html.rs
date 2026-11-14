//! HTML tags.

use super::{DocExcerpt, NodeRef, push};

/// An HTML start tag, like `<a href="#">` or `<br/>`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocHtmlStartTag {
    /// The element's name.
    pub name: String,
    /// The spacing after its name.
    pub spacing_after_name: Option<String>,
    /// Its attributes.
    pub html_attributes: Vec<DocHtmlAttribute>,
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

impl DocHtmlStartTag {
    pub(super) fn push_children<'a>(&'a self, children: &mut Vec<NodeRef<'a>>) {
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
pub struct DocHtmlEndTag {
    /// The element's name.
    pub name: String,
    /// Its `</`, name, spacing and `>` excerpts, when it was parsed.
    pub excerpts: HtmlTagExcerpts,
}

impl DocHtmlEndTag {
    pub(super) fn push_children<'a>(&'a self, children: &mut Vec<NodeRef<'a>>) {
        let e = &self.excerpts;
        push(children, &e.opening_delimiter);
        push(children, &e.name);
        push(children, &e.spacing_after_name);
        push(children, &e.closing_delimiter);
    }
}

/// An HTML attribute, like `href="#"`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocHtmlAttribute {
    /// Its name.
    pub name: String,
    /// The spacing after its name.
    pub spacing_after_name: Option<String>,
    /// The spacing after its `=`.
    pub spacing_after_equals: Option<String>,
    /// Its value, quotes and all.
    pub value: String,
    /// The spacing after its value.
    pub spacing_after_value: Option<String>,
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

impl DocHtmlAttribute {
    pub(super) fn push_children<'a>(&'a self, children: &mut Vec<NodeRef<'a>>) {
        let e = &self.excerpts;
        for excerpt in
            [&e.name, &e.spacing_after_name, &e.equals, &e.spacing_after_equals, &e.value, &e.spacing_after_value]
        {
            push(children, excerpt);
        }
    }
}
