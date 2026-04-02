use crate::document::DocumentState;
use crate::util::converters::range_on_line;
use tower_lsp::lsp_types::{PrepareRenameResponse, TextEdit, Url, WorkspaceEdit};
use tsn_checker::symbol::SymbolId;
use tsn_core::TokenKind;

pub fn build_prepare_rename(
    state: &DocumentState,
    line: u32,
    col: u32,
) -> Option<PrepareRenameResponse> {
    let token = find_ident_at(state, line, col)?;

    // Only allow rename when we have a precise symbol_id — prevents silently
    // renaming unrelated symbols with the same lexeme across different scopes.
    let sid = resolve_symbol_id(state, token.offset)?;
    state.symbols.iter().find(|s| s.symbol_id == Some(sid))?;

    let range = range_on_line(line, token.col, token.col + token.length);
    Some(PrepareRenameResponse::Range(range))
}

pub fn build_rename(
    state: &DocumentState,
    line: u32,
    col: u32,
    new_name: String,
) -> Option<WorkspaceEdit> {
    let token = find_ident_at(state, line, col)?;
    let target_id: Option<SymbolId> = resolve_symbol_id(state, token.offset);

    let edits: Vec<TextEdit> = state
        .tokens
        .iter()
        .filter(|t| {
            if !matches!(t.kind, TokenKind::Identifier) && !t.kind.can_be_identifier() {
                return false;
            }
            if let Some(id) = target_id {
                // Precise: only tokens whose symbol_id matches the target.
                resolve_symbol_id(state, t.offset) == Some(id)
            } else {
                // No symbol_id: cannot rename safely, skip.
                false
            }
        })
        .map(|t| TextEdit {
            range: range_on_line(t.line, t.col, t.col + t.length),
            new_text: new_name.clone(),
        })
        .collect();

    if edits.is_empty() {
        return None;
    }

    let url = Url::parse(&state.uri).ok()?;
    let mut changes = std::collections::HashMap::new();
    changes.insert(url, edits);
    Some(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    })
}

fn find_ident_at<'a>(
    state: &'a DocumentState,
    line: u32,
    col: u32,
) -> Option<&'a crate::document::TokenRecord> {
    state.tokens.iter().find(|t| {
        t.line == line
            && t.col <= col
            && col < t.col + t.length
            && (t.kind == TokenKind::Identifier || t.kind.can_be_identifier())
    })
}

fn resolve_symbol_id(state: &DocumentState, offset: u32) -> Option<SymbolId> {
    state
        .expr_types
        .get(&offset)
        .and_then(|info| info.symbol_id)
}
