//! The symbol a reference points to: its path, meaning and overload.

use std::fmt;

use super::{ComponentLike, ComponentPath, Error, Navigation};

/// The kind of declaration a symbol is, to tell apart declarations that share a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Meaning {
    /// `class`.
    Class,
    /// `interface`.
    Interface,
    /// `type`.
    TypeAlias,
    /// `enum`.
    Enum,
    /// `namespace`.
    Namespace,
    /// `function`.
    Function,
    /// `var`.
    Variable,
    /// `constructor`.
    Constructor,
    /// `member`: a class or enum member.
    Member,
    /// `event`.
    Event,
    /// `call`: a call signature.
    CallSignature,
    /// `new`: a construct signature.
    ConstructSignature,
    /// `index`: an index signature.
    IndexSignature,
    /// `complex`: any complex type.
    ComplexType,
}

impl Meaning {
    pub(super) const ALL: [Self; 14] = [
        Self::Class,
        Self::Interface,
        Self::TypeAlias,
        Self::Enum,
        Self::Namespace,
        Self::Function,
        Self::Variable,
        Self::Constructor,
        Self::Member,
        Self::Event,
        Self::CallSignature,
        Self::ConstructSignature,
        Self::IndexSignature,
        Self::ComplexType,
    ];

    /// Its keyword, like `class`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Interface => "interface",
            Self::TypeAlias => "type",
            Self::Enum => "enum",
            Self::Namespace => "namespace",
            Self::Function => "function",
            Self::Variable => "var",
            Self::Constructor => "constructor",
            Self::Member => "member",
            Self::Event => "event",
            Self::CallSignature => "call",
            Self::ConstructSignature => "new",
            Self::IndexSignature => "index",
            Self::ComplexType => "complex",
        }
    }

    pub(super) fn from_keyword(keyword: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|meaning| meaning.as_str() == keyword)
    }
}

impl fmt::Display for Meaning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A [`SymbolReference`]'s meaning and overload index.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SymbolReferenceOptions {
    /// The kind of declaration.
    pub meaning: Option<Meaning>,
    /// Which overload, a JavaScript number.
    pub overload_index: Option<f64>,
}

/// A reference to a TypeScript symbol: its path, and the meaning and overload that pick one
/// declaration of it.
#[derive(Clone, Debug, PartialEq)]
pub struct SymbolReference {
    component_path: Option<ComponentPath>,
    meaning: Option<Meaning>,
    overload_index: Option<f64>,
}

impl SymbolReference {
    /// The symbol at `component_path`, with `options`' meaning and overload index.
    pub fn new(component_path: Option<ComponentPath>, options: SymbolReferenceOptions) -> Self {
        Self { component_path, meaning: options.meaning, overload_index: options.overload_index }
    }

    /// The symbol without a path, meaning or overload index.
    pub fn empty() -> Self {
        Self::new(None, SymbolReferenceOptions::default())
    }

    /// Its path.
    pub fn component_path(&self) -> Option<&ComponentPath> {
        self.component_path.as_ref()
    }

    /// The kind of declaration it is.
    pub fn meaning(&self) -> Option<Meaning> {
        self.meaning
    }

    /// Which overload it is, a JavaScript number.
    pub fn overload_index(&self) -> Option<f64> {
        self.overload_index
    }

    /// The same symbol at another path.
    pub fn with_component_path(self, component_path: Option<ComponentPath>) -> Self {
        Self { component_path, ..self }
    }

    /// The same symbol with another meaning.
    pub fn with_meaning(self, meaning: Option<Meaning>) -> Self {
        Self { meaning, ..self }
    }

    /// The same symbol with another overload index.
    pub fn with_overload_index(self, overload_index: Option<f64>) -> Self {
        Self { overload_index, ..self }
    }

    /// The symbol a step from this one to `component` through `navigation`, without this one's
    /// meaning and overload index.
    pub fn add_navigation_step(
        self,
        navigation: Navigation,
        component: impl Into<ComponentLike>,
    ) -> Result<Self, Error> {
        let Some(component_path) = self.component_path else {
            return Err(Error::other("Cannot add a navigation step to an empty symbol reference."));
        };
        let component_path = component_path.add_navigation_step(navigation, component);
        Ok(Self::new(Some(component_path), SymbolReferenceOptions::default()))
    }
}

impl fmt::Display for SymbolReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(component_path) = &self.component_path {
            write!(f, "{component_path}")?;
        }
        match (self.meaning, self.overload_index) {
            (Some(meaning), Some(index)) => write!(f, ":{meaning}({})", js_number(index)),
            (Some(meaning), None) => write!(f, ":{meaning}"),
            (None, Some(index)) => write!(f, ":{}", js_number(index)),
            (None, None) => Ok(()),
        }
    }
}

/// `n` as JavaScript writes a number: exponents from 1e21 and below 1e-6, and no `-0`.
fn js_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if n == 0.0 {
        return "0".into();
    }
    if (1e-6..1e21).contains(&n.abs()) {
        return n.to_string();
    }
    // Rust writes the same shortest digits as JavaScript, but without `+` on a positive exponent.
    let exponential = format!("{n:e}");
    match exponential.split_once('e') {
        Some((digits, exponent)) if !exponent.starts_with('-') => format!("{digits}e+{exponent}"),
        _ => exponential,
    }
}
