//! Properties of quoted strings in `tsdocrs::beta::declaration_reference`: the escapes the scanner
//! takes, as ECMAScript does, and the ones that unescape, as JSON does.

use hegel::{TestCase, generators as gs};
use tsdocrs::beta::declaration_reference::{DeclarationReference as Reference, ModuleSource};

/// A piece of a quoted string's body: its text, whether ECMAScript takes it in a string, and what
/// JSON reads it as, if JSON takes it. No piece starts with a digit or a hex digit that could
/// continue the escape before it, and each rejected escape ends where the scanner stops reading it.
const PIECES: [(&str, bool, Option<&str>); 36] = [
    ("a", true, Some("a")),
    ("!.#~:,{}()[]@ ", true, Some("!.#~:,{}()[]@ ")),
    ("\u{e9}\u{1f600}", true, Some("\u{e9}\u{1f600}")),
    // A control character is fine in an ECMAScript string, but not in JSON.
    ("\t", true, None),
    ("\\\"", true, Some("\"")),
    ("\\\\", true, Some("\\")),
    ("\\/", true, Some("/")),
    ("\\b", true, Some("\u{8}")),
    ("\\f", true, Some("\u{c}")),
    ("\\n", true, Some("\n")),
    ("\\r", true, Some("\r")),
    ("\\t", true, Some("\t")),
    ("\\u0041", true, Some("A")),
    ("\\u00e9", true, Some("\u{e9}")),
    ("\\ud83d\\ude00", true, Some("\u{1f600}")),
    // A lone surrogate, which a Rust string can't hold.
    ("\\ud800", true, Some("\u{fffd}")),
    ("\\a", true, None),
    ("\\'", true, None),
    ("\\!", true, None),
    ("\\0", true, None),
    ("\\x41", true, None),
    ("\\u{1f600}", true, None),
    ("\\u{0}", true, None),
    ("\\01", false, None),
    ("\\7", false, None),
    ("\\xg", false, None),
    ("\\x4g", false, None),
    ("\\ug", false, None),
    ("\\u004g", false, None),
    ("\\u{}", false, None),
    ("\\u{4g", false, None),
    ("\\u{4", false, None),
    ("\\\n", false, None),
    ("\\\r", false, None),
    ("\n", false, None),
    ("\r", false, None),
];

/// A quoted string is a module source, and parses as a reference, exactly when ECMAScript takes
/// every escape in it. It unescapes when JSON takes them all, and a string left open never parses.
#[hegel::test(test_cases = 400)]
fn quoted_strings_take_ecmascript_escapes(tc: TestCase) {
    let pieces = tc.draw(gs::vecs(gs::sampled_from(PIECES.as_slice())).max_size(4));
    let body: String = pieces.iter().map(|piece| piece.0).collect();
    let is_valid = pieces.iter().all(|piece| piece.1);
    let json: Option<String> = pieces.iter().map(|piece| piece.2).collect();
    let quoted = format!("\"{body}\"");
    assert_eq!(Reference::is_well_formed_module_source_string(&quoted), is_valid, "{quoted:?}");
    let parsed = Reference::parse(&quoted).map(|reference| reference.to_string());
    assert_eq!(parsed.ok(), is_valid.then(|| quoted.clone()), "{quoted:?}");
    let invalid = format!("Invalid Component '{quoted}'");
    let unescaped = Reference::unescape_component_string(&quoted).map_err(|error| error.to_string());
    assert_eq!(unescaped, json.clone().ok_or(invalid), "{quoted:?}");
    let source = ModuleSource::new(&quoted, true);
    assert_eq!(source.is_ok(), is_valid, "{quoted:?}");
    if let Ok(source) = source {
        assert_eq!(source.path().ok(), json, "{quoted:?}");
    }
    // Ends that leave the string open: nothing, a `\` with nothing after it, and a `\u{` that runs
    // into the end.
    let end = tc.draw(gs::sampled_from(["", "\\", "\\u{4"].as_slice()));
    let open = format!("\"{body}{end}");
    assert!(Reference::parse(&open).is_err(), "{open:?}");
}
