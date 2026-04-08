use crate::document::DocumentState;
use crate::util::converters::range_on_line;
use std::collections::HashSet;
use tower_lsp::lsp_types::{DocumentHighlight, DocumentHighlightKind};
use tsn_core::TokenKind;

pub fn build_document_highlights(
    state: &DocumentState,
    line: u32,
    col: u32,
) -> Vec<DocumentHighlight> {
    let Some(tok) = state.tokens.iter().find(|t| {
        t.line == line && t.kind == TokenKind::Identifier && t.col <= col && col < t.col + t.length
    }) else {
        return Vec::new();
    };

    let name = tok.lexeme.as_str();

    // Collect declaration positions (line + col) from symbols.
    let decl_positions: HashSet<(u32, u32)> = state
        .symbols
        .iter()
        .filter(|s| s.name == name && s.line != u32::MAX)
        .map(|s| (s.line, s.col))
        .collect();

    state
        .tokens
        .iter()
        .filter(|t| t.kind == TokenKind::Identifier && t.lexeme == name)
        .map(|t| {
            let kind = if decl_positions.contains(&(t.line, t.col)) {
                // Exact declaration position → WRITE.
                DocumentHighlightKind::WRITE
            } else if is_assignment_lhs(state, t) {
                DocumentHighlightKind::WRITE
            } else {
                DocumentHighlightKind::READ
            };
            DocumentHighlight {
                range: range_on_line(t.line, t.col, t.col + t.length),
                kind: Some(kind),
            }
        })
        .collect()
}

/// Returns true if `tok` is on the left-hand side of an assignment.
/// An assignment is `ident =` where the `=` is not part of `==`, `!=`, `<=`, `>=`, `=>`.
fn is_assignment_lhs(state: &DocumentState, tok: &crate::document::TokenRecord) -> bool {
    // Find the index of tok in the token list.
    let idx = state
        .tokens
        .iter()
        .position(|t| std::ptr::eq(t, tok))
        .unwrap_or(usize::MAX);

    if idx == usize::MAX {
        return false;
    }

    // Look at token after `tok` (skip any `[expr]` subscript access for now — simple case).
    let next = state.tokens.get(idx + 1);
    match next.map(|t| t.kind) {
        Some(TokenKind::Eq) => {
            // EqEq/EqEqEq are different token kinds — TokenKind::Eq is a single `=`.
            true
        }
        _ => false,
    }
}
