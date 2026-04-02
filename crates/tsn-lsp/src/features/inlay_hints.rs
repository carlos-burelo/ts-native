use tower_lsp::lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, Position};
use tsn_checker::SymbolKind;

use crate::document::DocumentState;

pub fn build_inlay_hints(state: &DocumentState) -> Vec<InlayHint> {
    state
        .symbols
        .iter()
        .filter(|s| {
            matches!(
                s.kind,
                SymbolKind::Const | SymbolKind::Let | SymbolKind::Var
            ) && s.line != u32::MAX
                && !s.has_explicit_type
                && !s.type_str.is_empty()
        })
        .map(|s| {
            let hint_col = s.col + s.name.len() as u32;
            InlayHint {
                position: Position {
                    line: s.line,
                    character: hint_col,
                },
                label: InlayHintLabel::String(format!(": {}", s.type_str)),
                kind: Some(InlayHintKind::TYPE),
                text_edits: None,
                tooltip: None,
                padding_left: Some(false),
                padding_right: Some(true),
                data: None,
            }
        })
        .collect()
}
