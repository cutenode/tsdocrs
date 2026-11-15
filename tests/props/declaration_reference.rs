//! Properties of `tsdocrs::beta::declaration_reference`: its escapes, its parser, and how it writes
//! references back out.

use hegel::{TestCase, generators as gs};
use tsdocrs::beta::declaration_reference::{
    Component, ComponentLike, ComponentNavigation, ComponentPath, ComponentReference, ComponentRoot, ComponentString,
    DeclarationReference as Reference, ErrorKind, GlobalSource, Meaning, ModuleSource, Navigation, Source,
    SymbolReference, SymbolReferenceOptions,
};

/// What the reference syntax gives meaning to, with a few ordinary and non-ASCII characters.
const SYNTAX: &str = "ab1!.#~:,\"{}()[]@\\/ux0 \t\n\r\u{e9}\u{1f600}";

const NAVIGATIONS: [Navigation; 3] = [Navigation::Exports, Navigation::Members, Navigation::Locals];

const MEANINGS: [Meaning; 14] = [
    Meaning::Class,
    Meaning::Interface,
    Meaning::TypeAlias,
    Meaning::Enum,
    Meaning::Namespace,
    Meaning::Function,
    Meaning::Variable,
    Meaning::Constructor,
    Meaning::Member,
    Meaning::Event,
    Meaning::CallSignature,
    Meaning::ConstructSignature,
    Meaning::IndexSignature,
    Meaning::ComplexType,
];

/// Any text, mostly made of the syntax's own characters.
fn text(tc: &TestCase) -> String {
    if tc.draw(gs::weighted_booleans(0.2)) {
        return tc.draw(gs::text().max_size(8));
    }
    tc.draw(gs::text().alphabet(SYNTAX).max_size(8))
}

/// Whether `text` would be read as a JSON string by the unescapes.
fn is_quoted(text: &str) -> bool {
    text.len() >= 2 && text.starts_with('"') && text.ends_with('"')
}

/// A component's escape is well-formed, unescapes back to the text, and leaves the text as it is
/// when it's well-formed already and doesn't start like a string or a bracketed reference.
#[hegel::test(test_cases = 300)]
fn component_escapes_round_trip(tc: TestCase) {
    let text = text(&tc);
    let escaped = Reference::escape_component_string(&text);
    assert!(Reference::is_well_formed_component_string(&escaped), "{escaped:?}");
    assert_eq!(Reference::unescape_component_string(&escaped).as_deref(), Ok(text.as_str()));
    let is_as_is =
        !text.is_empty() && !text.starts_with(['[', '"']) && Reference::is_well_formed_component_string(&text);
    assert_eq!(escaped == text, is_as_is, "{text:?} escapes to {escaped:?}");
    assert_eq!(ComponentString::new(&text, false).map(|component| component.text().to_owned()), Ok(escaped));
}

/// A module source's escape is well-formed, unescapes back to the text, and leaves the text as it
/// is when it's well-formed already and doesn't start like a string.
#[hegel::test(test_cases = 300)]
fn module_source_escapes_round_trip(tc: TestCase) {
    let text = text(&tc);
    let escaped = Reference::escape_module_source_string(&text);
    assert!(Reference::is_well_formed_module_source_string(&escaped), "{escaped:?}");
    assert_eq!(Reference::unescape_module_source_string(&escaped).as_deref(), Ok(text.as_str()));
    let is_as_is = !text.is_empty() && !text.starts_with('"') && Reference::is_well_formed_module_source_string(&text);
    assert_eq!(escaped == text, is_as_is, "{text:?} escapes to {escaped:?}");
    let source = ModuleSource::new(&text, false).unwrap();
    assert_eq!(source.escaped_path(), escaped);
    assert_eq!(source.path().as_deref(), Ok(text.as_str()));
}

/// Unescaping text that isn't quoted gives it back if it's well-formed, and fails with upstream's
/// message if not. The same goes for making a component or module source from it as escaped.
#[hegel::test(test_cases = 300)]
fn unescaping_unquoted_text_checks_it(tc: TestCase) {
    let text = text(&tc);
    let component = Reference::unescape_component_string(&text);
    let source = Reference::unescape_module_source_string(&text);
    if is_quoted(&text) {
        assert_eq!(component.is_ok(), source.is_ok(), "{text:?}");
        return;
    }
    let invalid = |what: &str| format!("Invalid {what} '{text}'");
    let expected =
        if Reference::is_well_formed_component_string(&text) { Ok(text.clone()) } else { Err(invalid("Component")) };
    assert_eq!(component.map_err(|error| error.message), expected);
    let expected =
        Reference::is_well_formed_module_source_string(&text).then(|| text.clone()).ok_or(invalid("Module source"));
    assert_eq!(source.map_err(|error| error.message), expected);
    let component = ComponentString::new(&text, true).map(|component| component.text().to_owned());
    assert_eq!(component.is_ok(), Reference::is_well_formed_component_string(&text));
    let source = ModuleSource::new(&text, true).map(|source| source.escaped_path().to_owned());
    assert_eq!(source.is_ok(), Reference::is_well_formed_module_source_string(&text));
}

/// Parsing never panics, and a reference that parses writes out text that parses to the same
/// reference again.
#[hegel::test(test_cases = 500)]
fn parsed_references_write_out_the_same(tc: TestCase) {
    let text = tc.draw(gs::text().alphabet(SYNTAX).max_size(16));
    let Ok(reference) = Reference::parse(&text) else {
        return;
    };
    let written = reference.to_string();
    let reparsed = Reference::parse(&written).unwrap_or_else(|error| panic!("{text:?} wrote {written:?}: {error}"));
    assert_eq!(reparsed.to_string(), written);
    assert_eq!(reparsed.source(), reference.source());
    assert_eq!(reparsed.navigation(), reference.navigation());
    assert_eq!(reparsed.symbol(), reference.symbol());
    if !text.starts_with('[') {
        let component = Reference::parse_component(&text).map(|component| component.to_string());
        assert_eq!(component.is_ok(), Reference::is_well_formed_component_string(&text));
    }
}

/// Which references can have a source in a generated reference. A reference without a source is
/// read by scanning for a `!` that would start one, so a source after it, even in a later
/// bracketed component, would be read as its own: either all of them have one, or only the
/// outermost may.
#[derive(Clone, Copy, PartialEq)]
enum Sources {
    All,
    Outermost,
    None,
}

fn reference(tc: &TestCase, depth: usize, sources: Sources) -> Reference {
    let source: Option<Source> = match sources {
        Sources::None => None,
        Sources::All if tc.draw(gs::booleans()) => Some(GlobalSource.into()),
        Sources::Outermost if tc.draw(gs::booleans()) => None,
        Sources::Outermost if tc.draw(gs::booleans()) => Some(GlobalSource.into()),
        _ => Some(ModuleSource::new(&text(tc), false).unwrap().into()),
    };
    let nested = if sources == Sources::All { Sources::All } else { Sources::None };
    let symbol = tc.draw(gs::booleans()).then(|| symbol(tc, depth, nested));
    let is_module = matches!(source, Some(Source::Module(_)));
    let navigation = (is_module && symbol.is_some() && tc.draw(gs::booleans())).then_some(Navigation::Locals);
    Reference::new(source, navigation, symbol)
}

fn component(tc: &TestCase, depth: usize, sources: Sources) -> Component {
    if depth > 0 && tc.draw(gs::weighted_booleans(0.25)) {
        return Component::Reference(ComponentReference::new(reference(tc, depth - 1, sources)));
    }
    Component::String(ComponentString::new(&text(tc), false).unwrap())
}

fn navigation(tc: &TestCase) -> Navigation {
    tc.draw(gs::sampled_from(NAVIGATIONS.as_slice()))
}

fn meaning(tc: &TestCase) -> Option<Meaning> {
    tc.draw(gs::optional(gs::sampled_from(MEANINGS.as_slice())))
}

fn overload_index(tc: &TestCase) -> Option<f64> {
    tc.draw(gs::optional(gs::integers::<u32>())).map(f64::from)
}

fn path(tc: &TestCase, depth: usize, sources: Sources) -> ComponentPath {
    let mut path = ComponentPath::from(ComponentRoot::new(component(tc, depth, sources)));
    for _ in 0..tc.draw(gs::integers::<usize>().max_value(3)) {
        path = ComponentNavigation::new(path, navigation(tc), component(tc, depth, sources)).into();
    }
    path
}

fn symbol(tc: &TestCase, depth: usize, sources: Sources) -> SymbolReference {
    let options = SymbolReferenceOptions { meaning: meaning(tc), overload_index: overload_index(tc) };
    SymbolReference::new(Some(path(tc, depth, sources)), options)
}

/// A reference written out parses back to the same reference.
#[hegel::test(test_cases = 300)]
fn references_round_trip(tc: TestCase) {
    let sources = if tc.draw(gs::booleans()) { Sources::All } else { Sources::Outermost };
    let reference = reference(&tc, 2, sources);
    let written = reference.to_string();
    assert_eq!(Reference::parse(&written), Ok(reference), "{written:?}");
    let bracketed = format!("[{written}]");
    let component = ComponentReference::parse(&bracketed).map(|component| component.to_string());
    assert_eq!(component.is_ok(), !written.is_empty(), "{bracketed:?}");
}

/// A package name part: an NPM name or scope.
fn package_part(tc: &TestCase) -> String {
    tc.draw(gs::text().alphabet("abz09-_.").min_size(1).max_size(5))
}

/// A module made from a package and a path in it has that package's parts, and its path is the
/// package's name and that path, whether the scope comes with its `@` or without.
#[hegel::test(test_cases = 200)]
fn package_sources_keep_their_parts(tc: TestCase) {
    let scope = tc.draw(gs::booleans()).then(|| package_part(&tc));
    let name = package_part(&tc);
    let import_path =
        tc.draw(gs::booleans()).then(|| tc.draw(gs::from_regex("[a-z]{1,3}(/[a-z0-9]{1,3}){0,2}").fullmatch(true)));
    let package_name = scope.as_ref().map_or(name.clone(), |scope| format!("@{scope}/{name}"));
    let path = import_path.as_ref().map_or(package_name.clone(), |import_path| format!("{package_name}/{import_path}"));
    let at_scope = scope.as_ref().map(|scope| format!("@{scope}"));
    let sources = [
        ModuleSource::from_package(&package_name, import_path.as_deref()).unwrap(),
        ModuleSource::from_scoped_package(scope.as_deref(), &name, import_path.as_deref()).unwrap(),
        ModuleSource::from_scoped_package(at_scope.as_deref(), &name, import_path.as_deref()).unwrap(),
        ModuleSource::new(&path, true).unwrap(),
    ];
    for source in sources {
        assert_eq!(source.path().as_deref(), Ok(path.as_str()));
        assert_eq!(source.package_name().as_deref(), Ok(package_name.as_str()));
        assert_eq!(source.scope_name().as_deref(), Ok(at_scope.as_deref().unwrap_or_default()));
        assert_eq!(source.unscoped_package_name().as_deref(), Ok(name.as_str()));
        assert_eq!(source.import_path().as_deref(), Ok(import_path.as_deref().unwrap_or_default()));
    }
    let reference = Reference::package(&package_name, import_path.as_deref()).unwrap();
    assert_eq!(reference.to_string(), format!("{path}!"));
}

/// A navigation step from a reference without a symbol starts one, through exports for a member
/// step; a step from a symbol extends its path and drops its meaning and overload index.
#[hegel::test(test_cases = 100)]
fn navigation_steps_extend_the_path(tc: TestCase) {
    let (first, second) = (navigation(&tc), navigation(&tc));
    let (a, b) = (text(&tc), text(&tc));
    let module = Reference::module("m", true).unwrap();
    let reference = module.add_navigation_step(first, a.as_str()).unwrap().with_meaning(Some(Meaning::Class));
    let expected = if first == Navigation::Locals { Navigation::Locals } else { Navigation::Exports };
    assert_eq!(reference.navigation(), Some(expected));
    let reference = reference.add_navigation_step(second, b.as_str()).unwrap();
    let (a, b) = (Reference::escape_component_string(&a), Reference::escape_component_string(&b));
    let tilde = if expected == Navigation::Locals { "~" } else { "" };
    assert_eq!(reference.to_string(), format!("m!{tilde}{a}{second}{b}"));
}

/// Each `with_` of a reference sets its part and leaves the others, and giving it the part it has
/// gives back the same reference. Its navigation follows upstream's rule: none without both a
/// source and a symbol, locals in the global scope, and exports in a module unless set to locals.
#[hegel::test(test_cases = 200)]
fn reference_builders_set_one_part(tc: TestCase) {
    let (reference, other) = (reference(&tc, 1, Sources::All), reference(&tc, 1, Sources::All));
    let path_of = |reference: &Reference| reference.symbol().and_then(SymbolReference::component_path).cloned();
    let meaning_of = |reference: &Reference| reference.symbol().and_then(SymbolReference::meaning);
    let overload_of = |reference: &Reference| reference.symbol().and_then(SymbolReference::overload_index);

    assert_eq!(reference.is_empty(), reference.to_string().is_empty(), "{reference}");
    assert_eq!(Reference::from(Some(reference.clone())), reference);
    assert_eq!(reference.clone().with_source(reference.source().cloned()), reference);
    assert_eq!(reference.clone().with_symbol(reference.symbol().cloned()), reference);
    assert_eq!(reference.clone().with_meaning(meaning_of(&reference)), reference);
    assert_eq!(reference.clone().with_overload_index(overload_of(&reference)), reference);
    let same_navigation = reference.clone().with_navigation(reference.navigation());
    assert_eq!(same_navigation.to_string(), reference.to_string());
    let rebuilt = Reference::from(None)
        .with_source(reference.source().cloned())
        .with_navigation(reference.navigation())
        .with_symbol(reference.symbol().cloned());
    assert_eq!(rebuilt.to_string(), reference.to_string());
    assert!(Reference::empty().is_empty());
    let global = Reference::global().with_symbol(reference.symbol().cloned());
    let symbol = reference.symbol().map(ToString::to_string).unwrap_or_default();
    assert_eq!(global.to_string(), format!("!{symbol}"));
    assert_eq!(global.navigation(), reference.symbol().and(Some(Navigation::Locals)));

    let with_source = reference.clone().with_source(other.source().cloned());
    assert_eq!((with_source.source(), with_source.symbol()), (other.source(), reference.symbol()));
    let with_symbol = reference.clone().with_symbol(other.symbol().cloned());
    assert_eq!((with_symbol.source(), with_symbol.symbol()), (reference.source(), other.symbol()));

    let navigation = tc.draw(gs::optional(gs::sampled_from(NAVIGATIONS.as_slice())));
    let with_navigation = reference.clone().with_navigation(navigation);
    let expected = match (reference.source(), reference.symbol()) {
        (None, _) | (_, None) => None,
        (Some(Source::Global(_)), _) => Some(Navigation::Locals),
        (Some(Source::Module(_)), _) => Some(navigation.unwrap_or(Navigation::Exports)),
    };
    assert_eq!(with_navigation.navigation(), expected);
    assert_eq!((with_navigation.source(), with_navigation.symbol()), (reference.source(), reference.symbol()));

    let path = path(&tc, 1, Sources::All);
    let with_path = reference.clone().with_component_path(path.clone());
    assert_eq!(path_of(&with_path), Some(path));
    assert_eq!((meaning_of(&with_path), overload_of(&with_path)), (meaning_of(&reference), overload_of(&reference)));

    let meaning = meaning(&tc);
    let with_meaning = reference.clone().with_meaning(meaning);
    assert_eq!(meaning_of(&with_meaning), meaning);
    assert_eq!((path_of(&with_meaning), overload_of(&with_meaning)), (path_of(&reference), overload_of(&reference)));
    assert_eq!(with_meaning.source(), reference.source());

    let overload_index = overload_index(&tc);
    let with_overload = reference.clone().with_overload_index(overload_index);
    assert_eq!(overload_of(&with_overload), overload_index);
    assert_eq!((path_of(&with_overload), meaning_of(&with_overload)), (path_of(&reference), meaning_of(&reference)));
    assert_eq!(with_overload.source(), reference.source());
}

/// Each `with_` of a symbol sets its part and leaves the others, and giving it the part it has gives
/// back the same symbol. A symbol without a path can't take a navigation step, from itself or from
/// the reference it's in.
#[hegel::test(test_cases = 200)]
fn symbol_builders_set_one_part(tc: TestCase) {
    let (symbol, other) = (symbol(&tc, 1, Sources::All), symbol(&tc, 1, Sources::All));
    let parts =
        |symbol: &SymbolReference| (symbol.component_path().cloned(), symbol.meaning(), symbol.overload_index());
    let (path, meaning, overload_index) = parts(&symbol);
    let (other_path, other_meaning, other_overload_index) = parts(&other);

    assert_eq!(symbol.clone().with_component_path(path.clone()), symbol);
    assert_eq!(symbol.clone().with_meaning(meaning), symbol);
    assert_eq!(symbol.clone().with_overload_index(overload_index), symbol);
    let with_path = symbol.clone().with_component_path(other_path.clone());
    assert_eq!(parts(&with_path), (other_path, meaning, overload_index));
    let with_meaning = symbol.clone().with_meaning(other_meaning);
    assert_eq!(parts(&with_meaning), (path.clone(), other_meaning, overload_index));
    let with_overload_index = symbol.clone().with_overload_index(other_overload_index);
    assert_eq!(parts(&with_overload_index), (path, meaning, other_overload_index));

    assert_eq!(parts(&SymbolReference::empty()), (None, None, None));
    assert_eq!(SymbolReference::empty().to_string(), "");
    let pathless = symbol.with_component_path(None);
    let message = "Cannot add a navigation step to an empty symbol reference.";
    let step = navigation(&tc);
    let error = pathless.clone().add_navigation_step(step, "a").unwrap_err();
    assert_eq!((error.kind, error.to_string().as_str()), (ErrorKind::Other, message));
    let error = Reference::new(None, None, Some(pathless)).add_navigation_step(step, "a").unwrap_err();
    assert_eq!((error.kind, error.to_string().as_str()), (ErrorKind::Other, message));
}

/// Each `with_` of a path's root or step sets its part and leaves the others, a path's component
/// is its last one, and a path written out is its parent, navigation and component. Anything a
/// component can be made from becomes the component it stands for, and a component written out
/// parses back to itself unless it brackets the empty reference.
#[hegel::test(test_cases = 200)]
fn component_builders_set_one_part(tc: TestCase) {
    let (parent, other_parent) = (path(&tc, 1, Sources::All), path(&tc, 1, Sources::All));
    let (navigation, other_navigation) = (navigation(&tc), navigation(&tc));
    let (component, other) = (component(&tc, 1, Sources::All), component(&tc, 1, Sources::All));
    let parts = |step: &ComponentNavigation| (step.parent().clone(), step.navigation(), step.component().clone());

    let step = ComponentNavigation::new(parent.clone(), navigation, component.clone());
    assert_eq!(parts(&step), (parent.clone(), navigation, component.clone()));
    assert_eq!(step.clone().with_parent(parent.clone()), step);
    assert_eq!(step.clone().with_navigation(navigation), step);
    assert_eq!(step.clone().with_component(component.clone()), step);
    let with_parent = step.clone().with_parent(other_parent.clone());
    assert_eq!(parts(&with_parent), (other_parent, navigation, component.clone()));
    let with_navigation = step.clone().with_navigation(other_navigation);
    assert_eq!(parts(&with_navigation), (parent.clone(), other_navigation, component.clone()));
    let with_component = step.clone().with_component(other.clone());
    assert_eq!(parts(&with_component), (parent.clone(), navigation, other.clone()));
    let path = ComponentPath::from(step);
    assert_eq!(path.component(), &component);
    assert_eq!(path.to_string(), format!("{parent}{navigation}{component}"));
    assert_eq!(parent.add_navigation_step(navigation, component.clone()), path);

    let root = ComponentRoot::new(component.clone());
    assert_eq!(root.component(), &component);
    assert_eq!(root.clone().with_component(component.clone()), root);
    assert_eq!(ComponentPath::from(root.clone()).component(), &component);
    assert_eq!(root.clone().with_component(other.clone()).component(), &other);

    let text = text(&tc);
    let (reference, other_reference) = (reference(&tc, 0, Sources::All), reference(&tc, 0, Sources::All));
    let bracketed = ComponentReference::new(reference.clone());
    assert_eq!(bracketed.reference(), &reference);
    assert_eq!(bracketed.clone().with_reference(other_reference.clone()).reference(), &other_reference);
    let escaped = ComponentString::new(&text, false).unwrap();
    let likes: [(ComponentLike, Component); 6] = [
        (text.as_str().into(), Component::String(escaped.clone())),
        (text.clone().into(), Component::String(escaped.clone())),
        (escaped.clone().into(), Component::String(escaped)),
        (reference.clone().into(), Component::Reference(bracketed.clone())),
        (bracketed.clone().into(), Component::Reference(bracketed)),
        (component.clone().into(), component.clone()),
    ];
    for (like, expected) in likes {
        assert_eq!(root.clone().with_component(like).component(), &expected);
    }

    let written = component.to_string();
    let brackets_empty = matches!(&component, Component::Reference(bracketed) if bracketed.reference().is_empty());
    assert_eq!(Reference::parse_component(&written).ok(), (!brackets_empty).then_some(component), "{written:?}");
}

/// The tokens that can come after a symbol whose overload index is closed, and how upstream's
/// messages name them.
const AFTER_SYMBOL: [(&str, &str); 15] = [
    ("", "<eof>"),
    ("{", "{"),
    ("}", "}"),
    ("(", "("),
    (")", ")"),
    ("[", "["),
    ("!", "!"),
    (".", "."),
    ("#", "#"),
    ("~", "~"),
    (":", ":"),
    (",", ","),
    ("@", "@"),
    ("\"s\"", "<string>"),
    ("t", "<text>"),
];

/// A bracketed component that isn't closed after its symbol fails with upstream's message, naming
/// the token that came instead of the `]`.
#[hegel::test(test_cases = 100)]
fn unclosed_brackets_name_the_token_instead(tc: TestCase) {
    let meaning = tc.draw(gs::sampled_from(MEANINGS.as_slice()));
    let overload_index = tc.draw(gs::integers::<u32>());
    let symbol =
        symbol(&tc, 1, Sources::All).with_meaning(Some(meaning)).with_overload_index(Some(overload_index.into()));
    let (after, name) = tc.draw(gs::sampled_from(AFTER_SYMBOL.as_slice()));
    // Both references start with the global scope, so the `!`s after them aren't read as the end
    // of a module source.
    let text = format!("![!{symbol}{after}");
    let error = Reference::parse(&text).unwrap_err();
    let expected = format!("Invalid DeclarationReference '{text}':\n  Expected token ']', received '{name}' instead.");
    assert_eq!(error.kind, ErrorKind::Syntax);
    assert!(error.to_string().starts_with(&expected), "{error}");
}

/// Numbers an overload index is often written wrong for: the ends of the plain range, zeros, and
/// the ones JavaScript names.
const SPECIAL_NUMBERS: [f64; 11] =
    [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -0.0, 1e21, 9.99e20, 1e-6, 9.99e-7, -1e21, 5e-324];

/// An overload index is written as JavaScript writes the number: digits that read back as it, in
/// exponent form, with a sign, from 1e21 and below 1e-6, and `NaN`, `Infinity`, and `0` for `-0`.
#[hegel::test(test_cases = 300)]
fn overload_indexes_are_written_as_javascript_numbers(tc: TestCase) {
    let number = if tc.draw(gs::weighted_booleans(0.3)) {
        tc.draw(gs::sampled_from(SPECIAL_NUMBERS.as_slice()))
    } else {
        tc.draw(gs::floats::<f64>())
    };
    let meaning = meaning(&tc);
    let options = SymbolReferenceOptions { meaning, overload_index: Some(number) };
    let written = SymbolReference::new(None, options).to_string();
    let digits = match meaning {
        Some(meaning) => written.strip_prefix(&format!(":{meaning}(")).and_then(|rest| rest.strip_suffix(')')),
        None => written.strip_prefix(':'),
    };
    let digits = digits.unwrap_or_else(|| panic!("{written:?}"));
    match number {
        _ if number.is_nan() => assert_eq!(digits, "NaN"),
        f64::INFINITY => assert_eq!(digits, "Infinity"),
        f64::NEG_INFINITY => assert_eq!(digits, "-Infinity"),
        _ => {
            assert_eq!(digits.parse::<f64>(), Ok(number), "{digits}");
            assert_ne!(digits, "-0");
            let is_exponent = number != 0.0 && !(1e-6..1e21).contains(&number.abs());
            let exponent = digits.split_once('e').map(|(_, exponent)| exponent);
            assert_eq!(exponent.is_some(), is_exponent, "{digits}");
            assert!(exponent.is_none_or(|exponent| exponent.starts_with(['+', '-'])), "{digits}");
        }
    }
}

/// A package name part with a character NPM doesn't allow in it.
fn bad_package_part(tc: &TestCase) -> String {
    let mut part = package_part(tc);
    let at = tc.draw(gs::integers::<usize>().max_value(part.len()));
    part.insert(at, tc.draw(gs::sampled_from([' ', '$', '&', '*', '+', '=', '!'].as_slice())));
    part
}

/// A package can't be a module source when its name isn't a package's, NPM rejects its name, or
/// its import path is relative or absolute, and each fails with upstream's message. An import path
/// that only starts with dots is fine.
#[hegel::test(test_cases = 200)]
fn package_sources_reject_bad_parts(tc: TestCase) {
    let (scope, name) = (package_part(&tc), package_part(&tc));
    let import_path = tc.draw(gs::from_regex("[a-z0-9]{1,3}(/[a-z0-9]{1,3}){0,1}").fullmatch(true));

    let not_a_package = [String::new(), "/".into(), format!("@{scope}/"), format!("{name}/"), format!("/{name}")];
    let not_a_package = tc.draw(gs::sampled_from(not_a_package.as_slice()));
    let error = Reference::package(&not_a_package, None).unwrap_err();
    assert_eq!((error.kind, error.to_string().as_str()), (ErrorKind::Other, "Parsed package must be provided."));

    let bad_name = bad_package_part(&tc);
    let message = |package_name: &str| {
        format!("Invalid NPM package name: The package name \"{package_name}\" is not a valid package name")
    };
    let error = ModuleSource::from_package(&bad_name, Some(&import_path)).unwrap_err();
    assert_eq!((error.kind, error.to_string()), (ErrorKind::Syntax, message(&bad_name)));
    let error = ModuleSource::from_scoped_package(Some(&scope), &bad_name, None).unwrap_err();
    assert_eq!((error.kind, error.to_string()), (ErrorKind::Syntax, message(&format!("@{scope}/{bad_name}"))));

    let prefix = tc.draw(gs::sampled_from(["./", "../", "/", "\\", ".\\", "..\\"].as_slice()));
    let bad_path = match tc.draw(gs::sampled_from([".", "..", ""].as_slice())) {
        "" => format!("{prefix}{import_path}"),
        dots => dots.to_owned(),
    };
    let error = ModuleSource::from_package(&name, Some(&bad_path)).unwrap_err();
    // Upstream's message has no closing quote.
    assert_eq!((error.kind, error.to_string()), (ErrorKind::Syntax, format!("Invalid import path '{bad_path}")));
    let dotted = format!("{}{import_path}", tc.draw(gs::sampled_from([".", ".."].as_slice())));
    let source = ModuleSource::from_package(&name, Some(&dotted)).unwrap();
    assert_eq!(source.import_path().as_deref(), Ok(dotted.as_str()));
}
