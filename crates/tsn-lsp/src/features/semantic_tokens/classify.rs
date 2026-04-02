use std::collections::{HashMap, HashSet};

use tsn_checker::SymbolKind;
use tsn_core::{well_known, TokenKind};

use super::{
    TT_CLASS, TT_FUNCTION, TT_KEYWORD, TT_NUMBER, TT_PARAMETER, TT_PROPERTY, TT_STRING, TT_TYPE,
    TT_VARIABLE,
};
use crate::document::{MemberKind, ParamScope, TokenRecord};

pub fn classify(
    tok: &TokenRecord,
    symbol_map: &HashMap<String, SymbolKind>,
    next_is_lparen: bool,
    prev_is_dot: bool,
    type_param_names: &HashSet<String>,
) -> Option<u32> {
    use TokenKind::*;

    match tok.kind {
        TokenKind::True | TokenKind::False | TokenKind::Null => return Some(TT_NUMBER),
        TokenKind::This => return Some(TT_VARIABLE),
        TokenKind::Void => return Some(TT_TYPE),
        TokenKind::Arrow | FatArrow | PipeGt => return Some(TT_KEYWORD),
        _ => {}
    }

    if tok.kind.is_keyword() {
        if prev_is_dot {
            return Some(if next_is_lparen {
                TT_FUNCTION
            } else {
                TT_PROPERTY
            });
        }
        return Some(TT_KEYWORD);
    }

    match tok.kind {
        IntegerLiteral | FloatLiteral | BinaryLiteral | OctalLiteral | HexLiteral
        | BigIntLiteral | DecimalLiteral => Some(TT_NUMBER),

        Str | Char | Template | TemplateHead | TemplateMiddle | TemplateTail => Some(TT_STRING),

        Identifier => {
            if next_is_lparen {
                if prev_is_dot {
                    Some(TT_FUNCTION)
                } else {
                    Some(match symbol_map.get(tok.lexeme.as_str()) {
                        Some(SymbolKind::Class) | Some(SymbolKind::Struct) => TT_CLASS,
                        _ => TT_FUNCTION,
                    })
                }
            } else if prev_is_dot {
                Some(TT_PROPERTY)
            } else {
                Some(classify_identifier(
                    &tok.lexeme,
                    symbol_map,
                    type_param_names,
                ))
            }
        }

        FatArrow | PipeGt => Some(TT_KEYWORD),

        _ => None,
    }
}

pub fn classify_identifier(
    name: &str,
    symbol_map: &HashMap<String, SymbolKind>,
    type_param_names: &HashSet<String>,
) -> u32 {
    use well_known as wk;
    match name {
        wk::STR
        | wk::INT
        | wk::FLOAT
        | wk::DECIMAL
        | wk::BIGINT
        | wk::CHAR
        | wk::BOOL
        | wk::SYMBOL
        | "object"
        | wk::VOID
        | wk::NEVER
        | wk::DYNAMIC
        | wk::NULL
        | wk::FUTURE
        | wk::RESULT
        | wk::ARRAY => {
            return TT_TYPE;
        }
        _ => {}
    }

    match symbol_map.get(name) {
        Some(SymbolKind::Function) | Some(SymbolKind::Method) => TT_FUNCTION,
        Some(SymbolKind::Class) | Some(SymbolKind::Struct) => TT_CLASS,
        Some(SymbolKind::Interface)
        | Some(SymbolKind::TypeAlias)
        | Some(SymbolKind::Enum)
        | Some(SymbolKind::TypeParameter) => TT_TYPE,
        Some(SymbolKind::Namespace) | Some(SymbolKind::Extension) => TT_CLASS,
        Some(SymbolKind::Property) => TT_PROPERTY,
        Some(SymbolKind::Parameter) => TT_PARAMETER,
        Some(SymbolKind::Const) => TT_VARIABLE,
        Some(SymbolKind::Let | SymbolKind::Var) => TT_VARIABLE,
        None => {
            if type_param_names.contains(name) {
                TT_TYPE
            } else {
                TT_VARIABLE
            }
        }
    }
}

pub fn param_scope_type(tok: &TokenRecord, scopes: &[ParamScope]) -> bool {
    let mut best_range = u32::MAX;
    let mut found = false;
    for scope in scopes {
        if tok.line >= scope.body_start_line && tok.line <= scope.body_end_line {
            let range = scope.body_end_line - scope.body_start_line;
            if range < best_range && scope.params.iter().any(|(n, _)| n == &tok.lexeme) {
                best_range = range;
                found = true;
            }
        }
    }
    found
}

pub fn map_member_kind_to_tt(kind: &MemberKind) -> u32 {
    match kind {
        MemberKind::Method | MemberKind::Getter | MemberKind::Setter => TT_FUNCTION,
        MemberKind::Property => TT_PROPERTY,
        MemberKind::Constructor => TT_KEYWORD,
        MemberKind::Class
        | MemberKind::Interface
        | MemberKind::Namespace
        | MemberKind::Enum
        | MemberKind::Struct => TT_TYPE,
    }
}
