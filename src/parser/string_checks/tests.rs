use super::*;

const TAG_START: &str = "A TSDoc tag name must start with an \"@\" symbol";
const TAG_CHARACTERS: &str = "A TSDoc tag name must start with a letter and contain only letters and numbers";

#[test]
fn tsdoc_tag_names_are_an_at_sign_and_ascii_letters_and_digits() {
    for valid in ["@a", "@param", "@Z9", "@typeParam"] {
        assert_eq!(explain_if_invalid_tsdoc_tag_name(valid), None, "{valid}");
        assert_eq!(validate_tsdoc_tag_name(valid), Ok(()), "{valid}");
    }
    for (invalid, explanation) in [
        ("", TAG_START),
        ("param", TAG_START),
        ("@", TAG_CHARACTERS),
        ("@1a", TAG_CHARACTERS),
        ("@a-b", TAG_CHARACTERS),
        ("@a\n", TAG_CHARACTERS),
        ("@\u{17f}", TAG_CHARACTERS),
        ("@a\u{212a}", TAG_CHARACTERS),
    ] {
        assert_eq!(explain_if_invalid_tsdoc_tag_name(invalid), Some(explanation), "{invalid}");
        assert_eq!(validate_tsdoc_tag_name(invalid), Err(explanation.to_owned()), "{invalid}");
    }
}

#[test]
fn link_urls_need_a_scheme_and_something_after_it() {
    let scheme = concat!(
        "An @link URL must begin with a scheme comprised only of letters and numbers followed by \"://\".",
        " (For general URLs, use an HTML \"<a>\" tag instead.)"
    );
    let after = "An @link URL must have at least one character after \"://\"";
    for (url, explanation) in [
        ("https://example.com", None),
        ("a1://\u{1f600}", None),
        ("H://x", None),
        ("", Some("The URL cannot be empty")),
        ("1a://x", Some(scheme)),
        ("://x", Some(scheme)),
        ("a-b://x", Some(scheme)),
        ("mailto:x", Some(scheme)),
        ("\u{17f}://x", Some(scheme)),
        ("a://", Some(after)),
        ("a://\nx", Some(after)),
        ("a://\r", Some(after)),
        ("a://\u{2028}", Some(after)),
        ("a://\u{2029}", Some(after)),
    ] {
        assert_eq!(explain_if_invalid_link_url(url), explanation, "{url:?}");
    }
}

#[test]
fn html_names_are_an_ascii_letter_then_letters_digits_and_hyphens() {
    let explanation = "An HTML name must be an ASCII letter followed by zero or more letters, digits, or hyphens";
    for valid in ["a", "my-element", "H1", "a--"] {
        assert_eq!(explain_if_invalid_html_name(valid), None, "{valid}");
        assert_eq!(validate_html_name(valid), Ok(()), "{valid}");
    }
    for invalid in ["", "1a", "-a", "a_b", "a b", "\u{e9}", "a\n"] {
        assert_eq!(explain_if_invalid_html_name(invalid), Some(explanation), "{invalid:?}");
        assert_eq!(validate_html_name(invalid), Err(explanation.to_owned()), "{invalid:?}");
    }
}

#[test]
fn package_names_are_an_optional_scope_and_a_name() {
    for valid in ["a", "@a/b", "a-b_c.d", "@microsoft/rush-stack-compiler-3.5", "..", "@.../_"] {
        assert_eq!(explain_if_invalid_package_name(valid), None, "{valid}");
    }
    assert_eq!(explain_if_invalid_package_name("").as_deref(), Some("The package name cannot be an empty string"));
    for (invalid, quoted) in [
        ("@a", "\"@a\""),
        ("@/b", "\"@/b\""),
        ("@a/", "\"@a/\""),
        ("a/b", "\"a/b\""),
        ("@a/b/c", "\"@a/b/c\""),
        ("a@b", "\"a@b\""),
        ("a\"b\n", "\"a\\\"b\\n\""),
        ("\u{e9}", "\"\u{e9}\""),
    ] {
        let explanation = format!("The package name {quoted} is not a valid package name");
        assert_eq!(explain_if_invalid_package_name(invalid), Some(explanation), "{invalid:?}");
    }
}

#[test]
fn import_paths_have_no_empty_segments() {
    for (path, prefixed, explanation) in [
        ("", false, None),
        ("a/b", false, None),
        ("/a", true, None),
        ("a//b", true, Some("An import path must not contain \"//\"")),
        ("a/", true, Some("An import path must not end with \"/\"")),
        ("/", false, Some("An import path must not end with \"/\"")),
        ("/a", false, Some("An import path must not start with \"/\" unless prefixed by a package name")),
    ] {
        assert_eq!(explain_if_invalid_import_path(path, prefixed), explanation, "{path} {prefixed}");
    }
}

#[test]
fn system_selectors_are_the_reserved_names() {
    for selector in SYSTEM_SELECTORS {
        assert!(is_system_selector(selector));
    }
    for other in ["", "Class", "var", "member", "instances"] {
        assert!(!is_system_selector(other), "{other}");
    }
}

#[test]
fn unquoted_identifiers_are_ascii_words_not_starting_with_a_digit() {
    for valid in ["a", "_", "$", "a1", "A_$9"] {
        assert_eq!(explain_if_invalid_unquoted_identifier(valid), None, "{valid}");
        assert_eq!(explain_if_invalid_unquoted_member_identifier(valid), None, "{valid}");
    }
    for (invalid, explanation) in [
        ("", "The identifier cannot be an empty string"),
        ("a.b", "The identifier cannot non-word characters"),
        ("1.", "The identifier cannot non-word characters"),
        ("\u{e9}", "The identifier cannot non-word characters"),
        ("1a", "The identifier must not start with a number"),
    ] {
        assert_eq!(explain_if_invalid_unquoted_identifier(invalid), Some(explanation), "{invalid}");
        assert_eq!(explain_if_invalid_unquoted_member_identifier(invalid).as_deref(), Some(explanation), "{invalid}");
    }
}

#[test]
fn member_identifiers_quote_system_selectors() {
    assert_eq!(
        explain_if_invalid_unquoted_member_identifier("static").as_deref(),
        Some("The identifier \"static\" must be quoted because it is a TSDoc system selector name")
    );
}

#[test]
fn json_stringify_escapes_quotes_backslashes_and_control_characters() {
    assert_eq!(json_stringify(""), "\"\"");
    assert_eq!(json_stringify("a\"b\\c"), "\"a\\\"b\\\\c\"");
    assert_eq!(json_stringify("\u{8}\u{c}\n\r\t"), "\"\\b\\f\\n\\r\\t\"");
    assert_eq!(json_stringify("\u{0}\u{1f}\u{7f}"), "\"\\u0000\\u001f\u{7f}\"");
    assert_eq!(json_stringify("/\u{2028}\u{1f600}"), "\"/\u{2028}\u{1f600}\"");
}
