use tsn_checker::SymbolKind;

use crate::document::{DocumentState, SymbolRecord};

use super::member::member_at;
use super::token::token_at;

pub fn symbol_at_line(state: &DocumentState, line: u32) -> Option<&SymbolRecord> {
    state.symbols.iter().find(|s| s.line == line)
}

pub fn symbols_named<'a>(state: &'a DocumentState, name: &str) -> Vec<&'a SymbolRecord> {
    state.symbols.iter().filter(|s| s.name == name).collect()
}

pub fn symbol_at(state: &DocumentState, line: u32, col: u32) -> Option<&SymbolRecord> {
    let tok = token_at(state, line, col)?;

    if member_at(state, line, col).is_some() {
        return None;
    }

    if let Some(info) = state.expr_types.get(&tok.offset) {
        if let Some(sid) = info.symbol_id {
            return state.symbols.iter().find(|s| s.symbol_id == Some(sid));
        }
    }

    let mut best: Option<&SymbolRecord> = None;
    for sym in state.symbols.iter().filter(|s| s.name == tok.lexeme) {
        best = Some(match best {
            None => sym,
            Some(prev) => {
                if rank(sym.kind) > rank(prev.kind) {
                    sym
                } else {
                    prev
                }
            }
        });
    }
    best
}

fn rank(k: SymbolKind) -> u8 {
    use SymbolKind::*;
    match k {
        Class | Struct => 0,
        Interface | Enum => 1,
        Function => 2,
        Method => 3,
        Const => 4,
        Var | Let => 5,
        Property | TypeAlias => 6,
        Namespace | Extension => 7,
        TypeParameter => 8,
        Parameter => 9,
    }
}
