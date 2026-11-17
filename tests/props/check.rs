//! Properties of `check_string`, the parse that leaves paragraphs unsplit.

use hegel::TestCase;
use tsdocrs::{TSDocParser, parser::ParserContext};

use crate::common::{
    configuration,
    generate::{comment, config},
};

/// A drawn comment checks to what it parses to but for the tree, and fails to check the way it
/// fails to parse.
#[hegel::test(test_cases = 300)]
fn check_is_parse_but_for_the_tree(tc: TestCase) {
    let comment = comment(&tc);
    let Ok(configuration) = configuration(config(&tc).as_ref()) else { return };
    let parser = TSDocParser::new(&configuration);
    let checked = parser.check_string(&comment);
    match parser.parse_string(&comment) {
        Ok(parsed) => {
            assert_eq!(ParserContext { doc_comment: parsed.doc_comment.clone(), ..checked.unwrap() }, parsed);
        }
        Err(error) => assert_eq!(checked, Err(error)),
    }
}
