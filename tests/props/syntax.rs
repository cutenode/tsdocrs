//! Properties of TSDoc's small rules, each checked against upstream's own statement of it: the
//! regular expressions in `@microsoft/tsdoc`'s StringChecks and DocMemberSelector, written here
//! with the `regex` crate. Upstream's `/i` has no `u` flag, so it folds ASCII letters only, which
//! `[A-Za-z]` spells out. Also: where a message's line and column point, the names of the token
//! kinds, and a tsdoc.json read from disk or from memory.

use std::fs;

use hegel::{TestCase, generators as gs};
use regex::Regex;
use serde_json::{Value, json};
use tsdocrs::{
    TSDocConfigFile, TSDocMessageId as Id, TSDocParser, TSDocTagDefinition, TSDocTagSyntaxKind, config_file,
    nodes::{DocBlockTag, DocMemberIdentifier, DocMemberSelector, InlineTagBase, SelectorKind},
    parser::{TextRange, TokenKind, tokenizer},
};

use crate::common::generate::config;

/// TSDoc's system selectors, which a member identifier has to quote.
const SYSTEM_SELECTORS: [&str; 10] =
    ["instance", "static", "constructor", "class", "enum", "function", "interface", "namespace", "type", "variable"];

/// A string of up to `max` characters from `alphabet`.
fn string(tc: &TestCase, alphabet: &str, max: usize) -> String {
    tc.draw(gs::text().alphabet(alphabet).max_size(max))
}

/// The ids of what `comment` logs, with the default configuration.
fn logged(comment: &str) -> Vec<Id> {
    let context = TSDocParser::default().parse_string(comment).unwrap();
    context.log.iter().map(|m| m.message_id).collect()
}

/// A tag name is `@`, an ASCII letter, then letters and digits (`/^@[a-z][a-z0-9]*$/i`), wherever
/// a tag is made from one.
#[hegel::test(test_cases = 200)]
fn tag_names_are_an_at_sign_a_letter_then_letters_and_digits(tc: TestCase) {
    let rule = Regex::new("^@[A-Za-z][A-Za-z0-9]*$").unwrap();
    let name = format!("{}{}", if tc.draw(gs::booleans()) { "@" } else { "" }, string(&tc, "@aZ09_-é ſ", 5));
    let valid = rule.is_match(&name);
    assert_eq!(TSDocTagDefinition::new(&name, TSDocTagSyntaxKind::BlockTag, false).is_ok(), valid, "{name:?}");
    assert_eq!(DocBlockTag::new(&name).is_ok(), valid, "{name:?}");
    assert_eq!(InlineTagBase::new(&name).is_ok(), valid, "{name:?}");
}

/// A `{@link}` URL needs a scheme of ASCII letters and digits, `://`, and a character after it
/// (`/^[a-z][a-z0-9]*\:\/\/./i`).
#[hegel::test(test_cases = 200)]
fn link_urls_need_a_scheme_and_something_after_it(tc: TestCase) {
    let rule = Regex::new("^[A-Za-z][A-Za-z0-9]*://.").unwrap();
    let url = format!("{}://{}", string(&tc, "aZ1+.-", 4), string(&tc, "x/", 2));
    let invalid = logged(&format!("/** {{@link {url}}} */")).contains(&Id::LinkTagInvalidUrl);
    assert_eq!(invalid, !rule.is_match(&url), "{url:?}");
}

/// An HTML element name is a letter, then letters, digits and hyphens (`/^[a-z]+[a-z0-9\-]*$/i`).
#[hegel::test(test_cases = 200)]
fn html_names_are_a_letter_then_letters_digits_and_hyphens(tc: TestCase) {
    let rule = Regex::new("^[A-Za-z]+[A-Za-z0-9-]*$").unwrap();
    let name = format!("{}{}", string(&tc, "aZ1-._", 1).chars().next().unwrap_or('a'), string(&tc, "aZ1-._", 3));
    let explanation = "An HTML name must be an ASCII letter followed by zero or more letters, digits, or hyphens";
    let context = TSDocParser::default().parse_string(&format!("/** <{name}> */")).unwrap();
    let invalid = context.log.iter().any(|m| m.unformatted_text.ends_with(explanation));
    assert_eq!(invalid, !rule.is_match(&name), "{name:?}");
}

/// A declaration reference's package name is an optional `@scope/`, then a name, of ASCII letters,
/// digits, `-`, `_` and `.` (`/^(?:@[a-z0-9\-_\.]+\/)?[a-z0-9\-_\.]+$/i`).
#[hegel::test(test_cases = 200)]
fn package_names_are_npm_names(tc: TestCase) {
    let rule = Regex::new(r"^(?:@[A-Za-z0-9\-_.]+/)?[A-Za-z0-9\-_.]+$").unwrap();
    let scope = tc.draw(gs::optional(gs::text().alphabet("a1-_.@").max_size(3)));
    // A name that starts with `.` is read as an import path instead.
    let name = format!("{}{}", string(&tc, "a1-_@", 1).chars().next().unwrap_or('a'), string(&tc, "a1-_.@", 3));
    let package = scope.map_or(name.clone(), |scope| format!("@{scope}/{name}"));
    let invalid = logged(&format!("/** {{@link {package}#Foo}} */")).contains(&Id::ReferenceMalformedPackageName);
    assert_eq!(invalid, !rule.is_match(&package), "{package:?}");
}

/// An import path has no `//`, doesn't end in `/`, and starts with `/` only after a package name.
#[hegel::test(test_cases = 200)]
fn import_paths_are_relative_or_follow_a_package(tc: TestCase) {
    let packaged = tc.draw(gs::booleans());
    // Without a package, a path that starts with a letter would be read as one.
    let first = if packaged { "/" } else { tc.draw(gs::sampled_from(vec!["/", "."])) };
    let path = format!("{first}{}", string(&tc, "a/.", 4));
    let reference = format!("{}{path}#Foo", if packaged { "pkg" } else { "" });
    // A link that starts with `//` is a URL.
    if reference.starts_with("//") {
        return;
    }
    let invalid = logged(&format!("/** {{@link {reference}}} */")).contains(&Id::ReferenceMalformedImportPath);
    let wanted = path.contains("//") || path.ends_with('/') || (!packaged && path.starts_with('/'));
    assert_eq!(invalid, wanted, "{reference:?}");
}

/// A `@param` name is a word of ASCII letters, digits, `_` and `$`, not starting with a digit
/// (`/[^a-z0-9_$]/i`, `/^[0-9]/`).
#[hegel::test(test_cases = 200)]
fn parameter_names_are_identifiers(tc: TestCase) {
    let bad_character = Regex::new("[^A-Za-z0-9_$]").unwrap();
    let name = string(&tc, "aZ1_$.", 4);
    let invalid = name.is_empty() || bad_character.is_match(&name) || name.starts_with(|c: char| c.is_ascii_digit());
    let logged = logged(&format!("/**\n * @param {name} - x\n */"));
    assert_eq!(logged.contains(&Id::ParamTagWithInvalidName), invalid, "{name:?}");
}

/// A member identifier goes without quotes when it's an identifier and not a system selector.
#[hegel::test(test_cases = 200)]
fn member_identifiers_need_quotes_unless_they_are_identifiers(tc: TestCase) {
    let bad_character = Regex::new("[^A-Za-z0-9_$]").unwrap();
    let name = if tc.draw(gs::booleans()) {
        tc.draw(gs::sampled_from(SYSTEM_SELECTORS.to_vec())).to_owned()
    } else {
        string(&tc, "aZ1_$. ", 4)
    };
    let selector = SYSTEM_SELECTORS.contains(&name.as_str());
    let valid = !name.is_empty() && !bad_character.is_match(&name) && !name.starts_with(|c: char| c.is_ascii_digit());
    assert_eq!(DocMemberIdentifier::is_valid_identifier(&name), valid && !selector, "{name:?}");
    assert_eq!(DocMemberIdentifier::new(&name).has_quotes(), !valid || selector, "{name:?}");
}

/// A selector is an index (`/^[1-9][0-9]*$/`), a label (`/^[A-Z_][A-Z0-9_]+$/`) or one of TSDoc's
/// system selectors, with DocMemberSelector's message for each way it can fail.
#[hegel::test(test_cases = 300)]
fn selectors_are_indexes_labels_or_system_names(tc: TestCase) {
    let selector = if tc.draw(gs::booleans()) {
        tc.draw(gs::sampled_from(vec!["static", "class", "instance", "type", "nope", "0", "01", "10", "A", "AB", "_1"]))
            .to_owned()
    } else {
        string(&tc, "0a1Z_x$", 3)
    };
    let index = Regex::new("^[1-9][0-9]*$").unwrap();
    let label = Regex::new("^[A-Z_][A-Z0-9_]+$").unwrap();
    let read = DocMemberSelector::new(selector.clone());
    let (kind, message) = match selector.chars().next() {
        None => (SelectorKind::Error, Some("The selector cannot be an empty string".to_owned())),
        Some('0'..='9') if index.is_match(&selector) => (SelectorKind::Index, None),
        Some('0'..='9') => (
            SelectorKind::Error,
            Some("If the selector begins with a number, it must be a positive integer value".into()),
        ),
        Some('A'..='Z' | '_') if label.is_match(&selector) => (SelectorKind::Label, None),
        Some('A'..='Z' | '_') => (
            SelectorKind::Error,
            Some("A label selector must be comprised of upper case letters, numbers, and underscores and must not start with a number".into()),
        ),
        _ if SYSTEM_SELECTORS.contains(&selector.as_str()) => (SelectorKind::System, None),
        _ if selector.chars().all(|c| c.is_ascii_lowercase()) => (
            SelectorKind::Error,
            Some(format!("The selector \"{selector}\" is not a recognized TSDoc system selector name")),
        ),
        _ => (SelectorKind::Error, Some("Invalid syntax for selector".into())),
    };
    assert_eq!((read.selector_kind, read.error_message), (kind, message), "{selector:?}");
}

/// A location counts lines by `\n`, ignores `\r`, and counts columns in UTF-16 code units from 1,
/// as TSDoc does; outside the text it's line and column 0. A range's length, emptiness and text
/// agree.
#[hegel::test(test_cases = 200)]
fn locations_count_lines_and_utf16_columns(tc: TestCase) {
    let buffer = string(&tc, "a\n\r\té😀", 8);
    let mut boundaries: Vec<usize> = buffer.char_indices().map(|(at, _)| at).collect();
    boundaries.push(buffer.len());
    let index: isize = if tc.draw(gs::weighted_booleans(0.2)) {
        tc.draw(gs::sampled_from(vec![-1, buffer.len() as isize + 1]))
    } else {
        tc.draw(gs::sampled_from(boundaries.clone())) as isize
    };
    let location = TextRange::location(&buffer, index);
    if index < 0 || index as usize > buffer.len() {
        assert_eq!((location.line, location.column), (0, 0));
        return;
    }
    let before = &buffer[..index as usize];
    let line_start = before.rfind('\n').map_or(0, |at| at + 1);
    let column = before[line_start..].chars().filter(|&c| c != '\r').map(char::len_utf16).sum::<usize>() + 1;
    assert_eq!((location.line, location.column), (before.matches('\n').count() + 1, column), "{buffer:?} {index}");
    let end =
        tc.draw(gs::sampled_from(boundaries.iter().copied().filter(|&at| at >= index as usize).collect::<Vec<_>>()));
    let range = TextRange::new(index as usize, end);
    assert_eq!(range.text(&buffer), &buffer[index as usize..end]);
    assert_eq!((range.len(), range.is_empty()), (end - index as usize, end == index as usize));
}

/// Each token kind's name is TSDoc's enum name for it, which the Rust variant shares, and a
/// newline token's text is a line break.
#[hegel::test(test_cases = 100)]
fn token_kinds_have_tsdocs_names(tc: TestCase) {
    // Every kind, in a drawn order.
    let every = "a1_ \t!\"#$%&'()*+,-./:;<=>?@[\\]^`{|}~é";
    let line: String = tc.draw(gs::permutations(every.chars().collect::<Vec<_>>())).into_iter().collect();
    let lines = [TextRange::new(0, line.len())];
    for token in tokenizer::read_tokens(&line, &lines) {
        assert_eq!(token.kind.as_str(), format!("{:?}", token.kind));
        if token.kind == TokenKind::Newline {
            assert_eq!(token.text(&line), "\n");
        }
    }
}

/// A tsdoc.json sets up the same parser read from a file as from memory, and fails the same way.
#[hegel::test(test_cases = 100)]
fn a_tsdoc_json_reads_the_same_from_memory(tc: TestCase) {
    let mut keys = match config(&tc) {
        Some(Value::Object(keys)) => keys,
        _ => serde_json::Map::new(),
    };
    keys.insert("$schema".into(), json!(config_file::SCHEMA));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(config_file::FILE_NAME);
    fs::write(&path, Value::Object(keys.clone()).to_string()).unwrap();
    let from_file = config_file::load(&path).map(|(configuration, _)| configuration).map_err(|e| e.message_id);
    let file: TSDocConfigFile = serde_json::from_value(Value::Object(keys)).unwrap();
    let configured = file.configure_parser().map_err(|(id, _)| id);
    assert_eq!(configured, from_file);
    // Without its own checks first, it fails the same, unless only those checks catch it: the HTML
    // names, which it doesn't look at.
    let mut configuration = tsdocrs::TSDocConfiguration::new();
    configuration.clear(file.no_standard_tags.unwrap_or(false));
    let updated = file.update_parser(&mut configuration).map(|()| configuration).map_err(|(id, _)| id);
    if configured != Err(Id::ConfigFileSchemaError) {
        assert_eq!(updated, configured);
    }
}

/// With no tsconfig.json or package.json above a folder, there's no tsdoc.json for it, and every
/// folder up to the root was looked in.
#[hegel::test(test_cases = 10)]
fn no_project_folder_has_no_tsdoc_json(tc: TestCase) {
    let dir = tempfile::tempdir().unwrap();
    let below = tc.draw(gs::sampled_from(vec!["", "a", "a/b"]));
    let folder = dir.path().join(below);
    fs::create_dir_all(&folder).unwrap();
    let (found, looked) = config_file::find(&folder);
    // Some folder above the temporary one may be a project of its own; then this is about that one.
    if let Some(found) = found {
        assert!(!found.starts_with(dir.path()));
        return;
    }
    let folders = std::path::absolute(&folder).unwrap().ancestors().count();
    assert_eq!(looked.len(), folders * 2);
}
