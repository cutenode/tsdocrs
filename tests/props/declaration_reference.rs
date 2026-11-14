//! Properties of `tsdocrs::beta::declaration_reference`: its escapes, its parser, and how it writes
//! references back out.

use hegel::{TestCase, generators as gs};
use tsdocrs::beta::declaration_reference::{
    Component, ComponentNavigation, ComponentPath, ComponentReference, ComponentRoot, ComponentString,
    DeclarationReference as Reference, GlobalSource, Meaning, ModuleSource, Navigation, Source, SymbolReference,
    SymbolReferenceOptions,
};

/// What the reference syntax gives meaning to, with a few ordinary and non-ASCII characters.
const SYNTAX: &str = "ab1!.#~:,\"{}()[]@\\/ux0 \t\n\r\u{e9}\u{1f600}";

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

fn symbol(tc: &TestCase, depth: usize, sources: Sources) -> SymbolReference {
    let mut path = ComponentPath::from(ComponentRoot::new(component(tc, depth, sources)));
    for _ in 0..tc.draw(gs::integers::<usize>().max_value(3)) {
        let navigation = tc.draw(gs::sampled_from(vec![Navigation::Exports, Navigation::Members, Navigation::Locals]));
        path = ComponentNavigation::new(path, navigation, component(tc, depth, sources)).into();
    }
    let meaning = tc.draw(gs::optional(gs::sampled_from(MEANINGS.to_vec())));
    let overload_index = tc.draw(gs::optional(gs::integers::<u32>())).map(f64::from);
    SymbolReference::new(Some(path), SymbolReferenceOptions { meaning, overload_index })
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
    let navigations = vec![Navigation::Exports, Navigation::Members, Navigation::Locals];
    let (first, second) = (tc.draw(gs::sampled_from(navigations.clone())), tc.draw(gs::sampled_from(navigations)));
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
