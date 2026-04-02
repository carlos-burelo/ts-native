use crate::document::DocumentState;
use tower_lsp::lsp_types::{Location, Url};

pub fn build_references(state: &DocumentState, line: u32, col: u32) -> Option<Vec<Location>> {
    let token = state.tokens.iter().find(|t| {
        t.line == line
            && t.col <= col
            && col < t.col + t.length
            && (t.kind == tsn_core::TokenKind::Identifier || t.kind.can_be_identifier())
    })?;

    let target_id = state
        .expr_types
        .get(&token.offset)
        .and_then(|info| info.symbol_id);
    let name = &token.lexeme;
    let url = Url::parse(&state.uri).ok()?;

    let locs: Vec<Location> = state
        .tokens
        .iter()
        .filter(|t| {
            if !(t.kind == tsn_core::TokenKind::Identifier || t.kind.can_be_identifier()) {
                return false;
            }
            if let Some(id) = target_id {
                state
                    .expr_types
                    .get(&t.offset)
                    .and_then(|info| info.symbol_id)
                    == Some(id)
            } else {
                &t.lexeme == name
            }
        })
        .filter_map(|t| {
            let range = tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: t.line,
                    character: t.col,
                },
                end: tower_lsp::lsp_types::Position {
                    line: t.line,
                    character: t.col + t.length,
                },
            };
            Some(Location::new(url.clone(), range))
        })
        .collect();

    if locs.is_empty() {
        None
    } else {
        Some(locs)
    }
}
