use std::collections::HashMap;

use super::format::extract_enum_init_value;
use crate::document::{MemberKind, MemberRecord, SymbolRecord, TokenRecord};
use tsn_checker::types::FunctionType;
use tsn_checker::{BindResult, ClassMemberInfo, ClassMemberKind, SymbolKind};
use tsn_core::TypeKind;

pub fn map_members(ms: &[ClassMemberInfo], tokens: &[TokenRecord]) -> Vec<MemberRecord> {
    map_members_inner(ms, tokens, false)
}

pub fn map_enum_members(ms: &[ClassMemberInfo], tokens: &[TokenRecord]) -> Vec<MemberRecord> {
    map_members_inner(ms, tokens, true)
}

fn map_members_inner(
    ms: &[ClassMemberInfo],
    tokens: &[TokenRecord],
    is_enum: bool,
) -> Vec<MemberRecord> {
    ms.iter()
        .map(|m| {
            let type_str = if let TypeKind::Fn(ft) = &m.ty.0 {
                if !ft.is_arrow {
                    ft.return_type.to_string()
                } else {
                    m.ty.to_string()
                }
            } else if matches!(
                m.kind,
                ClassMemberKind::Class
                    | ClassMemberKind::Namespace
                    | ClassMemberKind::Interface
                    | ClassMemberKind::Enum
                    | ClassMemberKind::Struct
            ) {
                m.name.clone()
            } else {
                m.ty.to_string()
            };

            MemberRecord {
                name: m.name.clone(),
                type_str,
                params_str: m.params_str(),
                is_static: m.is_static,
                is_optional: m.is_optional,
                kind: if is_enum && m.kind == ClassMemberKind::Property {
                    MemberKind::EnumMember
                } else {
                    class_member_kind(&m.kind)
                },
                is_arrow: if let TypeKind::Fn(ft) = &m.ty.0 {
                    ft.is_arrow
                } else {
                    false
                },
                line: m.line,
                col: m.col,
                init_value: if m.kind == ClassMemberKind::Property
                    && m.return_type_str().contains("Enum")
                {
                    extract_enum_init_value(tokens, m.line)
                } else {
                    String::new()
                },
                ty: m.ty.clone(),
                members: map_members(&m.members, tokens),
            }
        })
        .collect()
}

pub fn class_member_kind(k: &ClassMemberKind) -> MemberKind {
    match k {
        ClassMemberKind::Constructor => MemberKind::Constructor,
        ClassMemberKind::Method => MemberKind::Method,
        ClassMemberKind::Property => MemberKind::Property,
        ClassMemberKind::Getter => MemberKind::Getter,
        ClassMemberKind::Setter => MemberKind::Setter,
        ClassMemberKind::Class => MemberKind::Class,
        ClassMemberKind::Interface => MemberKind::Interface,
        ClassMemberKind::Namespace => MemberKind::Namespace,
        ClassMemberKind::Enum => MemberKind::Enum,
        ClassMemberKind::Struct => MemberKind::Struct,
    }
}

pub fn inject_stdlib_symbols(
    symbols: &mut Vec<SymbolRecord>,
    symbol_map: &mut HashMap<String, SymbolKind>,
    bind: &BindResult,
) {
    for sym in bind.global_symbols() {
        if sym.origin_module.is_none() || symbol_map.contains_key(&sym.name) {
            continue;
        }

        let inferred_ty = sym.ty.clone().unwrap_or(tsn_checker::types::Type::Dynamic);
        let type_str = if let TypeKind::Fn(FunctionType {
            return_type,
            is_arrow: false,
            ..
        }) = &inferred_ty.0
        {
            return_type.to_string()
        } else {
            inferred_ty.to_string()
        };

        symbols.push(SymbolRecord {
            name: sym.name.clone(),
            kind: sym.kind,
            type_str,
            params_str: super::format::format_type_params(&inferred_ty),
            line: sym.line.saturating_sub(1),
            col: sym.col,
            has_explicit_type: sym.has_explicit_type,
            is_async: sym.is_async,
            is_arrow: matches!(
                &inferred_ty.0,
                TypeKind::Fn(FunctionType { is_arrow: true, .. })
            ),
            doc: sym.doc.clone(),
            members: Vec::new(),
            type_params: sym.type_params.clone(),
            ty: inferred_ty,
            symbol_id: None,
            full_range: sym.full_range.clone(),
            is_from_stdlib: true,
        });
        symbol_map.insert(sym.name.clone(), sym.kind);
    }
}
