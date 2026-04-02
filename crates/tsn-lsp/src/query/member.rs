use tsn_checker::SymbolKind;

use crate::document::{DocumentState, MemberRecord, MethodHoverInfo};

use super::token::token_at;

pub fn member_at(
    state: &DocumentState,
    line: u32,
    col: u32,
) -> Option<(String, SymbolKind, &MemberRecord)> {
    let tok = token_at(state, line, col)?;

    for sym in &state.symbols {
        if let Some(member) = find_member_at_pos_recursive(&sym.members, line, tok.col, &tok.lexeme)
        {
            let parent_name = find_parent_name_recursive(&sym.members, member)
                .unwrap_or(&sym.name)
                .to_owned();
            return Some((parent_name, sym.kind, member));
        }
    }
    None
}

pub fn members_of<'a>(state: &'a DocumentState, class_name: &str) -> Option<&'a [MemberRecord]> {
    state
        .symbols
        .iter()
        .find(|s| s.name == class_name)
        .map(|s| s.members.as_slice())
}

pub fn method_info_at(state: &DocumentState, line: u32, col: u32) -> Option<MethodHoverInfo> {
    state.method_at_pos(line, col)
}

fn find_member_at_pos_recursive<'a>(
    members: &'a [MemberRecord],
    line: u32,
    col: u32,
    name: &str,
) -> Option<&'a MemberRecord> {
    for m in members {
        if m.line == line && m.name == name && m.col <= col {
            return Some(m);
        }
        if let Some(found) = find_member_at_pos_recursive(&m.members, line, col, name) {
            return Some(found);
        }
    }
    None
}

fn find_parent_name_recursive<'a>(
    members: &'a [MemberRecord],
    target: &MemberRecord,
) -> Option<&'a str> {
    for m in members {
        if m.members.iter().any(|inner| std::ptr::eq(inner, target)) {
            return Some(&m.name);
        }
        if let Some(name) = find_parent_name_recursive(&m.members, target) {
            return Some(name);
        }
    }
    None
}
