//! What the upstream tests and the properties leave out: error messages, builders, and how numbers
//! and JSON escapes read and write.

use super::*;

fn message<T: fmt::Debug>(result: Result<T, Error>) -> (ErrorKind, String) {
    let error = result.unwrap_err();
    assert_eq!(error.to_string(), error.message);
    (error.kind, error.message)
}

fn syntax(message: &str) -> (ErrorKind, String) {
    (ErrorKind::Syntax, message.into())
}

#[test]
fn parse_errors_list_what_went_wrong() {
    for (text, expected) in [
        (
            "a:(x",
            "Invalid DeclarationReference 'a:(x':\n  Decimal digit expected\n  Expected token ')', received '<text>' instead.",
        ),
        ("a:(1", "Invalid DeclarationReference 'a:(1':\n  Expected token ')', received '<eof>' instead."),
        ("a.[b", "Invalid DeclarationReference 'a.[b':\n  Expected token ']', received '<eof>' instead."),
        ("\"abc", "Invalid DeclarationReference '\"abc':\n  String is unterminated"),
        ("\"ab\n!x", "Invalid DeclarationReference '\"ab\n!x':\n  Module source is unterminated"),
        ("a:", "Invalid DeclarationReference 'a:':\n  Decimal digit expected"),
        ("\"\\u{zz}\"", "Invalid DeclarationReference '\"\\u{zz}\"':\n  String is unterminated"),
        ("\"\\01\"", "Invalid DeclarationReference '\"\\01\"':\n  String is unterminated"),
        ("\"\\x4\"", "Invalid DeclarationReference '\"\\x4\"':\n  String is unterminated"),
        ("[!x]", "Invalid DeclarationReference '[!x]'"),
    ] {
        assert_eq!(message(DeclarationReference::parse(text)), syntax(expected), "{text:?}");
    }
}

#[test]
fn references_write_out_as_upstream_does() {
    for (text, written) in [
        ("a..b", "a..b"),
        ("!", "!"),
        ("m!~", "m!"),
        (":class", ":class"),
        ("\"\\u{1F600}\"", "\"\\u{1F600}\""),
        ("a:99999999999999999999999999", "a:1e+26"),
    ] {
        assert_eq!(DeclarationReference::parse(text).map(|reference| reference.to_string()).as_deref(), Ok(written));
    }
    let nines = format!("a:{}", "9".repeat(400));
    assert_eq!(DeclarationReference::parse(&nines).unwrap().to_string(), "a:Infinity");
}

#[test]
fn overload_indexes_write_out_as_javascript_numbers() {
    for (index, written) in [
        (1e21, ":1e+21"),
        (-1e21, ":-1e+21"),
        (1.5e-7, ":1.5e-7"),
        (0.000001, ":0.000001"),
        (0.5, ":0.5"),
        (-0.0, ":0"),
        (f64::NAN, ":NaN"),
        (f64::INFINITY, ":Infinity"),
        (f64::NEG_INFINITY, ":-Infinity"),
        (123456789012345680000.0, ":123456789012345680000"),
        (1.2345e25, ":1.2345e+25"),
        (-2.5, ":-2.5"),
    ] {
        assert_eq!(SymbolReference::empty().with_overload_index(Some(index)).to_string(), written, "{index}");
    }
}

#[test]
fn module_sources_report_bad_packages_and_paths() {
    let other = |message: &str| (ErrorKind::Other, message.to_owned());
    assert_eq!(message(ModuleSource::from_package("", None)), other("Parsed package must be provided."));
    assert_eq!(message(ModuleSource::from_package("a/", None)), other("Parsed package must be provided."));
    assert_eq!(
        message(ModuleSource::from_package("A B", None)),
        syntax("Invalid NPM package name: The package name \"A B\" is not a valid package name")
    );
    assert_eq!(
        message(ModuleSource::from_scoped_package(Some("@"), "y", None)),
        syntax("Invalid NPM package name: The package name \"@/y\" is not a valid package name")
    );
    for import_path in ["./b", "../b", ".", "..", ".\\b", "/b", "\\b"] {
        let expected = format!("Invalid import path '{import_path}");
        assert_eq!(message(ModuleSource::from_package("a", Some(import_path))), syntax(&expected));
    }
    assert_eq!(ModuleSource::from_package("a", Some("..b")).unwrap().escaped_path(), "a/..b");
    assert_eq!(message(ModuleSource::from_package("a", Some("b!"))), syntax("Invalid Module source 'a/b!'"));
    assert_eq!(message(ModuleSource::new("a!b", true)), syntax("Invalid Module source 'a!b'"));
    assert_eq!(message(ComponentString::new("a.b", true)), syntax("Invalid Component 'a.b'"));
    assert_eq!(message(ComponentReference::parse("[]")), syntax("Invalid component reference: '[]'"));
    assert_eq!(message(DeclarationReference::package("A B", None)).0, ErrorKind::Syntax);
    assert_eq!(message(DeclarationReference::module("a!b", true)).0, ErrorKind::Syntax);
}

#[test]
fn module_paths_outside_a_package_are_import_paths() {
    let source = ModuleSource::new("@a//b", true).unwrap();
    assert_eq!(source.package_name().as_deref(), Ok(""));
    assert_eq!(source.import_path().as_deref(), Ok("@a//b"));
    let source = ModuleSource::new("a/b\nc", true).unwrap();
    assert_eq!(source.import_path().as_deref(), Ok("a/b\nc"));
    // A scoped name given as the unscoped one keeps the parts it was given.
    let source = ModuleSource::from_scoped_package(None, "@x/y", None).unwrap();
    assert_eq!(source.scope_name().as_deref(), Ok(""));
    assert_eq!(source.unscoped_package_name().as_deref(), Ok("@x/y"));
    // Well-formed, but not JSON.
    let source = ModuleSource::new("\"\\x41\"", true).unwrap();
    let invalid = syntax("Invalid Module source '\"\\x41\"'");
    assert_eq!(message(source.path()), invalid);
    assert_eq!(message(source.package_name()), invalid);
    assert_eq!(message(source.scope_name()), invalid);
    assert_eq!(message(source.unscoped_package_name()), invalid);
    assert_eq!(message(source.import_path()), invalid);
}

#[test]
fn quoted_strings_unescape_as_json() {
    let unescape = |text: &str| ModuleSource::new(text, true).and_then(|source| source.path()).map_err(|_| ());
    assert_eq!(unescape("\"a\\/b\""), Ok("a/b".into()));
    assert_eq!(unescape("\"\\b\\f\\n\\r\\t\\\\\\\"\""), Ok("\u{8}\u{c}\n\r\t\\\"".into()));
    assert_eq!(unescape("\"\\ud83d\\ude00\""), Ok("\u{1f600}".into()));
    assert_eq!(unescape("\"\\u00E9\""), Ok("\u{e9}".into()));
    assert_eq!(unescape("\"a\tb\""), Err(()));
    assert_eq!(unescape("\"\\'\""), Err(()));
    assert_eq!(unescape("\"\\u{41}\""), Err(()));
    // Lone surrogates, which a Rust string can't hold.
    assert_eq!(unescape("\"\\ud83d\""), Ok("\u{fffd}".into()));
    assert_eq!(unescape("\"\\ude00\\ud83d\""), Ok("\u{fffd}\u{fffd}".into()));
    assert_eq!(unescape("\"\\ud83dx\""), Ok("\u{fffd}x".into()));
    assert_eq!(unescape("\"\\ud83d\\n\""), Ok("\u{fffd}\n".into()));
    assert_eq!(parse_json_string(" \"a\" \n"), Some("a".into()));
    assert_eq!(parse_json_string("\"a\" b"), None);
    assert_eq!(parse_json_string("\"a"), None);
    assert_eq!(parse_json_string("\"\\u12\""), None);
}

#[test]
fn builders_replace_one_part() {
    let a = || Component::String(ComponentString::new("a", false).unwrap());
    let reference = DeclarationReference::global()
        .with_component_path(ComponentRoot::new(a()).into())
        .with_navigation(Some(Navigation::Exports));
    assert_eq!(reference.to_string(), "!a");
    assert_eq!(reference.navigation(), Some(Navigation::Locals));
    let reference = reference.with_component_path(ComponentRoot::new(a()).with_component("b").into());
    assert_eq!(reference.to_string(), "!b");
    assert_eq!(DeclarationReference::empty().with_meaning(Some(Meaning::Class)).to_string(), ":class");
    assert_eq!(DeclarationReference::empty().with_overload_index(Some(2.0)).to_string(), ":2");
    assert!(DeclarationReference::empty().with_meaning(None).is_empty());
    assert!(DeclarationReference::empty().with_overload_index(None).is_empty());
    let reference = DeclarationReference::from(None).with_meaning(Some(Meaning::Enum)).with_overload_index(Some(1.0));
    assert_eq!(reference.to_string(), ":enum(1)");
    let reference = reference.with_meaning(None).with_source(Some(ModuleSource::new("m", true).unwrap().into()));
    assert_eq!(reference.to_string(), "m!:1");
    assert_eq!(reference.clone().with_symbol(None).to_string(), "m!");
    assert_eq!(DeclarationReference::from(Some(reference.clone())), reference);
    assert_eq!(DeclarationReference::module("a!b", false).unwrap().to_string(), "\"a!b\"!");
    assert_eq!(DeclarationReference::package("@s/p", Some("x")).unwrap().to_string(), "@s/p/x!");
    assert_eq!(
        message(SymbolReference::empty().add_navigation_step(Navigation::Exports, "a")),
        (ErrorKind::Other, "Cannot add a navigation step to an empty symbol reference.".into())
    );
}

#[test]
fn component_builders_replace_one_part() {
    let string = |text: &str| Component::String(ComponentString::new(text, false).unwrap());
    let navigation = ComponentNavigation::new(ComponentRoot::new(string("a")).into(), Navigation::Members, string("b"))
        .with_navigation(Navigation::Locals)
        .with_component("c.d")
        .with_parent(ComponentRoot::new(string("[x")).into());
    assert_eq!(navigation.to_string(), "\"[x\"~\"c.d\"");
    assert_eq!(navigation.parent().component().to_string(), "\"[x\"");
    let reference = DeclarationReference::parse("x!y").unwrap();
    assert_eq!(ComponentRoot::new(string("a")).with_component(reference.clone()).to_string(), "[x!y]");
    let component = ComponentReference::new(DeclarationReference::empty()).with_reference(reference.clone());
    assert_eq!(component.reference(), &reference);
    let path = ComponentPath::from(ComponentRoot::new(Component::Reference(component.clone())));
    assert_eq!(path.add_navigation_step(Navigation::Exports, String::from("z")).to_string(), "[x!y].z");
    assert_eq!(Component::from(ComponentLike::from(ComponentString::new("q", true).unwrap())).to_string(), "q");
    assert_eq!(Component::from(ComponentLike::from(component)).to_string(), "[x!y]");
    assert_eq!(DeclarationReference::parse_component("[x!y]").unwrap().to_string(), "[x!y]");
    assert_eq!(message(DeclarationReference::parse_component("a.b")), syntax("Invalid Component 'a.b'"));
}

#[test]
fn navigations_and_meanings_have_their_keywords() {
    let navigations = [Navigation::Exports, Navigation::Members, Navigation::Locals].map(|n| n.to_string());
    assert_eq!(navigations, [".", "#", "~"]);
    for meaning in Meaning::ALL {
        assert_eq!(Meaning::from_keyword(&meaning.to_string()), Some(meaning));
    }
    assert_eq!(Meaning::from_keyword("variable"), None);
    assert_eq!(Source::from(GlobalSource).to_string(), "!");
}

#[test]
fn expected_tokens_name_what_came_instead() {
    for (after, received) in [
        ("{", "{"),
        ("}", "}"),
        ("(", "("),
        ("[", "["),
        ("]", "]"),
        ("!", "!"),
        (".", "."),
        ("#", "#"),
        ("~", "~"),
        (":", ":"),
        (",", ","),
        ("@", "@"),
        ("\"x\"", "<string>"),
    ] {
        // A source first, so a `!` isn't read as the end of one.
        let text = format!("m!a:(1{after}");
        let expected =
            format!("Invalid DeclarationReference '{text}':\n  Expected token ')', received '{received}' instead.");
        assert_eq!(message(DeclarationReference::parse(&text)), syntax(&expected));
    }
}

#[test]
fn paths_end_in_their_last_component() {
    let a = Component::String(ComponentString::new("a", true).unwrap());
    let path = ComponentPath::from(ComponentRoot::new(a.clone())).add_navigation_step(Navigation::Members, a.clone());
    assert_eq!(path.component(), &a);
    assert_eq!(path.to_string(), "a#a");
}
