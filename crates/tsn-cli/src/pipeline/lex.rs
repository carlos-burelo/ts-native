use super::colors::{footer, header, C_TOKENS, DIM, R};
/// Lexer execution and token debugging
use crate::args::DebugFlags;

pub fn lex(source: &str, path: &str, verbose: bool, debug: &DebugFlags) -> Vec<tsn_core::Token> {
    let tokens = tsn_lexer::scan(source, path);

    if verbose {
        eprintln!("[tsn] lexed {} tokens", tokens.len());
    }

    if debug.tokens {
        header(C_TOKENS, "tokens", path);
        eprintln!(
            "{}  {:>4}  {:>8}  {:>6}  {:<22}  lexeme{}",
            DIM, "idx", "line:col", "off", "kind", R
        );
        for (i, tok) in tokens.iter().enumerate() {
            eprintln!(
                "  {:>4}  {:>4}:{:<4}  {:6}  {:<22}  {}{:?}{}",
                i,
                tok.range.start.line,
                tok.range.start.column,
                tok.range.start.offset,
                format!("{:?}", tok.kind),
                DIM,
                tok.lexeme,
                R
            );
        }
        footer(C_TOKENS, &format!("{} tokens", tokens.len()));
    }

    tokens
}
