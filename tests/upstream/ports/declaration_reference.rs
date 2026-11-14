//! tsdoc/src/beta/__tests__/DeclarationReference.test.ts.

use std::fmt::Debug;

use serde_json::Value;
use tsdocrs::beta::declaration_reference::{
    ComponentPath, ComponentReference, DeclarationReference, Error, GlobalSource, ModuleSource, Navigation, Source,
    SymbolReference,
};

use crate::harness::{Port, Test};

const FILE: &str = "tsdoc/src/beta/__tests__/DeclarationReference.test.ts";

/// The `.each` row's `key`, with `undefined` as None.
fn cell<'t>(test: &'t Test, key: &str) -> Result<Option<&'t Value>, String> {
    let value = test.each.as_ref().and_then(|row| row.get(key)).ok_or_else(|| format!("no `{key}` in the row"))?;
    Ok((value.get("$undefined") != Some(&Value::Bool(true))).then_some(value))
}

/// A string cell, or None for `undefined`.
fn string<'t>(test: &'t Test, key: &str) -> Result<Option<&'t str>, String> {
    cell(test, key)?.map(|value| value.as_str().ok_or_else(|| format!("`{key}` isn't a string"))).transpose()
}

fn required<'t>(test: &'t Test, key: &str) -> Result<&'t str, String> {
    string(test, key)?.ok_or_else(|| format!("`{key}` is undefined"))
}

fn boolean(test: &Test, key: &str) -> Result<bool, String> {
    cell(test, key)?.and_then(Value::as_bool).ok_or_else(|| format!("`{key}` isn't a boolean"))
}

fn expect<T: PartialEq + Debug>(what: &str, actual: T, expected: T) -> Result<(), String> {
    if actual == expected {
        return Ok(());
    }
    Err(format!("{what}: expected {expected:?}, got {actual:?}"))
}

fn parse(text: &str) -> Result<DeclarationReference, String> {
    DeclarationReference::parse(text).map_err(|error| error.message)
}

/// `expect(ref.symbol).toBeInstanceOf(SymbolReference)`.
fn symbol(reference: &DeclarationReference) -> Result<&SymbolReference, String> {
    reference.symbol().ok_or_else(|| "symbol: expected a SymbolReference, got undefined".into())
}

/// `expect(symbol.componentPath).toBeDefined()`.
fn component_path(symbol: &SymbolReference) -> Result<&ComponentPath, String> {
    symbol.component_path().ok_or_else(|| "componentPath: expected it defined".into())
}

/// The checks of the three `parse ... component text` tests.
fn parses_component(text: &str, component: &str) -> Result<(), String> {
    let reference = parse(text)?;
    expect("source", reference.source(), None)?;
    let path = component_path(symbol(&reference)?)?;
    expect("component", path.component().to_string().as_str(), component)
}

fn parse_component_text(_: &Test) -> Result<(), String> {
    parses_component("abc", "abc")
}

fn parse_component_text_string(_: &Test) -> Result<(), String> {
    parses_component("\"abc\"", "\"abc\"")
}

fn parse_bracketed_component_text(_: &Test) -> Result<(), String> {
    parses_component("[abc.[def]]", "[abc.[def]]")
}

fn parse_module_source(test: &Test) -> Result<(), String> {
    let reference = parse(required(test, "text")?)?;
    let Some(Source::Module(source)) = reference.source() else {
        return Err(format!("source: expected a ModuleSource, got {:?}", reference.source()));
    };
    expect("escapedPath", source.escaped_path(), required(test, "path")?)?;
    expect("navigation", reference.navigation().map(Navigation::as_str), string(test, "navigation")?)?;
    match string(test, "symbol")? {
        Some(expected) => {
            let path = component_path(symbol(&reference)?)?;
            expect("component", path.component().to_string().as_str(), expected)
        }
        None => expect("symbol", reference.symbol(), None),
    }
}

fn parse_global_source(test: &Test) -> Result<(), String> {
    let reference = parse(required(test, "text")?)?;
    expect("source", reference.source(), Some(&Source::Global(GlobalSource)))?;
    let path = component_path(symbol(&reference)?)?;
    expect("component", path.component().to_string().as_str(), required(test, "symbol")?)
}

fn parse_meaning(test: &Test) -> Result<(), String> {
    let reference = parse(required(test, "text")?)?;
    let meaning = symbol(&reference)?.meaning().map(|meaning| meaning.as_str());
    expect("meaning", meaning, Some(required(test, "meaning")?))
}

fn parse_complex(_: &Test) -> Result<(), String> {
    let reference = parse("foo/bar!N.C#z:member(1)")?;
    let Some(Source::Module(source)) = reference.source() else {
        return Err(format!("source: expected a ModuleSource, got {:?}", reference.source()));
    };
    expect("escapedPath", source.escaped_path(), "foo/bar")?;
    expect("navigation", reference.navigation(), Some(Navigation::Exports))?;
    let symbol = symbol(&reference)?;
    expect("meaning", symbol.meaning().map(|meaning| meaning.as_str()), Some("member"))?;
    expect("overloadIndex", symbol.overload_index(), Some(1.0))?;
    let Some(ComponentPath::Navigation(component1)) = symbol.component_path() else {
        return Err("component1: expected a ComponentNavigation".into());
    };
    expect("component1.navigation", component1.navigation(), Navigation::Members)?;
    expect("component1.component", component1.component().to_string().as_str(), "z")?;
    let ComponentPath::Navigation(component2) = component1.parent() else {
        return Err("component2: expected a ComponentNavigation".into());
    };
    expect("component2.navigation", component2.navigation(), Navigation::Exports)?;
    expect("component2.component", component2.component().to_string().as_str(), "C")?;
    let ComponentPath::Root(component3) = component2.parent() else {
        return Err("component3: expected a ComponentRoot".into());
    };
    expect("component3.component", component3.component().to_string().as_str(), "N")?;
    expect("toString", reference.to_string().as_str(), "foo/bar!N.C#z:member(1)")
}

fn parse_invalid_module_reference(_: &Test) -> Result<(), String> {
    match DeclarationReference::parse("@scope/foo") {
        Ok(reference) => Err(format!("expected a throw, got {reference}")),
        Err(_) => Ok(()),
    }
}

fn add_navigation_step(_: &Test) -> Result<(), String> {
    let component = ComponentReference::parse("[Symbol.iterator]").map_err(|error| error.message)?;
    let reference = DeclarationReference::empty()
        .add_navigation_step(Navigation::Members, component)
        .map_err(|error| error.message)?;
    let path = component_path(symbol(&reference)?)?;
    expect("component", path.component().to_string().as_str(), "[Symbol.iterator]")
}

fn is_well_formed_component_string(test: &Test) -> Result<(), String> {
    let text = required(test, "text")?;
    expect(
        "isWellFormedComponentString",
        DeclarationReference::is_well_formed_component_string(text),
        boolean(test, "expected")?,
    )
}

fn escape_component_string(test: &Test) -> Result<(), String> {
    let text = required(test, "text")?;
    expect(
        "escapeComponentString",
        DeclarationReference::escape_component_string(text).as_str(),
        required(test, "expected")?,
    )
}

/// The checks of the two unescape tests: a throw where `expected` is undefined.
fn unescapes(test: &Test, unescape: fn(&str) -> Result<String, Error>) -> Result<(), String> {
    let actual = unescape(required(test, "text")?).ok();
    expect("unescaped", actual.as_deref(), string(test, "expected")?)
}

fn unescape_component_string(test: &Test) -> Result<(), String> {
    unescapes(test, DeclarationReference::unescape_component_string)
}

fn is_well_formed_module_source_string(test: &Test) -> Result<(), String> {
    let text = required(test, "text")?;
    let actual = DeclarationReference::is_well_formed_module_source_string(text);
    expect("isWellFormedModuleSourceString", actual, boolean(test, "expected")?)
}

fn escape_module_source_string(test: &Test) -> Result<(), String> {
    let text = required(test, "text")?;
    let actual = DeclarationReference::escape_module_source_string(text);
    expect("escapeModuleSourceString", actual.as_str(), required(test, "expected")?)
}

fn unescape_module_source_string(test: &Test) -> Result<(), String> {
    unescapes(test, DeclarationReference::unescape_module_source_string)
}

fn package_parts(test: &Test) -> Result<(), String> {
    let source = ModuleSource::new(required(test, "text")?, true).map_err(|error| error.message)?;
    let message = |error: Error| error.message;
    expect("packageName", source.package_name().map_err(message)?.as_str(), required(test, "packageName")?)?;
    expect("scopeName", source.scope_name().map_err(message)?.as_str(), required(test, "scopeName")?)?;
    let unscoped_package_name = source.unscoped_package_name().map_err(message)?;
    expect("unscopedPackageName", unscoped_package_name.as_str(), required(test, "unscopedPackageName")?)?;
    expect("importPath", source.import_path().map_err(message)?.as_str(), required(test, "importPath")?)
}

/// `expect(source.path).toBe(text)`.
fn has_path(source: Result<ModuleSource, Error>, test: &Test) -> Result<(), String> {
    let path = source.and_then(|source| source.path()).map_err(|error| error.message)?;
    expect("path", path.as_str(), required(test, "text")?)
}

fn from_package(test: &Test) -> Result<(), String> {
    has_path(ModuleSource::from_package(required(test, "packageName")?, string(test, "importPath")?), test)
}

fn from_scoped_package(test: &Test) -> Result<(), String> {
    let source = ModuleSource::from_scoped_package(
        string(test, "scopeName")?,
        required(test, "unscopedPackageName")?,
        string(test, "importPath")?,
    );
    has_path(source, test)
}

inventory::submit! { Port { file: FILE, template: "parser parse component text", run: parse_component_text } }
inventory::submit! { Port { file: FILE, template: "parser parse component text string", run: parse_component_text_string } }
inventory::submit! { Port { file: FILE, template: "parser parse bracketed component text", run: parse_bracketed_component_text } }
inventory::submit! { Port { file: FILE, template: "parser parse module source $text", run: parse_module_source } }
inventory::submit! { Port { file: FILE, template: "parser parse global source $text", run: parse_global_source } }
inventory::submit! { Port { file: FILE, template: "parser parse meaning $meaning", run: parse_meaning } }
inventory::submit! { Port { file: FILE, template: "parser parse complex", run: parse_complex } }
inventory::submit! { Port { file: FILE, template: "parser parse invalid module reference", run: parse_invalid_module_reference } }
inventory::submit! { Port { file: FILE, template: "add navigation step", run: add_navigation_step } }
inventory::submit! {
    Port {
        file: FILE,
        template: "DeclarationReference isWellFormedComponentString($text)",
        run: is_well_formed_component_string,
    }
}
inventory::submit! {
    Port { file: FILE, template: "DeclarationReference escapeComponentString($text)", run: escape_component_string }
}
inventory::submit! {
    Port { file: FILE, template: "DeclarationReference unescapeComponentString($text)", run: unescape_component_string }
}
inventory::submit! {
    Port {
        file: FILE,
        template: "DeclarationReference isWellFormedModuleSourceString($text)",
        run: is_well_formed_module_source_string,
    }
}
inventory::submit! {
    Port {
        file: FILE,
        template: "DeclarationReference escapeModuleSourceString($text)",
        run: escape_module_source_string,
    }
}
inventory::submit! {
    Port {
        file: FILE,
        template: "DeclarationReference unescapeModuleSourceString($text)",
        run: unescape_module_source_string,
    }
}
inventory::submit! { Port { file: FILE, template: "ModuleSource package parts of $text", run: package_parts } }
inventory::submit! { Port { file: FILE, template: "ModuleSource fromPackage($packageName, $importPath)", run: from_package } }
inventory::submit! {
    Port {
        file: FILE,
        template: "ModuleSource fromScopedPackage($scopeName, $unscopedPackageName, $importPath)",
        run: from_scoped_package,
    }
}
