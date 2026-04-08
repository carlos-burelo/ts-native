use tower_lsp::lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, Position};
use tsn_checker::SymbolKind;
use tsn_core::TypeKind;

use crate::document::DocumentState;

pub fn build_inlay_hints(state: &DocumentState) -> Vec<InlayHint> {
    let mut hints = Vec::new();

    for s in &state.symbols {
        if s.line == u32::MAX {
            continue;
        }

        match s.kind {
            SymbolKind::Const | SymbolKind::Let | SymbolKind::Var
                if !s.has_explicit_type && !s.type_str.is_empty() =>
            {
                let hint_col = s.col + s.name.len() as u32;
                hints.push(InlayHint {
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
                });
            }

            SymbolKind::Function | SymbolKind::Method => {
                if let Some(hint) = fn_return_hint(state, s) {
                    hints.push(hint);
                }
            }

            _ => {}
        }
    }

    hints
}

fn fn_return_hint(
    state: &DocumentState,
    sym: &crate::document::SymbolRecord,
) -> Option<InlayHint> {
    // Only for functions without explicit return type annotation.
    if sym.has_explicit_type {
        return None;
    }

    // Check if declaration line has `->` before `{` or `=>` — means explicit return type.
    if line_has_explicit_return_annotation(state, sym.line) {
        return None;
    }

    // Extract return type from the symbol's type.
    let ret_ty = match &sym.ty.0 {
        TypeKind::Fn(ft) => ft.return_type.as_ref(),
        _ => return None,
    };

    // Skip void / Unknown / Dynamic.
    match &ret_ty.0 {
        TypeKind::Void | TypeKind::Dynamic => return None,
        _ => {}
    }
    let ret_str = ret_ty.to_string();
    if ret_str.is_empty() || ret_str == "unknown" || ret_str == "void" {
        return None;
    }

    // Position: after the `)` of the parameter list on the declaration line.
    let rparen_col = find_rparen_col_on_line(state, sym.line, sym.col)?;

    Some(InlayHint {
        position: Position {
            line: sym.line,
            character: rparen_col + 1,
        },
        label: InlayHintLabel::String(format!(": {}", ret_str)),
        kind: Some(InlayHintKind::TYPE),
        text_edits: None,
        tooltip: None,
        padding_left: Some(false),
        padding_right: Some(true),
        data: None,
    })
}

fn line_has_explicit_return_annotation(state: &DocumentState, line: u32) -> bool {
    let line_tokens: Vec<_> = state.tokens.iter().filter(|t| t.line == line).collect();
    let mut found_arrow = false;
    let mut found_lbrace = false;
    for tok in &line_tokens {
        if tok.kind == tsn_core::TokenKind::Arrow {
            found_arrow = true;
        }
        if tok.kind == tsn_core::TokenKind::LBrace || tok.kind == tsn_core::TokenKind::FatArrow {
            found_lbrace = true;
        }
    }
    found_arrow && found_lbrace
}

fn find_rparen_col_on_line(state: &DocumentState, line: u32, fn_col: u32) -> Option<u32> {
    // Find the matching `)` for the `(` after the function name on this line.
    let mut depth = 0i32;
    let mut rparen_col = None;
    for tok in state.tokens.iter().filter(|t| t.line == line && t.col >= fn_col) {
        match tok.kind {
            tsn_core::TokenKind::LParen => depth += 1,
            tsn_core::TokenKind::RParen => {
                depth -= 1;
                if depth == 0 {
                    rparen_col = Some(tok.col + tok.length.saturating_sub(1));
                    break;
                }
            }
            _ => {}
        }
    }
    rparen_col
}
