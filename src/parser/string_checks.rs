//! `@microsoft/tsdoc`'s StringChecks: the syntax checks for names and identifiers.
//!
//! Upstream writes these as case-insensitive regular expressions without the `u` flag, which match
//! ASCII letters only (non-ASCII letters that uppercase to ASCII, like `ſ`, don't match), so the
//! hand-written checks here compare ASCII bytes.

#[cfg(test)]
mod tests;

/// The selectors TSDoc reserves, which a member identifier must quote.
const SYSTEM_SELECTORS: [&str; 10] =
    ["instance", "static", "constructor", "class", "enum", "function", "interface", "namespace", "type", "variable"];

/// Why `tag_name` isn't a TSDoc tag name: an `@` and then ASCII letters and digits, starting with a
/// letter.
pub(crate) fn explain_if_invalid_tsdoc_tag_name(tag_name: &str) -> Option<&'static str> {
    let Some(name) = tag_name.strip_prefix('@') else {
        return Some("A TSDoc tag name must start with an \"@\" symbol");
    };
    let is_valid =
        name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic) && name.bytes().all(|b| b.is_ascii_alphanumeric());
    (!is_valid).then_some("A TSDoc tag name must start with a letter and contain only letters and numbers")
}

/// [`explain_if_invalid_tsdoc_tag_name`]'s explanation as an error, where upstream throws it.
pub(crate) fn validate_tsdoc_tag_name(tag_name: &str) -> Result<(), String> {
    explain_if_invalid_tsdoc_tag_name(tag_name).map_or(Ok(()), |explanation| Err(explanation.into()))
}

/// Why `url` can't be an `@link` URL: one needs a scheme of ASCII letters and digits, `://`, and a
/// character after it.
pub(crate) fn explain_if_invalid_link_url(url: &str) -> Option<&'static str> {
    if url.is_empty() {
        return Some("The URL cannot be empty");
    }
    let Some(after) = after_scheme(url) else {
        return Some(concat!(
            "An @link URL must begin with a scheme comprised only of letters and numbers followed by \"://\".",
            " (For general URLs, use an HTML \"<a>\" tag instead.)"
        ));
    };
    // The regular expression's `.`, which matches anything but a line terminator.
    let has_character = after.chars().next().is_some_and(|c| !is_line_terminator(c));
    (!has_character).then_some("An @link URL must have at least one character after \"://\"")
}

/// What follows `url`'s scheme and `://`, if it starts with them.
fn after_scheme(url: &str) -> Option<&str> {
    if !url.as_bytes().first()?.is_ascii_alphabetic() {
        return None;
    }
    url.trim_start_matches(|c: char| c.is_ascii_alphanumeric()).strip_prefix("://")
}

/// Whether `c` ends a line for a JavaScript regular expression's `.`.
pub(crate) fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// Why `html_name` isn't an HTML element or attribute name: an ASCII letter and then ASCII letters,
/// digits and hyphens, as CommonMark has it.
pub(crate) fn explain_if_invalid_html_name(html_name: &str) -> Option<&'static str> {
    let is_valid = html_name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && html_name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
    (!is_valid).then_some("An HTML name must be an ASCII letter followed by zero or more letters, digits, or hyphens")
}

/// [`explain_if_invalid_html_name`]'s explanation as an error, where upstream throws it.
pub(crate) fn validate_html_name(html_name: &str) -> Result<(), String> {
    explain_if_invalid_html_name(html_name).map_or(Ok(()), |explanation| Err(explanation.into()))
}

/// Why `package_name` isn't an NPM package name, with or without a scope.
pub(crate) fn explain_if_invalid_package_name(package_name: &str) -> Option<String> {
    if package_name.is_empty() {
        return Some("The package name cannot be an empty string".into());
    }
    let name = match package_name.strip_prefix('@') {
        Some(scoped) => scoped.split_once('/').filter(|(scope, _)| is_package_name_part(scope)).map(|(_, name)| name),
        None => Some(package_name),
    };
    if name.is_some_and(is_package_name_part) {
        return None;
    }
    Some(format!("The package name {} is not a valid package name", json_stringify(package_name)))
}

/// Whether `part` can be a package's scope or its name after the scope.
fn is_package_name_part(part: &str) -> bool {
    !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// Why `import_path` can't be a declaration reference's import path.
pub(crate) fn explain_if_invalid_import_path(
    import_path: &str,
    prefixed_by_package_name: bool,
) -> Option<&'static str> {
    if import_path.is_empty() {
        return None;
    }
    if import_path.contains("//") {
        return Some("An import path must not contain \"//\"");
    }
    if import_path.ends_with('/') {
        return Some("An import path must not end with \"/\"");
    }
    if !prefixed_by_package_name && import_path.starts_with('/') {
        return Some("An import path must not start with \"/\" unless prefixed by a package name");
    }
    None
}

/// Whether `selector` is one of TSDoc's system selectors, like `static` or `class`.
pub(crate) fn is_system_selector(selector: &str) -> bool {
    SYSTEM_SELECTORS.contains(&selector)
}

/// Why `identifier` isn't an ECMAScript identifier, by upstream's conservative heuristic: ASCII
/// letters, digits, `_` and `$`, not starting with a digit.
pub(crate) fn explain_if_invalid_unquoted_identifier(identifier: &str) -> Option<&'static str> {
    if identifier.is_empty() {
        return Some("The identifier cannot be an empty string");
    }
    if !identifier.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'$')) {
        return Some("The identifier cannot non-word characters");
    }
    if identifier.as_bytes()[0].is_ascii_digit() {
        return Some("The identifier must not start with a number");
    }
    None
}

/// Why `identifier` needs quotes as a member identifier in a declaration reference.
pub(crate) fn explain_if_invalid_unquoted_member_identifier(identifier: &str) -> Option<String> {
    if let Some(explanation) = explain_if_invalid_unquoted_identifier(identifier) {
        return Some(explanation.into());
    }
    // A selector name would make `MyClass.(static:instance)` ambiguous about which side of the
    // colon is the selector.
    is_system_selector(identifier)
        .then(|| format!("The identifier \"{identifier}\" must be quoted because it is a TSDoc system selector name"))
}

/// `JSON.stringify` of a string: `text` in double quotes, with `"`, `\` and control characters
/// escaped.
pub(crate) fn json_stringify(text: &str) -> String {
    let mut json = String::with_capacity(text.len() + 2);
    json.push('"');
    for c in text.chars() {
        match c {
            '"' => json.push_str("\\\""),
            '\\' => json.push_str("\\\\"),
            '\u{8}' => json.push_str("\\b"),
            '\u{c}' => json.push_str("\\f"),
            '\n' => json.push_str("\\n"),
            '\r' => json.push_str("\\r"),
            '\t' => json.push_str("\\t"),
            c if c < ' ' => json.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => json.push(c),
        }
    }
    json.push('"');
    json
}
