//! Declaration references, as in `{@link my-package#MyClass.(method:static)}`.

use super::{DocExcerpt, ExcerptKind, NodeRef, push};
use crate::parser::{TokenSequence, string_checks};

/// A reference to a declaration: a package and import path, then members.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocDeclarationReference {
    /// The package's name.
    pub package_name: Option<String>,
    /// The import path, after the package's name.
    pub import_path: Option<String>,
    /// The members, from the outermost.
    pub member_references: Vec<DocMemberReference>,
    /// Its excerpts, when it was parsed.
    pub excerpts: DeclarationReferenceExcerpts,
}

/// A parsed declaration reference's excerpts, before its members.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct DeclarationReferenceExcerpts {
    pub package_name: Option<DocExcerpt>,
    pub import_path: Option<DocExcerpt>,
    pub import_hash: Option<DocExcerpt>,
    pub spacing_after_import_hash: Option<DocExcerpt>,
}

impl DocDeclarationReference {
    pub(super) fn push_children<'a>(&'a self, children: &mut Vec<NodeRef<'a>>) {
        let e = &self.excerpts;
        for excerpt in [&e.package_name, &e.import_path, &e.import_hash, &e.spacing_after_import_hash] {
            push(children, excerpt);
        }
        children.extend(self.member_references.iter().map(NodeRef::MemberReference));
    }
}

/// One member of a declaration reference: an identifier or a symbol, with an optional selector.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocMemberReference {
    /// Whether a `.` comes before it.
    pub has_dot: bool,
    /// Its identifier, unless it's a symbol.
    pub member_identifier: Option<DocMemberIdentifier>,
    /// Its symbol, like `[MySymbols.example]`.
    pub member_symbol: Option<DocMemberSymbol>,
    /// Its selector, like `:static`.
    pub selector: Option<DocMemberSelector>,
    /// Its excerpts, when it was parsed.
    pub excerpts: MemberReferenceExcerpts,
}

/// A parsed member reference's excerpts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct MemberReferenceExcerpts {
    pub dot: Option<DocExcerpt>,
    pub spacing_after_dot: Option<DocExcerpt>,
    pub left_parenthesis: Option<DocExcerpt>,
    pub spacing_after_left_parenthesis: Option<DocExcerpt>,
    pub spacing_after_member: Option<DocExcerpt>,
    pub colon: Option<DocExcerpt>,
    pub spacing_after_colon: Option<DocExcerpt>,
    pub spacing_after_selector: Option<DocExcerpt>,
    pub right_parenthesis: Option<DocExcerpt>,
    pub spacing_after_right_parenthesis: Option<DocExcerpt>,
}

impl DocMemberReference {
    pub(super) fn push_children<'a>(&'a self, children: &mut Vec<NodeRef<'a>>) {
        let e = &self.excerpts;
        push(children, &e.dot);
        push(children, &e.spacing_after_dot);
        push(children, &e.left_parenthesis);
        push(children, &e.spacing_after_left_parenthesis);
        push(children, &self.member_identifier);
        push(children, &self.member_symbol);
        push(children, &e.spacing_after_member);
        push(children, &e.colon);
        push(children, &e.spacing_after_colon);
        push(children, &self.selector);
        push(children, &e.spacing_after_selector);
        push(children, &e.right_parenthesis);
        push(children, &e.spacing_after_right_parenthesis);
    }
}

/// A member's name, quoted or not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocMemberIdentifier {
    /// The name, without quotes.
    pub identifier: String,
    /// Its excerpts, when it was parsed: the left quote, the name and the right quote.
    pub excerpts: [Option<DocExcerpt>; 3],
}

impl DocMemberIdentifier {
    /// A member name, built by hand.
    pub fn new(identifier: impl Into<String>) -> Self {
        Self { identifier: identifier.into(), excerpts: [None; 3] }
    }

    /// Whether `identifier` can go without quotes.
    pub fn is_valid_identifier(identifier: &str) -> bool {
        string_checks::explain_if_invalid_unquoted_member_identifier(identifier).is_none()
    }

    /// Whether it's quoted: as written when it was parsed, or when it needs to be otherwise.
    pub fn has_quotes(&self) -> bool {
        match self.excerpts {
            [left, Some(_), _] => left.is_some(),
            _ => !Self::is_valid_identifier(&self.identifier),
        }
    }

    pub(super) fn push_children<'a>(&'a self, children: &mut Vec<NodeRef<'a>>) {
        self.excerpts.iter().for_each(|excerpt| push(children, excerpt));
    }
}

/// A member named by a symbol, in square brackets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocMemberSymbol {
    /// The symbol's declaration.
    pub symbol_reference: DocDeclarationReference,
    /// Its excerpts, when it was parsed: the `[`, the spacing after it and the `]`.
    pub excerpts: [Option<DocExcerpt>; 3],
}

impl DocMemberSymbol {
    pub(super) fn push_children<'a>(&'a self, children: &mut Vec<NodeRef<'a>>) {
        let [left, spacing, right] = &self.excerpts;
        push(children, left);
        push(children, spacing);
        children.push(NodeRef::DeclarationReference(&self.symbol_reference));
        push(children, right);
    }
}

/// What a member selector is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectorKind {
    /// Not a valid selector.
    Error,
    /// One of TSDoc's, like `static` or `class`.
    System,
    /// An overload's number, from 1.
    Index,
    /// A `{@label}`'s name, like `WITH_NAME`.
    Label,
}

/// A member selector, after the colon in `(method:static)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocMemberSelector {
    /// The selector.
    pub selector: String,
    /// What it is.
    pub selector_kind: SelectorKind,
    /// What's wrong with it, when its kind is [`SelectorKind::Error`].
    pub error_message: Option<String>,
    /// Its excerpt, when it was parsed.
    pub selector_excerpt: Option<DocExcerpt>,
}

impl DocMemberSelector {
    /// The selector `selector`, built by hand.
    pub fn new(selector: impl Into<String>) -> Self {
        Self::parsed(selector.into(), None)
    }

    pub(crate) fn parsed(selector: String, excerpt: Option<TokenSequence>) -> Self {
        let (selector_kind, error_message) = classify(&selector);
        let selector_excerpt = DocExcerpt::of(ExcerptKind::MemberSelector, excerpt);
        Self { selector, selector_kind, error_message, selector_excerpt }
    }

    pub(super) fn push_children<'a>(&'a self, children: &mut Vec<NodeRef<'a>>) {
        push(children, &self.selector_excerpt);
    }
}

/// What `selector` is, and what's wrong with it if it isn't valid.
fn classify(selector: &str) -> (SelectorKind, Option<String>) {
    let error = |text: String| (SelectorKind::Error, Some(text));
    let first = selector.chars().next();
    if first.is_none() {
        return error("The selector cannot be an empty string".into());
    }
    if first.is_some_and(|c| c.is_ascii_digit()) {
        let index =
            selector.starts_with(|c: char| ('1'..='9').contains(&c)) && selector.chars().all(|c| c.is_ascii_digit());
        if index {
            return (SelectorKind::Index, None);
        }
        return error("If the selector begins with a number, it must be a positive integer value".into());
    }
    if first.is_some_and(|c| c.is_ascii_uppercase() || c == '_') {
        let label =
            selector.len() >= 2 && selector.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
        if label {
            return (SelectorKind::Label, None);
        }
        return error(
            "A label selector must be comprised of upper case letters, numbers, and underscores and must not start with a number"
                .into(),
        );
    }
    if string_checks::is_system_selector(selector) {
        return (SelectorKind::System, None);
    }
    if selector.chars().all(|c| c.is_ascii_lowercase()) {
        let quoted = string_checks::json_stringify(selector);
        return error(format!("The selector {quoted} is not a recognized TSDoc system selector name"));
    }
    error("Invalid syntax for selector".into())
}
