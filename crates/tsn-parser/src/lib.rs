mod expressions;
mod parser;
mod profile;
mod stream;
mod types;

pub use parser::Parser;
pub use profile::ParseProfile;
pub use stream::TokenStream;

use tsn_core::{ast::Program, Diagnostic, Token};

pub fn parse(tokens: Vec<Token>, filename: &str) -> Result<Program, Vec<Diagnostic>> {
    let mut parser = Parser::new(tokens, filename.to_owned());
    parser.parse_program()
}

pub fn parse_with_profile(
    tokens: Vec<Token>,
    filename: &str,
) -> Result<(Program, ParseProfile), Vec<Diagnostic>> {
    let mut parser = Parser::new(tokens, filename.to_owned());
    parser.parse_program_with_profile()
}

pub fn parse_partial(tokens: Vec<Token>, filename: &str) -> (Program, Vec<Diagnostic>) {
    let mut parser = Parser::new(tokens, filename.to_owned());
    parser.parse_program_partial()
}
