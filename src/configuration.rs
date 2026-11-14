//! What the parser is set up with: the tags it knows, which of them the tool supports, which HTML
//! elements it allows, and what it reports. `@microsoft/tsdoc`'s TSDocConfiguration.

use crate::parser::{TSDocMessageId, string_checks};

/// How a tag is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TSDocTagSyntaxKind {
    /// In braces, inside other text: `{@link}`.
    InlineTag,
    /// Starting a section of its own: `@remarks`.
    BlockTag,
    /// By itself, marking the item: `@internal`.
    ModifierTag,
}

/// How widely a standard tag is meant to be supported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Standardization {
    /// Every tool should support it.
    Core,
    /// A tool may support it, and should support it the standard way.
    Extended,
    /// A tool may support it, and how is its own.
    Discretionary,
    /// Not a standard tag.
    None,
}

/// A tag the parser knows.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TSDocTagDefinition {
    /// Its name, with its `@`.
    pub tag_name: String,
    /// The same in upper case, which lookups go by.
    pub tag_name_with_upper_case: String,
    /// How it's written.
    pub syntax_kind: TSDocTagSyntaxKind,
    /// How widely it's meant to be supported.
    pub standardization: Standardization,
    /// Whether a comment may have it more than once.
    pub allow_multiple: bool,
}

impl TSDocTagDefinition {
    /// A tag named `tag_name` (an `@`, a letter, then letters and digits), or why it can't be.
    pub fn new(tag_name: &str, syntax_kind: TSDocTagSyntaxKind, allow_multiple: bool) -> Result<Self, String> {
        string_checks::validate_tsdoc_tag_name(tag_name)?;
        Ok(Self::standard(tag_name, syntax_kind, allow_multiple, Standardization::None))
    }

    fn standard(tag_name: &str, syntax_kind: TSDocTagSyntaxKind, allow_multiple: bool, s: Standardization) -> Self {
        let tag_name_with_upper_case = tag_name.to_uppercase();
        Self { tag_name: tag_name.into(), tag_name_with_upper_case, syntax_kind, standardization: s, allow_multiple }
    }
}

/// TSDoc's standard tags, in `@microsoft/tsdoc`'s order.
pub fn standard_tags() -> Vec<TSDocTagDefinition> {
    use Standardization::{Core, Discretionary, Extended};
    use TSDocTagSyntaxKind::{BlockTag, InlineTag, ModifierTag};
    [
        ("@alpha", ModifierTag, false, Discretionary),
        ("@beta", ModifierTag, false, Discretionary),
        ("@defaultValue", BlockTag, false, Extended),
        ("@decorator", BlockTag, true, Extended),
        ("@deprecated", BlockTag, false, Core),
        ("@eventProperty", ModifierTag, false, Extended),
        ("@example", BlockTag, true, Extended),
        ("@experimental", ModifierTag, false, Discretionary),
        ("@inheritDoc", InlineTag, false, Extended),
        ("@internal", ModifierTag, false, Discretionary),
        ("@label", InlineTag, false, Core),
        ("@link", InlineTag, true, Core),
        ("@override", ModifierTag, false, Extended),
        ("@packageDocumentation", ModifierTag, false, Core),
        ("@param", BlockTag, true, Core),
        ("@privateRemarks", BlockTag, false, Core),
        ("@public", ModifierTag, false, Discretionary),
        ("@readonly", ModifierTag, false, Extended),
        ("@remarks", BlockTag, false, Core),
        ("@returns", BlockTag, false, Core),
        ("@sealed", ModifierTag, false, Extended),
        ("@see", BlockTag, false, Extended),
        ("@throws", BlockTag, true, Extended),
        ("@typeParam", BlockTag, true, Core),
        ("@virtual", ModifierTag, false, Extended),
        ("@jsx", BlockTag, false, Extended),
        ("@jsxRuntime", BlockTag, false, Extended),
        ("@jsxFrag", BlockTag, false, Extended),
        ("@jsxImportSource", BlockTag, false, Extended),
    ]
    .into_iter()
    .map(|(name, kind, multiple, standardization)| TSDocTagDefinition::standard(name, kind, multiple, standardization))
    .collect()
}

/// What the parser reports.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TSDocValidationConfiguration {
    /// Whether it leaves undefined tags unreported.
    pub ignore_undefined_tags: bool,
    /// Whether it reports tags the configuration doesn't support.
    pub report_unsupported_tags: bool,
    /// Whether it reports HTML elements the configuration doesn't support.
    pub report_unsupported_html_elements: bool,
}

/// What the parser is set up with. The default has TSDoc's standard tags, none of them marked
/// supported, and reports neither unsupported tags nor HTML elements, like
/// `new TSDocConfiguration()`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TSDocConfiguration {
    tag_definitions: Vec<TSDocTagDefinition>,
    /// The upper-cased names of the supported tags, in the order they were marked.
    supported_tags: Vec<String>,
    /// What the parser reports.
    pub validation: TSDocValidationConfiguration,
    supported_html_elements: Vec<String>,
}

impl Default for TSDocConfiguration {
    fn default() -> Self {
        let mut configuration = Self {
            tag_definitions: Vec::new(),
            supported_tags: Vec::new(),
            validation: TSDocValidationConfiguration::default(),
            supported_html_elements: Vec::new(),
        };
        configuration.clear(false);
        configuration
    }
}

impl TSDocConfiguration {
    /// The default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets it: no tags but the standard ones, unless `no_standard_tags`, none supported, and no
    /// validation or HTML elements.
    pub fn clear(&mut self, no_standard_tags: bool) {
        self.tag_definitions.clear();
        self.supported_tags.clear();
        self.validation = TSDocValidationConfiguration::default();
        self.supported_html_elements.clear();
        if !no_standard_tags {
            self.tag_definitions = standard_tags();
        }
    }

    /// The tags it knows, in the order they were added.
    pub fn tag_definitions(&self) -> &[TSDocTagDefinition] {
        &self.tag_definitions
    }

    /// The tags it knows that are supported.
    pub fn supported_tag_definitions(&self) -> impl Iterator<Item = &TSDocTagDefinition> {
        self.tag_definitions.iter().filter(|tag| self.supported_tags.contains(&tag.tag_name_with_upper_case))
    }

    /// The HTML elements it allows, when it reports the others.
    pub fn supported_html_elements(&self) -> &[String] {
        &self.supported_html_elements
    }

    /// The tag named `tag_name`, in any case.
    pub fn try_get_tag_definition(&self, tag_name: &str) -> Option<&TSDocTagDefinition> {
        self.try_get_tag_definition_with_upper_case(&tag_name.to_uppercase())
    }

    /// The tag whose upper-cased name is `upper`.
    pub fn try_get_tag_definition_with_upper_case(&self, upper: &str) -> Option<&TSDocTagDefinition> {
        self.tag_definitions.iter().find(|tag| tag.tag_name_with_upper_case == upper)
    }

    /// Adds a tag, or says which tag already has its name.
    pub fn add_tag_definition(&mut self, tag: TSDocTagDefinition) -> Result<(), String> {
        if let Some(existing) = self.try_get_tag_definition_with_upper_case(&tag.tag_name_with_upper_case) {
            if *existing == tag {
                return Ok(());
            }
            return Err(format!("A tag is already defined using the name {}", existing.tag_name));
        }
        self.tag_definitions.push(tag);
        Ok(())
    }

    /// Adds tags, and marks them supported or not when `supported` says.
    pub fn add_tag_definitions(
        &mut self,
        tags: impl IntoIterator<Item = TSDocTagDefinition>,
        supported: Option<bool>,
    ) -> Result<(), String> {
        for tag in tags {
            let upper = tag.tag_name_with_upper_case.clone();
            self.add_tag_definition(tag)?;
            if let Some(supported) = supported {
                self.set_support_for_tag(&upper, supported)?;
            }
        }
        Ok(())
    }

    /// Whether the tag it knows named `tag_name` is supported.
    pub fn is_tag_supported(&self, tag: &TSDocTagDefinition) -> bool {
        self.supported_tags.contains(&tag.tag_name_with_upper_case)
    }

    /// Marks the tag named `tag_name` supported or not, and turns on reporting unsupported tags.
    pub fn set_support_for_tag(&mut self, tag_name: &str, supported: bool) -> Result<(), String> {
        let Some(tag) = self.try_get_tag_definition(tag_name) else {
            return Err("The specified TSDocTagDefinition is not defined for this TSDocConfiguration".into());
        };
        let upper = tag.tag_name_with_upper_case.clone();
        self.supported_tags.retain(|tag| *tag != upper);
        if supported {
            self.supported_tags.push(upper);
        }
        self.validation.report_unsupported_tags = true;
        Ok(())
    }

    /// Sets the HTML elements it allows, and turns on reporting the others.
    pub fn set_supported_html_elements(&mut self, elements: impl IntoIterator<Item = String>) {
        self.supported_html_elements.clear();
        self.validation.report_unsupported_html_elements = true;
        for element in elements {
            if !self.supported_html_elements.contains(&element) {
                self.supported_html_elements.push(element);
            }
        }
    }

    /// Whether it allows the HTML element `name`.
    pub fn is_html_element_supported(&self, name: &str) -> bool {
        self.supported_html_elements.iter().any(|element| element == name)
    }

    /// Whether `id` is one of TSDoc's message ids.
    pub fn is_known_message_id(&self, id: &str) -> bool {
        TSDocMessageId::from_id(id).is_some()
    }
}

/// A tsdoc.json's settings, as `@microsoft/tsdoc-config` reads them. Loading one from a file, with
/// the files it extends, is [`config_file::load`](crate::config_file::load).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TSDocConfigFile {
    /// Its `$schema`.
    #[serde(rename = "$schema", default)]
    pub schema: String,
    /// The files it extends, by path or package name.
    #[serde(default)]
    pub extends: Vec<String>,
    /// Whether it leaves TSDoc's standard tags out.
    pub no_standard_tags: Option<bool>,
    /// The tags it defines.
    #[serde(default)]
    pub tag_definitions: Vec<TagDefinitionJson>,
    /// Which tags it supports, by name.
    #[serde(default)]
    pub support_for_tags: indexmap::IndexMap<String, bool>,
    /// The HTML elements it allows.
    pub supported_html_elements: Option<Vec<String>>,
    /// Whether it reports the HTML elements it doesn't allow.
    pub report_unsupported_html_elements: Option<bool>,
}

/// A tag a tsdoc.json defines.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TagDefinitionJson {
    /// Its name, with its `@`.
    pub tag_name: String,
    /// How it's written: `inline`, `block` or `modifier`.
    pub syntax_kind: SyntaxKindJson,
    /// Whether a comment may have it more than once.
    #[serde(default)]
    pub allow_multiple: bool,
}

/// How a tag is written, as tsdoc.json spells it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[allow(missing_docs)]
pub enum SyntaxKindJson {
    Inline,
    Block,
    Modifier,
}

impl TSDocConfigFile {
    /// What's wrong with it on its own, by TSDoc's message id: an HTML element or tag name that
    /// isn't one, or a tag it defines twice.
    pub fn check(&self) -> Result<(), (TSDocMessageId, String)> {
        let html_name = |name: &String| name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        if let Some(name) = self.supported_html_elements.iter().flatten().find(|name| html_name(name)) {
            return Err((TSDocMessageId::ConfigFileSchemaError, format!("\"{name}\" isn't an HTML element name")));
        }
        let mut names: Vec<String> = Vec::new();
        for tag in &self.tag_definitions {
            if string_checks::explain_if_invalid_tsdoc_tag_name(&tag.tag_name).is_some() {
                let text = format!("\"{}\" isn't a tag name: an @, a letter, then letters and digits", tag.tag_name);
                return Err((TSDocMessageId::ConfigFileInvalidTagName, text));
            }
            let upper = tag.tag_name.to_uppercase();
            if names.contains(&upper) {
                let text = format!("\"tagDefinitions\" defines \"{}\" more than once", tag.tag_name);
                return Err((TSDocMessageId::ConfigFileDuplicateTagName, text));
            }
            names.push(upper);
        }
        Ok(())
    }

    /// The configuration it alone sets up, as tsdoc-config's `configureParser` does without the
    /// files it extends.
    pub fn configure_parser(&self) -> Result<TSDocConfiguration, (TSDocMessageId, String)> {
        self.check()?;
        let mut configuration = TSDocConfiguration::new();
        configuration.clear(self.no_standard_tags.unwrap_or(false));
        self.update_parser(&mut configuration)?;
        Ok(configuration)
    }

    /// Adds its tags, supported tags and HTML elements to `configuration`.
    pub fn update_parser(&self, configuration: &mut TSDocConfiguration) -> Result<(), (TSDocMessageId, String)> {
        for tag in &self.tag_definitions {
            if let Some(defined) = configuration.try_get_tag_definition(&tag.tag_name) {
                let text = format!("\"{}\" is defined already, as \"{}\"", tag.tag_name, defined.tag_name);
                return Err((TSDocMessageId::ConfigFileDuplicateTagName, text));
            }
            let kind = match tag.syntax_kind {
                SyntaxKindJson::Inline => TSDocTagSyntaxKind::InlineTag,
                SyntaxKindJson::Block => TSDocTagSyntaxKind::BlockTag,
                SyntaxKindJson::Modifier => TSDocTagSyntaxKind::ModifierTag,
            };
            let definition = TSDocTagDefinition::new(&tag.tag_name, kind, tag.allow_multiple)
                .map_err(|text| (TSDocMessageId::ConfigFileInvalidTagName, text))?;
            configuration
                .add_tag_definition(definition)
                .map_err(|text| (TSDocMessageId::ConfigFileDuplicateTagName, text))?;
        }
        for (name, &supported) in &self.support_for_tags {
            if configuration.set_support_for_tag(name, supported).is_err() {
                let text = format!("\"supportForTags\" names \"{name}\", which isn't defined");
                return Err((TSDocMessageId::ConfigFileUndefinedTag, text));
            }
        }
        if let Some(html) = &self.supported_html_elements {
            configuration.set_supported_html_elements(html.iter().cloned());
        }
        if let Some(report) = self.report_unsupported_html_elements {
            configuration.validation.report_unsupported_html_elements = report;
        }
        Ok(())
    }
}
