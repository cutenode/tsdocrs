//! The parser of a declaration reference, which keeps going after an error and collects them.

use super::scanner::{Scanner, Token};
use super::{
    Component, ComponentNavigation, ComponentPath, ComponentReference, ComponentRoot, ComponentString,
    DeclarationReference, GlobalSource, Meaning, ModuleSource, Navigation, Source, SymbolReference,
    SymbolReferenceOptions,
};

pub(super) struct Parser<'a> {
    scanner: Scanner<'a>,
    pub(super) errors: Vec<String>,
}

impl<'a> Parser<'a> {
    pub(super) fn new(text: &'a str) -> Self {
        let mut scanner = Scanner::new(text);
        scanner.scan();
        Self { scanner, errors: Vec::new() }
    }

    pub(super) fn eof(&self) -> bool {
        self.token() == Token::Eof
    }

    fn token(&self) -> Token {
        self.scanner.token()
    }

    pub(super) fn parse_declaration_reference(&mut self) -> DeclarationReference {
        let mut source = None;
        let mut navigation = None;
        if self.optional_token(Token::Exclamation) {
            source = Some(Source::Global(GlobalSource));
        } else if self.scanner.rescan_module_source() == Token::ModuleSource {
            source = Some(Source::Module(self.parse_module_source()));
            if self.optional_token(Token::Tilde) {
                navigation = Some(Navigation::Locals);
            }
        }
        let symbol = if self.is_start_of_component() {
            let root = ComponentRoot::new(self.parse_component());
            let path = self.parse_component_rest(root.into());
            Some(self.parse_symbol_rest(path))
        } else if self.token() == Token::Colon {
            let root = ComponentRoot::new(Component::String(ComponentString::parsed(String::new())));
            Some(self.parse_symbol_rest(root.into()))
        } else {
            None
        };
        DeclarationReference::new(source, navigation, symbol)
    }

    fn parse_module_source(&mut self) -> ModuleSource {
        let source = self.parse_token_string("Module source");
        self.expect_token(Token::Exclamation);
        ModuleSource::parsed(source)
    }

    fn parse_symbol_rest(&mut self, component_path: ComponentPath) -> SymbolReference {
        let mut options = SymbolReferenceOptions::default();
        if self.optional_token(Token::Colon) {
            options.meaning = self.try_parse_meaning();
            options.overload_index = self.try_parse_overload_index(options.meaning.is_some());
        }
        SymbolReference::new(Some(component_path), options)
    }

    fn parse_component_rest(&mut self, mut component_path: ComponentPath) -> ComponentPath {
        loop {
            let navigation = match self.token() {
                Token::Dot => Navigation::Exports,
                Token::Hash => Navigation::Members,
                Token::Tilde => Navigation::Locals,
                _ => return component_path,
            };
            self.scanner.scan();
            let component = self.parse_component();
            component_path = ComponentNavigation::new(component_path, navigation, component).into();
        }
    }

    fn try_parse_meaning(&mut self) -> Option<Meaning> {
        if self.token() != Token::Text {
            return None;
        }
        let meaning = Meaning::from_keyword(self.scanner.token_text())?;
        self.scanner.scan();
        Some(meaning)
    }

    fn try_parse_overload_index(&mut self, has_meaning: bool) -> Option<f64> {
        if self.optional_token(Token::OpenParen) {
            let overload_index = self.parse_decimal_digits();
            self.expect_token(Token::CloseParen);
            return Some(overload_index);
        }
        (!has_meaning).then(|| self.parse_decimal_digits())
    }

    fn parse_decimal_digits(&mut self) -> f64 {
        let text = self.scanner.token_text();
        let is_digits = self.token() == Token::Text && !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
        if !is_digits {
            self.errors.push("Decimal digit expected".into());
            return 0.0;
        }
        self.scanner.scan();
        // Past f64's range, digits read as infinity, as they do in JavaScript.
        text.parse().expect("digits are a float")
    }

    fn is_start_of_component(&self) -> bool {
        matches!(self.token(), Token::Text | Token::String | Token::OpenBracket)
    }

    /// Reads the token, which the caller has checked is a `what` token, and reports it if it's an
    /// unterminated string.
    fn parse_token_string(&mut self, what: &str) -> String {
        let text = self.scanner.token_text().to_owned();
        let string_is_unterminated = self.scanner.string_is_unterminated;
        self.scanner.scan();
        if string_is_unterminated {
            self.errors.push(format!("{what} is unterminated"));
        }
        text
    }

    fn parse_component(&mut self) -> Component {
        if self.token() == Token::OpenBracket {
            self.scanner.scan();
            let reference = self.parse_declaration_reference();
            self.expect_token(Token::CloseBracket);
            return Component::Reference(ComponentReference::new(reference));
        }
        let text = if self.token() == Token::String {
            self.parse_token_string("String")
        } else {
            // Text runs to the next punctuator or string, so at most one text token is read here,
            // and none after a navigation with no component.
            let mut text = String::new();
            while self.token() == Token::Text {
                text += &self.parse_token_string("Text");
            }
            text
        };
        Component::String(ComponentString::parsed(text))
    }

    fn optional_token(&mut self, token: Token) -> bool {
        if self.token() != token {
            return false;
        }
        self.scanner.scan();
        true
    }

    fn expect_token(&mut self, token: Token) {
        if self.token() != token {
            let (expected, actual) = (token.as_str(), self.token().as_str());
            self.errors.push(format!("Expected token '{expected}', received '{actual}' instead."));
            return;
        }
        self.scanner.scan();
    }
}
