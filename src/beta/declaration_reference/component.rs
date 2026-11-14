//! The components of a symbol's path, and the path itself.

use std::fmt;

use super::{DeclarationReference, Error, Navigation};

/// One step of a symbol's path: a name, or a bracketed reference to a symbol, like
/// `[Symbol.iterator]`.
#[derive(Clone, Debug, PartialEq)]
pub enum Component {
    /// A name, escaped.
    String(ComponentString),
    /// A bracketed reference.
    Reference(ComponentReference),
}

impl From<ComponentLike> for Component {
    /// The component `value` stands for, with a string escaped as it needs.
    fn from(value: ComponentLike) -> Self {
        match value {
            ComponentLike::Component(component) => component,
            ComponentLike::Reference(reference) => Self::Reference(ComponentReference::new(reference)),
            ComponentLike::String(text) => Self::String(ComponentString::escaping(&text)),
        }
    }
}

/// The component `value` stands for: upstream's `Component.from`.
pub(super) fn from_like(value: impl Into<ComponentLike>) -> Component {
    let value: ComponentLike = value.into();
    Component::from(value)
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(component) => write!(f, "{component}"),
            Self::Reference(component) => write!(f, "{component}"),
        }
    }
}

/// What a component can be made from: a component, a reference to bracket, or a name to escape.
#[derive(Clone, Debug, PartialEq)]
pub enum ComponentLike {
    /// A component as it is.
    Component(Component),
    /// A reference, which becomes a bracketed component.
    Reference(DeclarationReference),
    /// A name, which is escaped as it needs.
    String(String),
}

impl From<Component> for ComponentLike {
    fn from(component: Component) -> Self {
        Self::Component(component)
    }
}

impl From<ComponentString> for ComponentLike {
    fn from(component: ComponentString) -> Self {
        Self::Component(Component::String(component))
    }
}

impl From<ComponentReference> for ComponentLike {
    fn from(component: ComponentReference) -> Self {
        Self::Component(Component::Reference(component))
    }
}

impl From<DeclarationReference> for ComponentLike {
    fn from(reference: DeclarationReference) -> Self {
        Self::Reference(reference)
    }
}

impl From<&str> for ComponentLike {
    fn from(text: &str) -> Self {
        Self::String(text.into())
    }
}

impl From<String> for ComponentLike {
    fn from(text: String) -> Self {
        Self::String(text)
    }
}

/// A component's name, as it's written: escaped, in quotes if it needs them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentString {
    text: String,
}

impl ComponentString {
    /// The component for `text`, which is escaped already if `user_escaped`, or is escaped here if
    /// not.
    pub fn new(text: &str, user_escaped: bool) -> Result<Self, Error> {
        if !user_escaped {
            return Ok(Self::escaping(text));
        }
        if !DeclarationReference::is_well_formed_component_string(text) {
            return Err(Error::syntax(format!("Invalid Component '{text}'")));
        }
        Ok(Self { text: text.into() })
    }

    fn escaping(text: &str) -> Self {
        Self { text: DeclarationReference::escape_component_string(text) }
    }

    /// A component the parser read, which is taken as it is.
    pub(super) fn parsed(text: String) -> Self {
        Self { text }
    }

    /// Its escaped text.
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for ComponentString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// A bracketed reference as a component, like `[Symbol.iterator]`.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentReference {
    reference: Box<DeclarationReference>,
}

impl ComponentReference {
    /// The component for `reference`.
    pub fn new(reference: DeclarationReference) -> Self {
        Self { reference: Box::new(reference) }
    }

    /// Parses a bracketed reference, brackets included.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let inner = text.strip_prefix('[').and_then(|text| text.strip_suffix(']')).filter(|inner| !inner.is_empty());
        let Some(inner) = inner else {
            return Err(Error::syntax(format!("Invalid component reference: '{text}'")));
        };
        DeclarationReference::parse(inner).map(Self::new)
    }

    /// The reference in the brackets.
    pub fn reference(&self) -> &DeclarationReference {
        &self.reference
    }

    /// The same component with another reference.
    pub fn with_reference(self, reference: DeclarationReference) -> Self {
        Self::new(reference)
    }
}

impl fmt::Display for ComponentReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.reference)
    }
}

/// A symbol's path: its first component, or a path and a step from it.
#[derive(Clone, Debug, PartialEq)]
pub enum ComponentPath {
    /// The first component.
    Root(ComponentRoot),
    /// A step from a shorter path.
    Navigation(ComponentNavigation),
}

impl ComponentPath {
    /// Its last component.
    pub fn component(&self) -> &Component {
        match self {
            Self::Root(root) => root.component(),
            Self::Navigation(navigation) => navigation.component(),
        }
    }

    /// The path with a step to `component` through `navigation` at its end.
    pub fn add_navigation_step(self, navigation: Navigation, component: impl Into<ComponentLike>) -> Self {
        ComponentNavigation::new(self, navigation, from_like(component)).into()
    }
}

impl From<ComponentRoot> for ComponentPath {
    fn from(root: ComponentRoot) -> Self {
        Self::Root(root)
    }
}

impl From<ComponentNavigation> for ComponentPath {
    fn from(navigation: ComponentNavigation) -> Self {
        Self::Navigation(navigation)
    }
}

impl fmt::Display for ComponentPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root(root) => write!(f, "{root}"),
            Self::Navigation(navigation) => write!(f, "{navigation}"),
        }
    }
}

/// The first component of a symbol's path.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentRoot {
    component: Component,
}

impl ComponentRoot {
    /// The path that is just `component`.
    pub fn new(component: Component) -> Self {
        Self { component }
    }

    /// Its component.
    pub fn component(&self) -> &Component {
        &self.component
    }

    /// The same path with another component.
    pub fn with_component(self, component: impl Into<ComponentLike>) -> Self {
        Self::new(from_like(component))
    }
}

impl fmt::Display for ComponentRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.component)
    }
}

/// A step from a path to a component, through exports, members or locals.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentNavigation {
    parent: Box<ComponentPath>,
    navigation: Navigation,
    component: Component,
}

impl ComponentNavigation {
    /// The step from `parent` to `component` through `navigation`.
    pub fn new(parent: ComponentPath, navigation: Navigation, component: Component) -> Self {
        Self { parent: Box::new(parent), navigation, component }
    }

    /// The path the step is from.
    pub fn parent(&self) -> &ComponentPath {
        &self.parent
    }

    /// The symbol table the step looks the component up in.
    pub fn navigation(&self) -> Navigation {
        self.navigation
    }

    /// The component the step is to.
    pub fn component(&self) -> &Component {
        &self.component
    }

    /// The same step from another path.
    pub fn with_parent(self, parent: ComponentPath) -> Self {
        Self { parent: Box::new(parent), ..self }
    }

    /// The same step through another symbol table.
    pub fn with_navigation(self, navigation: Navigation) -> Self {
        Self { navigation, ..self }
    }

    /// The same step to another component.
    pub fn with_component(self, component: impl Into<ComponentLike>) -> Self {
        Self { component: from_like(component), ..self }
    }
}

impl fmt::Display for ComponentNavigation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.parent, self.navigation, self.component)
    }
}
