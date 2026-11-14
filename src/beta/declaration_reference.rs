//! `@microsoft/tsdoc`'s beta DeclarationReference: a reference to a declaration, like
//! `@scope/pkg!Class#member:function(1)`, and the parser and escapes for its syntax.

mod component;
mod module_source;
mod parser;
mod scanner;
mod symbol;
#[cfg(test)]
mod tests;

use std::fmt;

pub use component::{
    Component, ComponentLike, ComponentNavigation, ComponentPath, ComponentReference, ComponentRoot, ComponentString,
};
pub use module_source::ModuleSource;
pub use symbol::{Meaning, SymbolReference, SymbolReferenceOptions};

use crate::parser::string_checks::json_stringify;

/// What upstream throws, word for word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    /// Which JavaScript error upstream throws.
    pub kind: ErrorKind,
    /// The error's message.
    pub message: String,
}

/// The JavaScript error class upstream throws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// A `SyntaxError`: text that isn't valid in a reference.
    Syntax,
    /// A plain `Error`: an operation that can't be done on what it's given.
    Other,
}

impl Error {
    fn syntax(message: String) -> Self {
        Self { kind: ErrorKind::Syntax, message }
    }

    fn other(message: &str) -> Self {
        Self { kind: ErrorKind::Other, message: message.into() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

/// A reference to a declaration: where it's from, and the symbol in it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeclarationReference {
    source: Option<Source>,
    navigation: Option<Navigation>,
    symbol: Option<SymbolReference>,
}

impl DeclarationReference {
    /// A reference to `symbol` in `source`, looked up through `navigation` (exports or locals).
    pub fn new(source: Option<Source>, navigation: Option<Navigation>, symbol: Option<SymbolReference>) -> Self {
        Self { source, navigation, symbol }
    }

    /// The module or global scope the symbol is in.
    pub fn source(&self) -> Option<&Source> {
        self.source.as_ref()
    }

    /// How the symbol is looked up in the source: locals in the global scope, and exports in a
    /// module unless it says locals. None without both a source and a symbol.
    pub fn navigation(&self) -> Option<Navigation> {
        match (&self.source, &self.symbol) {
            (None, _) | (_, None) => None,
            (Some(Source::Global(_)), _) => Some(Navigation::Locals),
            (Some(Source::Module(_)), _) => Some(self.navigation.unwrap_or(Navigation::Exports)),
        }
    }

    /// The symbol it refers to.
    pub fn symbol(&self) -> Option<&SymbolReference> {
        self.symbol.as_ref()
    }

    /// Whether it has neither a source nor a symbol.
    pub fn is_empty(&self) -> bool {
        self.source.is_none() && self.symbol.is_none()
    }

    /// Parses a declaration reference, like `pkg!Class#member`.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut parser = parser::Parser::new(text);
        let reference = parser.parse_declaration_reference();
        if !parser.errors.is_empty() {
            let errors = parser.errors.join("\n  ");
            return Err(Error::syntax(format!("Invalid DeclarationReference '{text}':\n  {errors}")));
        }
        if !parser.eof() {
            return Err(Error::syntax(format!("Invalid DeclarationReference '{text}'")));
        }
        Ok(reference)
    }

    /// Parses a navigation component: a bracketed reference, or an escaped string.
    pub fn parse_component(text: &str) -> Result<Component, Error> {
        if text.starts_with('[') {
            return ComponentReference::parse(text).map(Component::Reference);
        }
        ComponentString::new(text, true).map(Component::String)
    }

    /// Whether `text` can be a navigation component as it is: a quoted string, or text without
    /// any of `{}()[]!.#~:,@"`.
    pub fn is_well_formed_component_string(text: &str) -> bool {
        scanner::is_well_formed_component_string(text)
    }

    /// `text` as a navigation component: in quotes if it has any of `!.#~:,"{}()@` or starts with
    /// `[`.
    pub fn escape_component_string(text: &str) -> String {
        if text.is_empty() {
            return "\"\"".into();
        }
        if text.starts_with(['[', '"']) || !Self::is_well_formed_component_string(text) {
            return json_stringify(text);
        }
        text.into()
    }

    /// The text a navigation component stands for, without its quotes and escapes.
    pub fn unescape_component_string(text: &str) -> Result<String, Error> {
        unescape(text, "Component", Self::is_well_formed_component_string)
    }

    /// Whether `text` can be a module source as it is, without its trailing `!`.
    pub fn is_well_formed_module_source_string(text: &str) -> bool {
        scanner::is_well_formed_module_source_string(text)
    }

    /// `text` as a module source: in quotes if it has any of `!"`.
    pub fn escape_module_source_string(text: &str) -> String {
        if text.is_empty() {
            return "\"\"".into();
        }
        if text.starts_with('"') || !Self::is_well_formed_module_source_string(text) {
            return json_stringify(text);
        }
        text.into()
    }

    /// The path a module source stands for, without its quotes and escapes or a trailing `!`.
    pub fn unescape_module_source_string(text: &str) -> Result<String, Error> {
        unescape(text, "Module source", Self::is_well_formed_module_source_string)
    }

    /// The reference to nothing.
    pub fn empty() -> Self {
        Self::default()
    }

    /// A reference to a package, or a path in it.
    pub fn package(package_name: &str, import_path: Option<&str>) -> Result<Self, Error> {
        Ok(Self::new(Some(ModuleSource::from_package(package_name, import_path)?.into()), None, None))
    }

    /// A reference to the module at `path`, escaped already if `user_escaped` (upstream's default)
    /// or escaped here if not.
    pub fn module(path: &str, user_escaped: bool) -> Result<Self, Error> {
        Ok(Self::new(Some(ModuleSource::new(path, user_escaped)?.into()), None, None))
    }

    /// A reference to the global scope.
    pub fn global() -> Self {
        Self::new(Some(GlobalSource.into()), None, None)
    }

    /// The same reference with another source.
    pub fn with_source(self, source: Option<Source>) -> Self {
        Self { source, ..self }
    }

    /// The same reference with another navigation into its source.
    pub fn with_navigation(self, navigation: Option<Navigation>) -> Self {
        Self { navigation, ..self }
    }

    /// The same reference with another symbol.
    pub fn with_symbol(self, symbol: Option<SymbolReference>) -> Self {
        Self { symbol, ..self }
    }

    /// The same reference with another component path in its symbol.
    pub fn with_component_path(mut self, component_path: ComponentPath) -> Self {
        let symbol = match self.symbol.take() {
            Some(symbol) => symbol.with_component_path(Some(component_path)),
            None => SymbolReference::new(Some(component_path), SymbolReferenceOptions::default()),
        };
        self.with_symbol(Some(symbol))
    }

    /// The same reference with another meaning for its symbol.
    pub fn with_meaning(mut self, meaning: Option<Meaning>) -> Self {
        let symbol = match self.symbol.take() {
            Some(symbol) => symbol,
            None if meaning.is_none() => return self,
            None => SymbolReference::empty(),
        };
        self.with_symbol(Some(symbol.with_meaning(meaning)))
    }

    /// The same reference with another overload index for its symbol.
    pub fn with_overload_index(mut self, overload_index: Option<f64>) -> Self {
        let symbol = match self.symbol.take() {
            Some(symbol) => symbol,
            None if overload_index.is_none() => return self,
            None => SymbolReference::empty(),
        };
        self.with_symbol(Some(symbol.with_overload_index(overload_index)))
    }

    /// The reference to `component` through `navigation` from this one's symbol, or from its
    /// source when it has no symbol.
    pub fn add_navigation_step(
        self,
        navigation: Navigation,
        component: impl Into<ComponentLike>,
    ) -> Result<Self, Error> {
        if let Some(symbol) = self.symbol {
            let symbol = symbol.add_navigation_step(navigation, component)?;
            return Ok(Self { symbol: Some(symbol), ..self });
        }
        // A source has no members of its own, so a member step from it reads its exports.
        let navigation = if navigation == Navigation::Members { Navigation::Exports } else { navigation };
        let root = ComponentRoot::new(component::from_like(component));
        let symbol = SymbolReference::new(Some(root.into()), SymbolReferenceOptions::default());
        Ok(Self::new(self.source, Some(navigation), Some(symbol)))
    }
}

impl From<Option<DeclarationReference>> for DeclarationReference {
    /// `base`, or the empty reference.
    fn from(base: Option<DeclarationReference>) -> Self {
        base.unwrap_or_default()
    }
}

impl fmt::Display for DeclarationReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(source) = &self.source {
            write!(f, "{source}")?;
        }
        // Exports are a module's default, and the global scope only has locals, so only a
        // module's locals are written.
        let is_module = matches!(self.source, Some(Source::Module(_)));
        if is_module && self.symbol.is_some() && self.navigation() == Some(Navigation::Locals) {
            f.write_str("~")?;
        }
        if let Some(symbol) = &self.symbol {
            write!(f, "{symbol}")?;
        }
        Ok(())
    }
}

/// The symbol table the next component is looked up in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Navigation {
    /// The exports: `.`.
    Exports,
    /// The instance members: `#`.
    Members,
    /// The locals: `~`.
    Locals,
}

impl Navigation {
    /// Its punctuator.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exports => ".",
            Self::Members => "#",
            Self::Locals => "~",
        }
    }
}

impl fmt::Display for Navigation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where a referenced symbol is: a module, or the global scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// A module, like `pkg/path!`.
    Module(ModuleSource),
    /// The global scope: `!`.
    Global(GlobalSource),
}

impl From<ModuleSource> for Source {
    fn from(source: ModuleSource) -> Self {
        Self::Module(source)
    }
}

impl From<GlobalSource> for Source {
    fn from(source: GlobalSource) -> Self {
        Self::Global(source)
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Module(source) => write!(f, "{source}"),
            Self::Global(source) => write!(f, "{source}"),
        }
    }
}

/// The global scope, upstream's `GlobalSource.instance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GlobalSource;

impl fmt::Display for GlobalSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("!")
    }
}

/// Unescapes a component (`what` is `Component`) or module source (`Module source`): a quoted
/// string is read as JSON, and anything else must be well-formed as it is.
fn unescape(text: &str, what: &str, is_well_formed: fn(&str) -> bool) -> Result<String, Error> {
    let invalid = || Error::syntax(format!("Invalid {what} '{text}'"));
    if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
        return parse_json_string(text).ok_or_else(invalid);
    }
    if !is_well_formed(text) {
        return Err(invalid());
    }
    Ok(text.into())
}

/// `JSON.parse` of a JSON string, or None where it throws. A `\u` escape of a lone surrogate, which
/// a Rust string can't hold, reads as U+FFFD.
fn parse_json_string(json: &str) -> Option<String> {
    let is_whitespace = |c: char| matches!(c, ' ' | '\t' | '\n' | '\r');
    let mut chars = json.trim_start_matches(is_whitespace).strip_prefix('"')?.chars();
    let mut text = String::new();
    loop {
        match chars.next()? {
            '"' => return chars.as_str().trim_start_matches(is_whitespace).is_empty().then_some(text),
            '\\' => {
                let unescaped = match chars.next()? {
                    '"' => '"',
                    '\\' => '\\',
                    '/' => '/',
                    'b' => '\u{8}',
                    'f' => '\u{c}',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'u' => {
                        let unit = hex4(&mut chars)?;
                        let mut after = chars.clone();
                        let low = (after.next() == Some('\\') && after.next() == Some('u'))
                            .then(|| hex4(&mut after))
                            .flatten()
                            .filter(|low| (0xdc00..0xe000).contains(low) && (0xd800..0xdc00).contains(&unit));
                        match low {
                            Some(low) => {
                                chars = after;
                                char::from_u32(0x10000 + ((unit - 0xd800) << 10) + (low - 0xdc00))?
                            }
                            None => char::from_u32(unit).unwrap_or(char::REPLACEMENT_CHARACTER),
                        }
                    }
                    _ => return None,
                };
                text.push(unescaped);
            }
            c if c < ' ' => return None,
            c => text.push(c),
        }
    }
}

/// Reads four hex digits.
fn hex4(chars: &mut std::str::Chars<'_>) -> Option<u32> {
    (0..4).try_fold(0, |value, _| Some(value * 16 + chars.next()?.to_digit(16)?))
}
