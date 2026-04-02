use tower_lsp::lsp_types::FoldingRange;
use tsn_core::TokenKind;

use crate::document::{DocumentState, TokenRecord};

pub fn build_folding_ranges(state: &DocumentState) -> Vec<FoldingRange> {
    fold_tokens(&state.tokens)
}

pub fn fold_tokens(tokens: &[TokenRecord]) -> Vec<FoldingRange> {
    let mut ranges = Vec::new();
    let mut brace_stack: Vec<u32> = Vec::new();
    let mut bracket_stack: Vec<u32> = Vec::new();

    for tok in tokens {
        match tok.kind {
            TokenKind::LBrace => brace_stack.push(tok.line),
            TokenKind::RBrace => {
                if let Some(start) = brace_stack.pop() {
                    if tok.line > start {
                        ranges.push(fold(start, tok.line));
                    }
                }
            }
            TokenKind::LBracket => bracket_stack.push(tok.line),
            TokenKind::RBracket => {
                if let Some(start) = bracket_stack.pop() {
                    if tok.line > start {
                        ranges.push(fold(start, tok.line));
                    }
                }
            }
            _ => {}
        }
    }

    ranges
}

#[inline]
fn fold(start_line: u32, end_line: u32) -> FoldingRange {
    FoldingRange {
        start_line,
        start_character: None,
        end_line,
        end_character: None,
        kind: None,
        collapsed_text: None,
    }
}
