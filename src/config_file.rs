//! tsdoc.json: the tags and HTML elements a project's TSDoc defines, read the way
//! `@microsoft/tsdoc-config` reads them, with the files it `extends`, and found the way it finds
//! one for a source file.

use std::{
    fs,
    path::{Path, PathBuf},
};

use oxc_resolver::{ResolveOptions, Resolver};
use serde::Deserialize;
use serde_json::Value;

use crate::{TSDocConfigFile, TSDocConfiguration, TSDocMessageId};

/// The file's name.
pub const FILE_NAME: &str = "tsdoc.json";

/// The `$schema` a tsdoc.json has to have.
pub const SCHEMA: &str = "https://developer.microsoft.com/json-schemas/tsdoc/v0/tsdoc.schema.json";

/// Why a tsdoc.json can't be used: the file, TSDoc's message id, and what's wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigError {
    /// The file.
    pub path: PathBuf,
    /// What's wrong, by one of TSDoc's `tsdoc-config-*` ids.
    pub message_id: TSDocMessageId,
    /// What's wrong.
    pub text: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}: {}", self.path.display(), self.message_id, self.text)
    }
}

impl std::error::Error for ConfigError {}

fn error(path: &Path, message_id: TSDocMessageId, text: impl std::fmt::Display) -> ConfigError {
    ConfigError { path: path.to_owned(), message_id, text: text.to_string() }
}


/// A tsdoc.json and the files it extends, read.
struct Loaded {
    path: PathBuf,
    file: TSDocConfigFile,
    extends: Vec<Loaded>,
}

/// The configuration the tsdoc.json at `path` sets up, and the files it read, `path` first.
pub fn load(path: &Path) -> Result<(TSDocConfiguration, Vec<PathBuf>), ConfigError> {
    let resolver = Resolver::new(ResolveOptions::default());
    let loaded = read(path, &resolver, &mut Vec::new())?;
    let mut configuration = TSDocConfiguration::new();
    configuration.clear(no_standard_tags(&loaded).unwrap_or(false));
    configure(&loaded, &mut configuration)?;
    let mut sources = Vec::new();
    paths(&loaded, &mut sources);
    Ok((configuration, sources))
}

/// Reads the file at `path`, and the files it extends. `chain` holds the real paths of the files
/// that extend it, to stop at a cycle.
fn read(path: &Path, resolver: &Resolver, chain: &mut Vec<PathBuf>) -> Result<Loaded, ConfigError> {
    let not_found = |e: std::io::Error| error(path, TSDocMessageId::ConfigFileNotFound, e);
    let real = dunce::canonicalize(path).map_err(not_found)?;
    if chain.contains(&real) {
        let text = "it extends itself through the files it extends";
        return Err(error(path, TSDocMessageId::ConfigFileCyclicExtends, text));
    }
    let mut text = fs::read_to_string(path).map_err(not_found)?;
    let invalid = |e: &dyn std::fmt::Display| error(path, TSDocMessageId::ConfigInvalidJson, e);
    json_strip_comments::strip(&mut text).map_err(|e| invalid(&e))?;
    let value: Value = serde_json::from_str(&text).map_err(|e| invalid(&e))?;
    if value.get("$schema").and_then(Value::as_str) != Some(SCHEMA) {
        let text = format!("expected a \"$schema\" of \"{SCHEMA}\"");
        return Err(error(path, TSDocMessageId::ConfigFileUnsupportedSchema, text));
    }
    let file = TSDocConfigFile::deserialize(value).map_err(|e| error(path, TSDocMessageId::ConfigFileSchemaError, e))?;
    file.check().map_err(|(message_id, text)| error(path, message_id, text))?;
    let dir = real.parent().expect("a file is in a folder").to_owned();
    chain.push(real);
    let mut extends = Vec::new();
    for specifier in &file.extends {
        let resolved = resolver.resolve(&dir, specifier).map_err(|e| {
            let text = format!("can't resolve \"{specifier}\": {e}");
            error(path, TSDocMessageId::ConfigFileUnresolvedExtends, text)
        })?;
        extends.push(read(resolved.path(), resolver, chain)?);
    }
    chain.pop();
    Ok(Loaded { path: path.to_owned(), file, extends })
}

/// `noStandardTags`, from the file or else the last of the files it extends that sets it.
fn no_standard_tags(loaded: &Loaded) -> Option<bool> {
    loaded.file.no_standard_tags.or_else(|| loaded.extends.iter().rev().find_map(no_standard_tags))
}

/// Adds what `loaded` sets to `configuration`, after what the files it extends set.
fn configure(loaded: &Loaded, configuration: &mut TSDocConfiguration) -> Result<(), ConfigError> {
    for base in &loaded.extends {
        configure(base, configuration)?;
    }
    loaded.file.update_parser(configuration).map_err(|(message_id, text)| error(&loaded.path, message_id, text))
}

/// The paths of `loaded` and the files it extends.
fn paths(loaded: &Loaded, out: &mut Vec<PathBuf>) {
    out.push(loaded.path.clone());
    loaded.extends.iter().for_each(|base| paths(base, out));
}

/// The tsdoc.json `@microsoft/tsdoc-config` reads for a file in `folder`: the one in the nearest
/// folder from there up that has a tsconfig.json or a package.json, if it's there, and the files
/// it looked for to find it.
pub fn find(folder: &Path) -> (Option<PathBuf>, Vec<PathBuf>) {
    let folder = std::path::absolute(folder).unwrap_or_else(|_| folder.to_owned());
    let mut looked = Vec::new();
    for dir in folder.ancestors() {
        for marker in ["tsconfig.json", "package.json"] {
            looked.push(dir.join(marker));
            if dir.join(marker).is_file() {
                let config = dir.join(FILE_NAME);
                looked.push(config.clone());
                return (config.is_file().then_some(config), looked);
            }
        }
    }
    (None, looked)
}
