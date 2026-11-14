//! A module a reference's symbol is in, and the package it's from.

use std::fmt;

use super::{DeclarationReference, Error};
use crate::parser::string_checks::{explain_if_invalid_package_name, is_line_terminator};

/// A module, like `@scope/pkg/path`, written with a `!` after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleSource {
    escaped_path: String,
    /// The package parts [`ModuleSource::from_package`] and [`ModuleSource::from_scoped_package`]
    /// were given, which upstream keeps instead of parsing them from the path.
    package: Option<Package>,
}

/// A module path's package and the path in it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Package {
    package_name: String,
    scope_name: String,
    unscoped_package_name: String,
    import_path: Option<String>,
}

impl ModuleSource {
    /// The module at `path`, which is escaped already if `user_escaped` (upstream's default), or
    /// is escaped here if not.
    pub fn new(path: &str, user_escaped: bool) -> Result<Self, Error> {
        let escaped_path = if user_escaped {
            if !DeclarationReference::is_well_formed_module_source_string(path) {
                return Err(Error::syntax(format!("Invalid Module source '{path}'")));
            }
            path.into()
        } else {
            DeclarationReference::escape_module_source_string(path)
        };
        Ok(Self { escaped_path, package: None })
    }

    /// A module source the parser read, which is taken as it is.
    pub(super) fn parsed(escaped_path: String) -> Self {
        Self { escaped_path, package: None }
    }

    /// Its path as it's written, escaped.
    pub fn escaped_path(&self) -> &str {
        &self.escaped_path
    }

    /// Its path, unescaped. An escape JSON doesn't have, like `\x41`, is well-formed but can't be
    /// unescaped.
    pub fn path(&self) -> Result<String, Error> {
        DeclarationReference::unescape_module_source_string(&self.escaped_path)
    }

    /// Its package's name, scope included, or an empty string if the path isn't in a package.
    pub fn package_name(&self) -> Result<String, Error> {
        Ok(self.package()?.package_name)
    }

    /// Its package's scope, with its `@`, or an empty string.
    pub fn scope_name(&self) -> Result<String, Error> {
        let scope_name = self.package()?.scope_name;
        Ok(if scope_name.is_empty() { scope_name } else { format!("@{scope_name}") })
    }

    /// Its package's name without the scope.
    pub fn unscoped_package_name(&self) -> Result<String, Error> {
        Ok(self.package()?.unscoped_package_name)
    }

    /// The path in its package, or the whole path if it isn't in a package.
    pub fn import_path(&self) -> Result<String, Error> {
        Ok(self.package()?.import_path.unwrap_or_default())
    }

    /// The module for a package by its scope (with or without its `@`) and unscoped name, or a path
    /// in it.
    pub fn from_scoped_package(
        scope_name: Option<&str>,
        unscoped_package_name: &str,
        import_path: Option<&str>,
    ) -> Result<Self, Error> {
        let (scope_name, package_name) = match scope_name.filter(|scope_name| !scope_name.is_empty()) {
            Some(scope_name) => {
                let scope_name = scope_name.strip_prefix('@').unwrap_or(scope_name);
                (scope_name, format!("@{scope_name}/{unscoped_package_name}"))
            }
            None => ("", unscoped_package_name.to_owned()),
        };
        let package = Package {
            package_name: package_name.clone(),
            scope_name: scope_name.into(),
            unscoped_package_name: unscoped_package_name.into(),
            import_path: None,
        };
        Self::from_package_name(Some(package), &package_name, import_path)
    }

    /// The module for a package, or a path in it.
    pub fn from_package(package_name: &str, import_path: Option<&str>) -> Result<Self, Error> {
        Self::from_package_name(parse_package_name(package_name), package_name, import_path)
    }

    fn from_package_name(
        package: Option<Package>,
        package_name: &str,
        import_path: Option<&str>,
    ) -> Result<Self, Error> {
        let Some(mut package) = package else {
            return Err(Error::other("Parsed package must be provided."));
        };
        if let Some(explanation) = explain_if_invalid_package_name(package_name) {
            return Err(Error::syntax(format!("Invalid NPM package name: {explanation}")));
        }
        let mut path = package_name.to_owned();
        if let Some(import_path) = import_path.filter(|import_path| !import_path.is_empty()) {
            if is_invalid_import_path(import_path) {
                // Upstream's message has no closing quote.
                return Err(Error::syntax(format!("Invalid import path '{import_path}")));
            }
            path = format!("{path}/{import_path}");
            package.import_path = Some(import_path.into());
        }
        let source = Self::new(&path, true)?;
        Ok(Self { package: Some(package), ..source })
    }

    /// The package parts it was made from, or else parsed from its path.
    fn package(&self) -> Result<Package, Error> {
        if let Some(package) = &self.package {
            return Ok(package.clone());
        }
        let path = self.path()?;
        let package = parse_package_name(&path)
            .filter(|package| explain_if_invalid_package_name(&package.package_name).is_none());
        Ok(package.unwrap_or(Package {
            package_name: String::new(),
            scope_name: String::new(),
            unscoped_package_name: String::new(),
            import_path: Some(path),
        }))
    }
}

impl fmt::Display for ModuleSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}!", self.escaped_path)
    }
}

/// Splits a module path into its package and the path in it, as upstream's
/// `/^((?:@([^/]+?)\/)?([^/]+?))(?:\/(.+))?$/` does: a scope is tried first and dropped if the rest
/// doesn't match without it, and the path in the package can't have a line terminator.
fn parse_package_name(text: &str) -> Option<Package> {
    let scoped = text.strip_prefix('@').and_then(|rest| {
        let (scope, after) = rest.split_once('/')?;
        let (name, import_path) = name_and_import_path(after)?;
        (!scope.is_empty()).then_some((scope, name, import_path))
    });
    let (scope, name, import_path) = match scoped {
        Some(scoped) => scoped,
        None => {
            let (name, import_path) = name_and_import_path(text)?;
            ("", name, import_path)
        }
    };
    let package_name_length = text.len() - import_path.map_or(0, |import_path| import_path.len() + 1);
    Some(Package {
        package_name: text[..package_name_length].into(),
        scope_name: scope.into(),
        unscoped_package_name: name.into(),
        import_path: import_path.map(Into::into),
    })
}

/// A package name without its scope, up to the first `/`, and the path after that, if any.
fn name_and_import_path(text: &str) -> Option<(&str, Option<&str>)> {
    let Some((name, import_path)) = text.split_once('/') else {
        return (!text.is_empty()).then_some((text, None));
    };
    let is_valid = !name.is_empty() && !import_path.is_empty() && !import_path.contains(is_line_terminator);
    is_valid.then_some((name, Some(import_path)))
}

/// Whether an import path is relative or absolute, as upstream's `/^(\.\.?([\\/]|$)|[\\/])/`
/// tests: `.`, `..`, or starting with `./`, `../` or `/`, with either slash.
fn is_invalid_import_path(import_path: &str) -> bool {
    let is_slash = |c: char| c == '/' || c == '\\';
    let after_dots = import_path.strip_prefix("..").or_else(|| import_path.strip_prefix('.'));
    import_path.starts_with(is_slash) || after_dots.is_some_and(|rest| rest.is_empty() || rest.starts_with(is_slash))
}
