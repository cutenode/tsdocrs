//! Each node kind's own fields, by the names JavaScript gets them as, for both of the package's tree
//! formats: `parse()`'s JSON and `parseTrees()`'s numbers.

use tsdocrs::nodes::{NodeRef, SelectorKind};

/// A field's value.
pub(crate) enum Value<'a> {
    Text(&'a str),
    Bool(bool),
}

/// Calls `f` with each of `node`'s own fields, in order: what its kind has besides its children,
/// and besides an excerpt's kind and text, which each format writes its own way.
pub(crate) fn for_each<'a>(node: NodeRef<'a>, mut f: impl FnMut(&'static str, Value<'a>)) {
    use Value::{Bool, Text};
    match node {
        NodeRef::BlockTag(tag) => f("tagName", Text(&tag.tag_name)),
        NodeRef::PlainText(text) => f("text", Text(&text.text)),
        NodeRef::ErrorText(error) => {
            f("text", Text(&error.text));
            f("messageId", Text(error.message_id.as_str()));
            f("errorMessage", Text(&error.error_message));
        }
        NodeRef::EscapedText(escaped) => {
            f("encodedText", Text(&escaped.encoded_text));
            f("decodedText", Text(&escaped.decoded_text));
        }
        NodeRef::CodeSpan(code) => f("code", Text(&code.code)),
        NodeRef::FencedCode(code) => {
            f("language", Text(&code.language));
            f("code", Text(&code.code));
        }
        NodeRef::HtmlStartTag(tag) => {
            f("name", Text(&tag.name));
            f("selfClosingTag", Bool(tag.self_closing_tag));
        }
        NodeRef::HtmlEndTag(tag) => f("name", Text(&tag.name)),
        NodeRef::HtmlAttribute(attribute) => {
            f("name", Text(&attribute.name));
            f("value", Text(&attribute.value));
        }
        NodeRef::InlineTag(tag) => {
            f("tagName", Text(&tag.base.tag_name));
            f("tagContent", Text(&tag.tag_content));
        }
        NodeRef::LinkTag(link) => {
            f("tagName", Text(&link.base.tag_name));
            if let Some(url) = &link.url_destination {
                f("urlDestination", Text(url));
            }
            if let Some(text) = &link.link_text {
                f("linkText", Text(text));
            }
        }
        NodeRef::InheritDocTag(tag) => f("tagName", Text(&tag.base.tag_name)),
        NodeRef::ParamBlock(block) => f("parameterName", Text(&block.parameter_name)),
        NodeRef::DeclarationReference(reference) => {
            if let Some(name) = &reference.package_name {
                f("packageName", Text(name));
            }
            if let Some(path) = &reference.import_path {
                f("importPath", Text(path));
            }
        }
        NodeRef::MemberIdentifier(identifier) => {
            f("identifier", Text(&identifier.identifier));
            f("hasQuotes", Bool(identifier.has_quotes()));
        }
        NodeRef::MemberReference(member) => f("hasDot", Bool(member.has_dot)),
        NodeRef::MemberSelector(selector) => {
            f("selector", Text(&selector.selector));
            let kind = match selector.selector_kind {
                SelectorKind::Error => "error",
                SelectorKind::System => "system",
                SelectorKind::Index => "index",
                SelectorKind::Label => "label",
            };
            f("selectorKind", Text(kind));
        }
        _ => {}
    }
}
