//! Text and code: plain text, line breaks, escapes, mistakes, block tags, code spans and fences.

use std::borrow::Cow;

use super::{DocExcerpt, ExcerptKind, NodeRef, push};
use crate::parser::{TSDocMessageId, TokenSequence};

/// Plain text, on one line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocPlainText<'a> {
    /// The text.
    pub text: Cow<'a, str>,
    /// Its excerpt, when it was parsed.
    pub text_excerpt: Option<DocExcerpt>,
}

impl<'a> DocPlainText<'a> {
    /// Text without newlines, built by hand.
    pub fn new(text: impl Into<Cow<'a, str>>) -> Result<Self, String> {
        let text = text.into();
        if text.contains('\n') {
            return Err("The DocPlainText content must not contain newline characters".into());
        }
        Ok(Self { text, text_excerpt: None })
    }

    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        push(children, &self.text_excerpt);
    }
}

/// The end of a line, which Markdown reads as a space.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocSoftBreak {
    /// Its excerpt, when it was parsed.
    pub soft_break_excerpt: Option<DocExcerpt>,
}

impl DocSoftBreak {
    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        push(children, &self.soft_break_excerpt);
    }
}

/// Text the parser couldn't read, with what's wrong with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocErrorText<'a> {
    /// The text.
    pub text: Cow<'a, str>,
    /// Its excerpt.
    pub text_excerpt: DocExcerpt,
    /// What's wrong.
    pub message_id: TSDocMessageId,
    /// What's wrong, in TSDoc's words.
    pub error_message: String,
    /// Where the mistake is, which may be past the text.
    pub error_location: TokenSequence,
}

impl<'a> DocErrorText<'a> {
    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        children.push(NodeRef::Excerpt(&self.text_excerpt));
    }
}

/// How an escaped character was escaped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EscapeStyle {
    /// A backslash before it, as in CommonMark.
    CommonMarkBackslash,
}

/// An escaped character.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocEscapedText<'a> {
    /// How it's escaped.
    pub escape_style: EscapeStyle,
    /// The text as written, like `\{`.
    pub encoded_text: Cow<'a, str>,
    /// Its excerpt.
    pub encoded_text_excerpt: DocExcerpt,
    /// The character it stands for.
    pub decoded_text: Cow<'a, str>,
}

impl<'a> DocEscapedText<'a> {
    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        children.push(NodeRef::Excerpt(&self.encoded_text_excerpt));
    }
}

/// A block or modifier tag's name, like `@remarks`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocBlockTag<'a> {
    /// Its name, with its `@`.
    pub tag_name: Cow<'a, str>,
    /// The same, upper-cased.
    pub tag_name_with_upper_case: String,
    /// Its excerpt, when it was parsed.
    pub tag_name_excerpt: Option<DocExcerpt>,
}

impl<'a> DocBlockTag<'a> {
    /// A tag named `tag_name`, built by hand, or why it can't be.
    pub fn new(tag_name: &str) -> Result<Self, String> {
        crate::parser::string_checks::validate_tsdoc_tag_name(tag_name)?;
        Ok(Self::parsed(Cow::Owned(tag_name.to_owned()), None))
    }

    pub(crate) fn parsed(tag_name: Cow<'a, str>, excerpt: Option<TokenSequence>) -> Self {
        let tag_name_with_upper_case = tag_name.to_uppercase();
        let tag_name_excerpt = DocExcerpt::of(ExcerptKind::BlockTag, excerpt);
        Self { tag_name, tag_name_with_upper_case, tag_name_excerpt }
    }

    /// The tokens of its name, when it was parsed.
    pub fn token_sequence(&self) -> Option<TokenSequence> {
        self.tag_name_excerpt.map(|excerpt| excerpt.content)
    }

    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        push(children, &self.tag_name_excerpt);
    }
}

/// Code in backticks: `` `code` ``.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocCodeSpan<'a> {
    /// The code.
    pub code: Cow<'a, str>,
    /// The excerpts of its opening backtick, its code and its closing backtick, when it was
    /// parsed.
    pub excerpts: Option<[DocExcerpt; 3]>,
}

impl<'a> DocCodeSpan<'a> {
    /// A code span of `code`, built by hand.
    pub fn new(code: impl Into<Cow<'a, str>>) -> Self {
        Self { code: code.into(), excerpts: None }
    }

    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        children.extend(self.excerpts.iter().flatten().map(NodeRef::Excerpt));
    }
}

/// A fenced code block.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocFencedCode<'a> {
    /// Its language, or empty.
    pub language: Cow<'a, str>,
    /// Its code, up to and with the newline before the closing fence.
    pub code: Cow<'a, str>,
    /// Its excerpts, when it was parsed.
    pub excerpts: FencedCodeExcerpts,
}

/// A parsed fenced code block's excerpts, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct FencedCodeExcerpts {
    pub opening_fence: Option<DocExcerpt>,
    pub spacing_after_opening_fence: Option<DocExcerpt>,
    pub language: Option<DocExcerpt>,
    pub spacing_after_language: Option<DocExcerpt>,
    pub code: Option<DocExcerpt>,
    pub spacing_before_closing_fence: Option<DocExcerpt>,
    pub closing_fence: Option<DocExcerpt>,
    pub spacing_after_closing_fence: Option<DocExcerpt>,
}

impl<'a> DocFencedCode<'a> {
    /// A code block of `code` in `language`, built by hand.
    pub fn new(language: impl Into<Cow<'a, str>>, code: impl Into<Cow<'a, str>>) -> Self {
        Self { language: language.into(), code: code.into(), excerpts: FencedCodeExcerpts::default() }
    }

    pub(super) fn push_children<'r>(&'r self, children: &mut Vec<NodeRef<'r>>) {
        let e = &self.excerpts;
        for excerpt in [
            &e.opening_fence,
            &e.spacing_after_opening_fence,
            &e.language,
            &e.spacing_after_language,
            &e.code,
            &e.spacing_before_closing_fence,
            &e.closing_fence,
            &e.spacing_after_closing_fence,
        ] {
            push(children, excerpt);
        }
    }
}
