//! A TSDoc parser: a port of `@microsoft/tsdoc` that reads a `/** */` comment into the same tree
//! of doc nodes, with the same messages, ids and texts.
//!
//! ```
//! use tsdocrs::{TSDocConfiguration, TSDocParser, nodes::NodeRef};
//!
//! let configuration = TSDocConfiguration::new();
//! let parser = TSDocParser::new(&configuration);
//! let context = parser.parse_string("/**\n * Adds.\n * @param a - The first\n * @bogus\n */").unwrap();
//! assert_eq!(context.doc_comment.params.blocks[0].parameter_name, "a");
//! let message = &context.log[0];
//! assert_eq!(message.message_id.as_str(), "tsdoc-undefined-tag");
//! assert_eq!(message.text, "(4,4): The TSDoc tag \"@bogus\" is not defined in this configuration");
//! assert_eq!(NodeRef::Comment(&context.doc_comment).child_nodes().len(), 2);
//! ```
//!
//! Offsets are bytes into the parsed `&str`, where `@microsoft/tsdoc` counts UTF-16 code units;
//! the line and column in a message's text count UTF-16 code units, as TSDoc's do.

pub mod beta;
#[cfg(feature = "config-file")]
pub mod config_file;
mod configuration;
pub mod emitters;
pub mod nodes;
pub mod parser;
pub mod transforms;

pub use self::{
    configuration::{
        Standardization, SyntaxKindJson, TSDocConfigFile, TSDocConfiguration, TSDocTagDefinition, TSDocTagSyntaxKind,
        TSDocValidationConfiguration, TagDefinitionJson, standard_tags,
    },
    parser::{ParseError, ParsedTag, ParserContext, ParserMessage, TSDocMessageId, TSDocParser},
};
