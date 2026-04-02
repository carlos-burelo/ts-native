use crate::document::{DocumentState, MemberKind, MemberRecord};
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat};
use tsn_core::TokenKind;

pub enum ReceiverInfo {
    Named(String, bool),

    Anonymous(Vec<MemberRecord>),
}

pub fn build_member_completions(
    state: &DocumentState,
    info: ReceiverInfo,
    use_snippets: bool,
) -> Vec<CompletionItem> {
    match info {
        ReceiverInfo::Named(name, is_instance) => {
            let sym = state.symbols.iter().find(|s| s.name == name);
            let mut items = Vec::new();
            let mut seen = std::collections::HashSet::new();

            for m in sym
                .map(|s| s.members.as_slice())
                .unwrap_or_default()
                .iter()
                .filter(|m| m.is_static != is_instance)
                .filter(|m| m.kind != MemberKind::Constructor)
            {
                if seen.insert(m.name.clone()) {
                    items.push(member_to_completion_item(m, use_snippets));
                }
            }

            if is_instance {
                if let Some(exts) = state.extension_members.get(&name) {
                    for m in exts {
                        if seen.insert(m.name.clone()) {
                            items.push(member_to_completion_item(m, use_snippets));
                        }
                    }
                }
            }

            items
        }
        ReceiverInfo::Anonymous(members) => members
            .into_iter()
            .filter(|m| m.kind != MemberKind::Constructor)
            .map(|m| member_to_completion_item(&m, use_snippets))
            .collect(),
    }
}

fn member_to_completion_item(m: &MemberRecord, use_snippets: bool) -> CompletionItem {
    let (item_kind, insert_text, insert_text_format) = match m.kind {
        MemberKind::Property | MemberKind::Getter | MemberKind::Setter => {
            (CompletionItemKind::PROPERTY, m.name.clone(), None)
        }
        MemberKind::Method => {
            if use_snippets {
                (
                    CompletionItemKind::METHOD,
                    format!("{}($0)", m.name),
                    Some(InsertTextFormat::SNIPPET),
                )
            } else {
                (CompletionItemKind::METHOD, m.name.clone(), None)
            }
        }
        MemberKind::Constructor => (CompletionItemKind::CONSTRUCTOR, m.name.clone(), None),
        MemberKind::Class => (CompletionItemKind::CLASS, m.name.clone(), None),
        MemberKind::Interface => (CompletionItemKind::INTERFACE, m.name.clone(), None),
        MemberKind::Namespace => (CompletionItemKind::MODULE, m.name.clone(), None),
        MemberKind::Enum => (CompletionItemKind::ENUM, m.name.clone(), None),
        MemberKind::Struct => (CompletionItemKind::STRUCT, m.name.clone(), None),
    };
    let detail = match m.kind {
        MemberKind::Property | MemberKind::Getter => Some(m.type_str.clone()),
        MemberKind::Setter => Some(format!("({})", m.params_str)),
        MemberKind::Constructor => Some(format!("({})", m.params_str)),
        MemberKind::Method => Some(format!("({}): {}", m.params_str, m.type_str)),
        MemberKind::Class => Some("class".to_owned()),
        MemberKind::Interface => Some("interface".to_owned()),
        MemberKind::Namespace => Some("namespace".to_owned()),
        MemberKind::Enum => Some("enum".to_owned()),
        MemberKind::Struct => Some("struct".to_owned()),
    };
    CompletionItem {
        label: m.name.clone(),
        kind: Some(item_kind),
        detail,
        insert_text: Some(insert_text),
        insert_text_format,
        ..Default::default()
    }
}

pub fn dot_receiver(
    state: &DocumentState,
    line: u32,
    col: u32,
    trigger_char: Option<&str>,
) -> Option<ReceiverInfo> {
    let line_toks: Vec<_> = state.tokens.iter().filter(|t| t.line == line).collect();

    let dot_idx = line_toks
        .iter()
        .rposition(|t| t.kind == TokenKind::Dot && t.col < col);

    if dot_idx.is_none() {
        if trigger_char == Some(".") {
            return dot_receiver_source_fallback(state, line, col);
        }
        return None;
    }
    let dot_idx = dot_idx?;

    if dot_idx == 0 {
        return None;
    }

    let before = line_toks[dot_idx - 1];

    if before.kind == TokenKind::This {
        let enclosing = state
            .symbols
            .iter()
            .filter(|s| {
                !s.is_from_stdlib
                    && matches!(
                        s.kind,
                        tsn_checker::SymbolKind::Class | tsn_checker::SymbolKind::Interface
                    )
                    && s.line <= line
            })
            .max_by_key(|s| s.line);
        if let Some(cls) = enclosing {
            return Some(ReceiverInfo::Named(cls.name.clone(), true));
        }
        return None;
    }

    use tsn_core::well_known as wk;
    let literal_type: Option<&str> = match before.kind {
        TokenKind::Str => Some(wk::STR),
        TokenKind::IntegerLiteral
        | TokenKind::HexLiteral
        | TokenKind::BinaryLiteral
        | TokenKind::OctalLiteral => Some(wk::INT),
        TokenKind::FloatLiteral => Some(wk::FLOAT),
        TokenKind::DecimalLiteral => Some(wk::DECIMAL),
        TokenKind::BigIntLiteral => Some(wk::BIGINT),
        TokenKind::True | TokenKind::False => Some(wk::BOOL),
        TokenKind::Char => Some(wk::CHAR),
        _ => None,
    };
    if let Some(prim) = literal_type {
        return Some(ReceiverInfo::Named(prim.to_owned(), true));
    }

    if before.kind == TokenKind::Identifier {
        let sym = state.symbols.iter().find(|s| s.name == before.lexeme);
        if let Some(sym) = sym {
            match sym.kind {
                tsn_checker::SymbolKind::Class
                | tsn_checker::SymbolKind::Namespace
                | tsn_checker::SymbolKind::Interface
                | tsn_checker::SymbolKind::Enum => {
                    return Some(ReceiverInfo::Named(sym.name.clone(), false));
                }
                tsn_checker::SymbolKind::Let
                | tsn_checker::SymbolKind::Var
                | tsn_checker::SymbolKind::Const
                | tsn_checker::SymbolKind::Parameter => {
                    if !sym.members.is_empty() {
                        return Some(ReceiverInfo::Anonymous(sym.members.clone()));
                    }
                    if !sym.type_str.is_empty() {
                        if sym.type_str.ends_with("[]") {
                            return Some(ReceiverInfo::Named(
                                tsn_core::well_known::ARRAY.to_owned(),
                                true,
                            ));
                        }
                        let class_name = sym.type_str.split('<').next().unwrap_or("").trim();
                        if !class_name.is_empty() {
                            return Some(ReceiverInfo::Named(class_name.to_owned(), true));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if let Some(info) = state.expr_types.get(&before.offset) {
        match &info.ty.0 {
            tsn_core::TypeKind::Object(members) => {
                let recs = members
                    .iter()
                    .filter_map(|m| type_member_to_record(m))
                    .collect();
                return Some(ReceiverInfo::Anonymous(recs));
            }
            tsn_core::TypeKind::Array(_) => {
                return Some(ReceiverInfo::Named(
                    tsn_core::well_known::ARRAY.to_owned(),
                    true,
                ));
            }
            tsn_core::TypeKind::Named(name, _) | tsn_core::TypeKind::Generic(name, ..) => {
                return Some(ReceiverInfo::Named(name.clone(), true));
            }
            tsn_core::TypeKind::Int => return Some(ReceiverInfo::Named(wk::INT.to_owned(), true)),
            tsn_core::TypeKind::Float => {
                return Some(ReceiverInfo::Named(wk::FLOAT.to_owned(), true))
            }
            tsn_core::TypeKind::Str => return Some(ReceiverInfo::Named(wk::STR.to_owned(), true)),
            tsn_core::TypeKind::Bool => {
                return Some(ReceiverInfo::Named(wk::BOOL.to_owned(), true))
            }
            tsn_core::TypeKind::Char => {
                return Some(ReceiverInfo::Named(wk::CHAR.to_owned(), true))
            }
            tsn_core::TypeKind::Decimal => {
                return Some(ReceiverInfo::Named(wk::DECIMAL.to_owned(), true))
            }
            tsn_core::TypeKind::BigInt => {
                return Some(ReceiverInfo::Named(wk::BIGINT.to_owned(), true))
            }
            _ => {}
        }
    }

    None
}

fn type_member_to_record(m: &tsn_checker::types::ObjectTypeMember) -> Option<MemberRecord> {
    use tsn_checker::types::ObjectTypeMember::*;
    match m {
        Property { name, ty, .. } => Some(MemberRecord {
            name: name.clone(),
            type_str: ty.to_string(),
            params_str: String::new(),
            is_static: false,
            is_optional: false,
            kind: MemberKind::Property,
            is_arrow: false,
            line: 0,
            col: 0,
            init_value: String::new(),
            ty: ty.clone(),
            members: Vec::new(),
        }),
        Method {
            name,
            params,
            return_type,
            ..
        } => {
            let params_str = params
                .iter()
                .map(|p| {
                    format!(
                        "{}: {}",
                        p.name.as_deref().unwrap_or("arg"),
                        p.ty.to_string()
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            Some(MemberRecord {
                name: name.clone(),
                type_str: return_type.to_string(),
                params_str,
                is_static: false,
                is_optional: false,
                kind: MemberKind::Method,
                is_arrow: false,
                line: 0,
                col: 0,
                init_value: String::new(),
                ty: tsn_checker::Type(tsn_core::TypeKind::Fn(tsn_checker::types::FunctionType {
                    params: params.clone(),
                    return_type: return_type.clone(),
                    is_arrow: false,
                    type_params: Vec::new(),
                })),
                members: Vec::new(),
            })
        }
        _ => None,
    }
}

fn dot_receiver_source_fallback(
    state: &DocumentState,
    line: u32,
    col: u32,
) -> Option<ReceiverInfo> {
    if col < 2 {
        return None;
    }
    let src_line = state.source.lines().nth(line as usize)?;
    let id_end = (col as usize)
        .saturating_sub(2)
        .min(src_line.len().saturating_sub(1));

    let bytes = src_line.as_bytes();
    if id_end >= bytes.len() {
        return None;
    }
    if !bytes[id_end].is_ascii_alphanumeric() && bytes[id_end] != b'_' {
        return None;
    }

    let mut start = id_end;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }
    let name = &src_line[start..=id_end];
    if name.is_empty() {
        return None;
    }

    let sym = state.symbols.iter().find(|s| s.name == name)?;
    match sym.kind {
        tsn_checker::SymbolKind::Class
        | tsn_checker::SymbolKind::Namespace
        | tsn_checker::SymbolKind::Interface
        | tsn_checker::SymbolKind::Enum => Some(ReceiverInfo::Named(sym.name.clone(), false)),

        tsn_checker::SymbolKind::Let
        | tsn_checker::SymbolKind::Var
        | tsn_checker::SymbolKind::Const
        | tsn_checker::SymbolKind::Parameter => {
            if !sym.members.is_empty() {
                return Some(ReceiverInfo::Anonymous(sym.members.clone()));
            }
            if sym.type_str.is_empty() {
                return None;
            }
            if sym.type_str.ends_with("[]") {
                return Some(ReceiverInfo::Named(
                    tsn_core::well_known::ARRAY.to_owned(),
                    true,
                ));
            }
            let class_name = sym.type_str.split('<').next()?.trim();
            state
                .symbols
                .iter()
                .find(|s| {
                    s.name == class_name
                        && !s.members.is_empty()
                        && matches!(
                            s.kind,
                            tsn_checker::SymbolKind::Class
                                | tsn_checker::SymbolKind::Interface
                                | tsn_checker::SymbolKind::Namespace
                        )
                })
                .map(|s| ReceiverInfo::Named(s.name.clone(), true))
        }

        _ => None,
    }
}
pub fn pattern_receiver(state: &DocumentState, line: u32, col: u32) -> Option<ReceiverInfo> {
    let line_toks: Vec<_> = state.tokens.iter().filter(|t| t.line == line).collect();

    let _brace_open_idx = line_toks
        .iter()
        .rposition(|t| t.kind == TokenKind::LBrace && t.col < col)?;

    let eq_idx = line_toks
        .iter()
        .position(|t| t.kind == TokenKind::Eq && t.col >= col)?;

    let rhs_idx = eq_idx + 1;
    if rhs_idx >= line_toks.len() {
        return None;
    }
    let rhs_tok = line_toks[rhs_idx];

    if let Some(info) = state.expr_types.get(&rhs_tok.offset) {
        match &info.ty.0 {
            tsn_core::TypeKind::Object(members) => {
                let recs = members
                    .iter()
                    .filter_map(|m| type_member_to_record(m))
                    .collect();
                return Some(ReceiverInfo::Anonymous(recs));
            }
            tsn_core::TypeKind::Named(name, _) | tsn_core::TypeKind::Generic(name, ..) => {
                return Some(ReceiverInfo::Named(name.clone(), true));
            }
            _ => {}
        }
    } else {
        if rhs_tok.kind == TokenKind::Identifier {
            let sym = state.symbols.iter().find(|s| s.name == rhs_tok.lexeme);
            if let Some(sym) = sym {
                if !sym.members.is_empty() {
                    return Some(ReceiverInfo::Anonymous(sym.members.clone()));
                }
                if !sym.type_str.is_empty() {
                    let class_name = sym.type_str.split('<').next().unwrap_or("").trim();
                    if !class_name.is_empty() {
                        return Some(ReceiverInfo::Named(class_name.to_owned(), true));
                    }
                }
            }
        }
    }

    None
}
