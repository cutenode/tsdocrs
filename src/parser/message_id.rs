//! TSDoc's message ids: what each message the parser and the tsdoc.json loader report is about.
//! They're `@microsoft/tsdoc`'s, the ids `eslint-plugin-tsdoc` reports.

/// What a message is about, by `@microsoft/tsdoc`'s id for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TSDocMessageId {
    /// `tsdoc-config-file-not-found`
    ConfigFileNotFound,
    /// `tsdoc-config-invalid-json`
    ConfigInvalidJson,
    /// `tsdoc-config-unsupported-schema`
    ConfigFileUnsupportedSchema,
    /// `tsdoc-config-schema-error`
    ConfigFileSchemaError,
    /// `tsdoc-config-cyclic-extends`
    ConfigFileCyclicExtends,
    /// `tsdoc-config-unresolved-extends`
    ConfigFileUnresolvedExtends,
    /// `tsdoc-config-undefined-tag`
    ConfigFileUndefinedTag,
    /// `tsdoc-config-duplicate-tag-name`
    ConfigFileDuplicateTagName,
    /// `tsdoc-config-invalid-tag-name`
    ConfigFileInvalidTagName,
    /// `tsdoc-comment-not-found`
    CommentNotFound,
    /// `tsdoc-comment-missing-opening-delimiter`
    CommentOpeningDelimiterSyntax,
    /// `tsdoc-comment-missing-closing-delimiter`
    CommentMissingClosingDelimiter,
    /// `tsdoc-extra-inheritdoc-tag`
    ExtraInheritDocTag,
    /// `tsdoc-escape-right-brace`
    EscapeRightBrace,
    /// `tsdoc-escape-greater-than`
    EscapeGreaterThan,
    /// `tsdoc-missing-deprecation-message`
    MissingDeprecationMessage,
    /// `tsdoc-inheritdoc-incompatible-tag`
    InheritDocIncompatibleTag,
    /// `tsdoc-inheritdoc-incompatible-summary`
    InheritDocIncompatibleSummary,
    /// `tsdoc-inline-tag-missing-braces`
    InlineTagMissingBraces,
    /// `tsdoc-tag-should-not-have-braces`
    TagShouldNotHaveBraces,
    /// `tsdoc-unsupported-tag`
    UnsupportedTag,
    /// `tsdoc-undefined-tag`
    UndefinedTag,
    /// `tsdoc-param-tag-with-invalid-type`
    ParamTagWithInvalidType,
    /// `tsdoc-param-tag-with-invalid-optional-name`
    ParamTagWithInvalidOptionalName,
    /// `tsdoc-param-tag-with-invalid-name`
    ParamTagWithInvalidName,
    /// `tsdoc-param-tag-missing-hyphen`
    ParamTagMissingHyphen,
    /// `tsdoc-unnecessary-backslash`
    UnnecessaryBackslash,
    /// `tsdoc-missing-tag`
    MissingTag,
    /// `tsdoc-at-sign-in-word`
    AtSignInWord,
    /// `tsdoc-at-sign-without-tag-name`
    AtSignWithoutTagName,
    /// `tsdoc-malformed-inline-tag`
    MalformedInlineTag,
    /// `tsdoc-characters-after-block-tag`
    CharactersAfterBlockTag,
    /// `tsdoc-malformed-tag-name`
    MalformedTagName,
    /// `tsdoc-characters-after-inline-tag`
    CharactersAfterInlineTag,
    /// `tsdoc-inline-tag-missing-right-brace`
    InlineTagMissingRightBrace,
    /// `tsdoc-inline-tag-unescaped-brace`
    InlineTagUnescapedBrace,
    /// `tsdoc-inheritdoc-tag-syntax`
    InheritDocTagSyntax,
    /// `tsdoc-link-tag-empty`
    LinkTagEmpty,
    /// `tsdoc-link-tag-unescaped-text`
    LinkTagUnescapedText,
    /// `tsdoc-link-tag-destination-syntax`
    LinkTagDestinationSyntax,
    /// `tsdoc-link-tag-invalid-url`
    LinkTagInvalidUrl,
    /// `tsdoc-reference-missing-hash`
    ReferenceMissingHash,
    /// `tsdoc-reference-hash-syntax`
    ReferenceHashSyntax,
    /// `tsdoc-reference-malformed-package-name`
    ReferenceMalformedPackageName,
    /// `tsdoc-reference-malformed-import-path`
    ReferenceMalformedImportPath,
    /// `tsdoc-missing-reference`
    MissingReference,
    /// `tsdoc-reference-missing-dot`
    ReferenceMissingDot,
    /// `tsdoc-reference-selector-missing-parens`
    ReferenceSelectorMissingParens,
    /// `tsdoc-reference-missing-colon`
    ReferenceMissingColon,
    /// `tsdoc-reference-missing-right-paren`
    ReferenceMissingRightParen,
    /// `tsdoc-reference-symbol-syntax`
    ReferenceSymbolSyntax,
    /// `tsdoc-reference-missing-right-bracket`
    ReferenceMissingRightBracket,
    /// `tsdoc-reference-missing-quote`
    ReferenceMissingQuote,
    /// `tsdoc-reference-empty-identifier`
    ReferenceEmptyIdentifier,
    /// `tsdoc-reference-missing-identifier`
    ReferenceMissingIdentifier,
    /// `tsdoc-reference-unquoted-identifier`
    ReferenceUnquotedIdentifier,
    /// `tsdoc-reference-missing-label`
    ReferenceMissingLabel,
    /// `tsdoc-reference-selector-syntax`
    ReferenceSelectorSyntax,
    /// `tsdoc-html-tag-missing-greater-than`
    HtmlTagMissingGreaterThan,
    /// `tsdoc-html-tag-missing-equals`
    HtmlTagMissingEquals,
    /// `tsdoc-html-tag-missing-string`
    HtmlTagMissingString,
    /// `tsdoc-html-string-missing-quote`
    HtmlStringMissingQuote,
    /// `tsdoc-text-after-html-string`
    TextAfterHtmlString,
    /// `tsdoc-missing-html-end-tag`
    MissingHtmlEndTag,
    /// `tsdoc-malformed-html-name`
    MalformedHtmlName,
    /// `tsdoc-unsupported-html-name`
    UnsupportedHtmlElementName,
    /// `tsdoc-code-fence-opening-indent`
    CodeFenceOpeningIndent,
    /// `tsdoc-code-fence-specifier-syntax`
    CodeFenceSpecifierSyntax,
    /// `tsdoc-code-fence-closing-indent`
    CodeFenceClosingIndent,
    /// `tsdoc-code-fence-missing-delimiter`
    CodeFenceMissingDelimiter,
    /// `tsdoc-code-fence-closing-syntax`
    CodeFenceClosingSyntax,
    /// `tsdoc-code-span-empty`
    CodeSpanEmpty,
    /// `tsdoc-code-span-missing-delimiter`
    CodeSpanMissingDelimiter,
}

impl TSDocMessageId {
    /// Every id, in `@microsoft/tsdoc`'s order.
    pub const ALL: [TSDocMessageId; 73] = [
        Self::ConfigFileNotFound,
        Self::ConfigInvalidJson,
        Self::ConfigFileUnsupportedSchema,
        Self::ConfigFileSchemaError,
        Self::ConfigFileCyclicExtends,
        Self::ConfigFileUnresolvedExtends,
        Self::ConfigFileUndefinedTag,
        Self::ConfigFileDuplicateTagName,
        Self::ConfigFileInvalidTagName,
        Self::CommentNotFound,
        Self::CommentOpeningDelimiterSyntax,
        Self::CommentMissingClosingDelimiter,
        Self::ExtraInheritDocTag,
        Self::EscapeRightBrace,
        Self::EscapeGreaterThan,
        Self::MissingDeprecationMessage,
        Self::InheritDocIncompatibleTag,
        Self::InheritDocIncompatibleSummary,
        Self::InlineTagMissingBraces,
        Self::TagShouldNotHaveBraces,
        Self::UnsupportedTag,
        Self::UndefinedTag,
        Self::ParamTagWithInvalidType,
        Self::ParamTagWithInvalidOptionalName,
        Self::ParamTagWithInvalidName,
        Self::ParamTagMissingHyphen,
        Self::UnnecessaryBackslash,
        Self::MissingTag,
        Self::AtSignInWord,
        Self::AtSignWithoutTagName,
        Self::MalformedInlineTag,
        Self::CharactersAfterBlockTag,
        Self::MalformedTagName,
        Self::CharactersAfterInlineTag,
        Self::InlineTagMissingRightBrace,
        Self::InlineTagUnescapedBrace,
        Self::InheritDocTagSyntax,
        Self::LinkTagEmpty,
        Self::LinkTagUnescapedText,
        Self::LinkTagDestinationSyntax,
        Self::LinkTagInvalidUrl,
        Self::ReferenceMissingHash,
        Self::ReferenceHashSyntax,
        Self::ReferenceMalformedPackageName,
        Self::ReferenceMalformedImportPath,
        Self::MissingReference,
        Self::ReferenceMissingDot,
        Self::ReferenceSelectorMissingParens,
        Self::ReferenceMissingColon,
        Self::ReferenceMissingRightParen,
        Self::ReferenceSymbolSyntax,
        Self::ReferenceMissingRightBracket,
        Self::ReferenceMissingQuote,
        Self::ReferenceEmptyIdentifier,
        Self::ReferenceMissingIdentifier,
        Self::ReferenceUnquotedIdentifier,
        Self::ReferenceMissingLabel,
        Self::ReferenceSelectorSyntax,
        Self::HtmlTagMissingGreaterThan,
        Self::HtmlTagMissingEquals,
        Self::HtmlTagMissingString,
        Self::HtmlStringMissingQuote,
        Self::TextAfterHtmlString,
        Self::MissingHtmlEndTag,
        Self::MalformedHtmlName,
        Self::UnsupportedHtmlElementName,
        Self::CodeFenceOpeningIndent,
        Self::CodeFenceSpecifierSyntax,
        Self::CodeFenceClosingIndent,
        Self::CodeFenceMissingDelimiter,
        Self::CodeFenceClosingSyntax,
        Self::CodeSpanEmpty,
        Self::CodeSpanMissingDelimiter,
    ];

    /// The id, like `tsdoc-undefined-tag`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConfigFileNotFound => "tsdoc-config-file-not-found",
            Self::ConfigInvalidJson => "tsdoc-config-invalid-json",
            Self::ConfigFileUnsupportedSchema => "tsdoc-config-unsupported-schema",
            Self::ConfigFileSchemaError => "tsdoc-config-schema-error",
            Self::ConfigFileCyclicExtends => "tsdoc-config-cyclic-extends",
            Self::ConfigFileUnresolvedExtends => "tsdoc-config-unresolved-extends",
            Self::ConfigFileUndefinedTag => "tsdoc-config-undefined-tag",
            Self::ConfigFileDuplicateTagName => "tsdoc-config-duplicate-tag-name",
            Self::ConfigFileInvalidTagName => "tsdoc-config-invalid-tag-name",
            Self::CommentNotFound => "tsdoc-comment-not-found",
            Self::CommentOpeningDelimiterSyntax => "tsdoc-comment-missing-opening-delimiter",
            Self::CommentMissingClosingDelimiter => "tsdoc-comment-missing-closing-delimiter",
            Self::ExtraInheritDocTag => "tsdoc-extra-inheritdoc-tag",
            Self::EscapeRightBrace => "tsdoc-escape-right-brace",
            Self::EscapeGreaterThan => "tsdoc-escape-greater-than",
            Self::MissingDeprecationMessage => "tsdoc-missing-deprecation-message",
            Self::InheritDocIncompatibleTag => "tsdoc-inheritdoc-incompatible-tag",
            Self::InheritDocIncompatibleSummary => "tsdoc-inheritdoc-incompatible-summary",
            Self::InlineTagMissingBraces => "tsdoc-inline-tag-missing-braces",
            Self::TagShouldNotHaveBraces => "tsdoc-tag-should-not-have-braces",
            Self::UnsupportedTag => "tsdoc-unsupported-tag",
            Self::UndefinedTag => "tsdoc-undefined-tag",
            Self::ParamTagWithInvalidType => "tsdoc-param-tag-with-invalid-type",
            Self::ParamTagWithInvalidOptionalName => "tsdoc-param-tag-with-invalid-optional-name",
            Self::ParamTagWithInvalidName => "tsdoc-param-tag-with-invalid-name",
            Self::ParamTagMissingHyphen => "tsdoc-param-tag-missing-hyphen",
            Self::UnnecessaryBackslash => "tsdoc-unnecessary-backslash",
            Self::MissingTag => "tsdoc-missing-tag",
            Self::AtSignInWord => "tsdoc-at-sign-in-word",
            Self::AtSignWithoutTagName => "tsdoc-at-sign-without-tag-name",
            Self::MalformedInlineTag => "tsdoc-malformed-inline-tag",
            Self::CharactersAfterBlockTag => "tsdoc-characters-after-block-tag",
            Self::MalformedTagName => "tsdoc-malformed-tag-name",
            Self::CharactersAfterInlineTag => "tsdoc-characters-after-inline-tag",
            Self::InlineTagMissingRightBrace => "tsdoc-inline-tag-missing-right-brace",
            Self::InlineTagUnescapedBrace => "tsdoc-inline-tag-unescaped-brace",
            Self::InheritDocTagSyntax => "tsdoc-inheritdoc-tag-syntax",
            Self::LinkTagEmpty => "tsdoc-link-tag-empty",
            Self::LinkTagUnescapedText => "tsdoc-link-tag-unescaped-text",
            Self::LinkTagDestinationSyntax => "tsdoc-link-tag-destination-syntax",
            Self::LinkTagInvalidUrl => "tsdoc-link-tag-invalid-url",
            Self::ReferenceMissingHash => "tsdoc-reference-missing-hash",
            Self::ReferenceHashSyntax => "tsdoc-reference-hash-syntax",
            Self::ReferenceMalformedPackageName => "tsdoc-reference-malformed-package-name",
            Self::ReferenceMalformedImportPath => "tsdoc-reference-malformed-import-path",
            Self::MissingReference => "tsdoc-missing-reference",
            Self::ReferenceMissingDot => "tsdoc-reference-missing-dot",
            Self::ReferenceSelectorMissingParens => "tsdoc-reference-selector-missing-parens",
            Self::ReferenceMissingColon => "tsdoc-reference-missing-colon",
            Self::ReferenceMissingRightParen => "tsdoc-reference-missing-right-paren",
            Self::ReferenceSymbolSyntax => "tsdoc-reference-symbol-syntax",
            Self::ReferenceMissingRightBracket => "tsdoc-reference-missing-right-bracket",
            Self::ReferenceMissingQuote => "tsdoc-reference-missing-quote",
            Self::ReferenceEmptyIdentifier => "tsdoc-reference-empty-identifier",
            Self::ReferenceMissingIdentifier => "tsdoc-reference-missing-identifier",
            Self::ReferenceUnquotedIdentifier => "tsdoc-reference-unquoted-identifier",
            Self::ReferenceMissingLabel => "tsdoc-reference-missing-label",
            Self::ReferenceSelectorSyntax => "tsdoc-reference-selector-syntax",
            Self::HtmlTagMissingGreaterThan => "tsdoc-html-tag-missing-greater-than",
            Self::HtmlTagMissingEquals => "tsdoc-html-tag-missing-equals",
            Self::HtmlTagMissingString => "tsdoc-html-tag-missing-string",
            Self::HtmlStringMissingQuote => "tsdoc-html-string-missing-quote",
            Self::TextAfterHtmlString => "tsdoc-text-after-html-string",
            Self::MissingHtmlEndTag => "tsdoc-missing-html-end-tag",
            Self::MalformedHtmlName => "tsdoc-malformed-html-name",
            Self::UnsupportedHtmlElementName => "tsdoc-unsupported-html-name",
            Self::CodeFenceOpeningIndent => "tsdoc-code-fence-opening-indent",
            Self::CodeFenceSpecifierSyntax => "tsdoc-code-fence-specifier-syntax",
            Self::CodeFenceClosingIndent => "tsdoc-code-fence-closing-indent",
            Self::CodeFenceMissingDelimiter => "tsdoc-code-fence-missing-delimiter",
            Self::CodeFenceClosingSyntax => "tsdoc-code-fence-closing-syntax",
            Self::CodeSpanEmpty => "tsdoc-code-span-empty",
            Self::CodeSpanMissingDelimiter => "tsdoc-code-span-missing-delimiter",
        }
    }

    /// The id named `id`, if TSDoc has one.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == id)
    }
}

impl std::fmt::Display for TSDocMessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
