mod classify;

use std::collections::HashMap;

use once_cell::sync::Lazy;
use tower_lsp::lsp_types::{SemanticTokenModifier, SemanticTokenType, SemanticTokensLegend};
use tsn_checker::SymbolKind;
use tsn_core::TokenKind;

use crate::document::DocumentState;
use crate::query;

pub static LEGEND: Lazy<SemanticTokensLegend> = Lazy::new(|| SemanticTokensLegend {
    token_types: vec![
        SemanticTokenType::KEYWORD,
        SemanticTokenType::TYPE,
        SemanticTokenType::VARIABLE,
        SemanticTokenType::FUNCTION,
        SemanticTokenType::CLASS,
        SemanticTokenType::PARAMETER,
        SemanticTokenType::PROPERTY,
        SemanticTokenType::NUMBER,
        SemanticTokenType::STRING,
        SemanticTokenType::ENUM_MEMBER,
    ],
    token_modifiers: vec![
        SemanticTokenModifier::DECLARATION,
        SemanticTokenModifier::READONLY,
        SemanticTokenModifier::ASYNC,
    ],
});

pub const TT_KEYWORD: u32 = 0;
pub const TT_TYPE: u32 = 1;
pub const TT_VARIABLE: u32 = 2;
pub const TT_FUNCTION: u32 = 3;
pub const TT_CLASS: u32 = 4;
pub const TT_PARAMETER: u32 = 5;
pub const TT_PROPERTY: u32 = 6;
pub const TT_NUMBER: u32 = 7;
pub const TT_STRING: u32 = 8;
pub const TT_ENUM_MEMBER: u32 = 9;

pub fn build_semantic_tokens(state: &DocumentState) -> Vec<u32> {
    use crate::document::MemberKind;

    let mut member_overrides: HashMap<(u32, String), u32> = HashMap::new();
    for sym in &state.symbols {
        if matches!(
            sym.kind,
            SymbolKind::Class | SymbolKind::Interface | SymbolKind::Enum
        ) {
            for member in &sym.members {
                if member.line == u32::MAX {
                    continue;
                }
                let tt = if sym.kind == SymbolKind::Enum {
                    TT_ENUM_MEMBER
                } else {
                    match member.kind {
                        MemberKind::Property => TT_PROPERTY,
                        MemberKind::Method | MemberKind::Getter | MemberKind::Setter => TT_FUNCTION,
                        MemberKind::Constructor => continue,
                        MemberKind::Class | MemberKind::Namespace | MemberKind::Struct => TT_CLASS,
                        MemberKind::Interface | MemberKind::Enum => TT_TYPE,
                        MemberKind::EnumMember => TT_ENUM_MEMBER,
                    }
                };
                member_overrides.insert((member.line, member.name.clone()), tt);
            }
        }
    }

    let tokens = &state.tokens;
    let mut result = Vec::with_capacity(tokens.len() * 5);
    let mut prev_line: u32 = 0;
    let mut prev_col: u32 = 0;
    let mut paren_depth: i32 = 0;

    for (i, tok) in tokens.iter().enumerate() {
        let next_is_lparen = tokens
            .get(i + 1)
            .map(|t| t.kind == TokenKind::LParen)
            .unwrap_or(false);
        let prev_is_dot = i
            .checked_sub(1)
            .and_then(|j| tokens.get(j))
            .map(|t| t.kind == TokenKind::Dot)
            .unwrap_or(false);
        let next_is_colon = tokens
            .get(i + 1)
            .map(|t| t.kind == TokenKind::Colon)
            .unwrap_or(false);

        let mut token_type = None;

        if token_type.is_none() {
            token_type = member_overrides
                .get(&(tok.line, tok.lexeme.clone()))
                .copied();
        }

        if token_type.is_none() {
            if let Some(res) = query::resolve_chain(state, tok.line, tok.col) {
                token_type = Some(match res {
                    crate::document::ChainResult::Symbol(s) => match s.kind {
                        SymbolKind::Function | SymbolKind::Method => TT_FUNCTION,
                        SymbolKind::Class
                        | SymbolKind::Struct
                        | SymbolKind::Namespace
                        | SymbolKind::Extension => TT_CLASS,
                        SymbolKind::Interface
                        | SymbolKind::TypeAlias
                        | SymbolKind::Enum
                        | SymbolKind::TypeParameter => TT_TYPE,
                        SymbolKind::Parameter => TT_PARAMETER,
                        SymbolKind::Property => TT_PROPERTY,
                        SymbolKind::Const | SymbolKind::Let | SymbolKind::Var => {
                            if matches!(s.ty.0, tsn_core::TypeKind::Fn(_)) {
                                TT_FUNCTION
                            } else {
                                TT_VARIABLE
                            }
                        }
                    },
                    crate::document::ChainResult::Member { member, .. } => {
                        classify::map_member_kind_to_tt(&member.kind)
                    }
                    crate::document::ChainResult::DynamicMember { member, .. } => {
                        classify::map_member_kind_to_tt(&member.kind)
                    }
                });
            }
        }

        if token_type.is_none() {
            if let Some((_, _, m)) = query::member_at(state, tok.line, tok.col) {
                token_type = Some(classify::map_member_kind_to_tt(&m.kind));
            }
        }

        if token_type.is_none() {
            token_type = if tok.kind == TokenKind::Identifier
                && !prev_is_dot
                && !next_is_lparen
                && paren_depth > 0
                && next_is_colon
            {
                Some(TT_PARAMETER)
            } else if tok.kind == TokenKind::Identifier
                && !prev_is_dot
                && !next_is_lparen
                && paren_depth == 0
                && next_is_colon
            {
                Some(TT_PROPERTY)
            } else if tok.kind == TokenKind::Identifier
                && !prev_is_dot
                && !next_is_lparen
                && paren_depth == 0
                && classify::param_scope_type(tok, &state.param_scopes)
            {
                Some(TT_PARAMETER)
            } else {
                classify::classify(
                    tok,
                    &state.symbol_map,
                    next_is_lparen,
                    prev_is_dot,
                    &state.type_param_names,
                )
            };
        }

        match tok.kind {
            TokenKind::LParen => paren_depth += 1,
            TokenKind::RParen if paren_depth > 0 => paren_depth -= 1,
            _ => {}
        }

        if token_type.is_some()
            && (tok.kind != TokenKind::Identifier
                && !tok.kind.is_keyword()
                && !tok.kind.is_literal()
                && !matches!(
                    tok.kind,
                    TokenKind::Arrow | TokenKind::FatArrow | TokenKind::PipeGt
                ))
        {
            token_type = None;
        }

        let Some(token_type) = token_type else {
            continue;
        };

        // For template head/middle/tail, trim the `${` and `}` delimiters from the
        // emitted range so only the literal string content gets the string color.
        // TemplateHead   `literal${  → emit from col,    length-2  (drop `${`)
        // TemplateMiddle }literal${  → emit from col+1,  length-3  (drop `}` and `${`)
        // TemplateTail   }literal`   → emit from col+1,  length-1  (drop `}`)
        let (emit_col, emit_len) = match tok.kind {
            TokenKind::TemplateHead => (tok.col, tok.length.saturating_sub(2)),
            TokenKind::TemplateMiddle => (tok.col + 1, tok.length.saturating_sub(3)),
            TokenKind::TemplateTail => (tok.col + 1, tok.length.saturating_sub(1)),
            _ => (tok.col, tok.length),
        };

        if emit_len == 0 {
            continue;
        }

        let modifier: u32 = match tok.kind {
            TokenKind::This => 2,
            TokenKind::Identifier => match state.symbol_map.get(tok.lexeme.as_str()) {
                Some(SymbolKind::Const) => 2,
                _ => 0,
            },
            _ => 0,
        };

        let delta_line = tok.line - prev_line;
        let delta_start = if delta_line == 0 {
            emit_col - prev_col
        } else {
            emit_col
        };

        result.push(delta_line);
        result.push(delta_start);
        result.push(emit_len);
        result.push(token_type);
        result.push(modifier);

        prev_line = tok.line;
        prev_col = emit_col;
    }

    result
}
