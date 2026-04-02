use std::path::Path;

use tower_lsp::lsp_types::{Hover, HoverContents, LanguageString, MarkedString};
use tsn_checker::SymbolKind;
use tsn_core::TokenKind;

use crate::document::{
    uri_to_path, ChainResult, DocumentState, MemberKind, MemberRecord, SymbolRecord,
};
use crate::query;

pub fn build_hover(state: &DocumentState, line: u32, col: u32) -> Option<Hover> {
    if let Some(ctx) = query::import_path_at(&state.source, line, col) {
        return import_path_hover(&ctx.specifier, &state.uri);
    }

    // Handle `this` keyword — resolve to the innermost enclosing class.
    // (token_at only finds Identifier tokens, so scan directly for This.)
    let tok_any = state
        .tokens
        .iter()
        .find(|t| t.line == line && t.col <= col && col < t.col + t.length);
    if let Some(tok) = tok_any {
        if tok.kind == TokenKind::This {
            let enclosing = state
                .symbols
                .iter()
                .filter(|s| {
                    !s.is_from_stdlib
                        && matches!(s.kind, SymbolKind::Class | SymbolKind::Interface)
                        && s.line <= line
                })
                .max_by_key(|s| s.line);
            if let Some(cls) = enclosing {
                return Some(make_lang_hover(format!("(this) this: {}", cls.name)));
            }
            return Some(make_lang_hover("(this) this".to_owned()));
        }
    }

    if let Some(res) = query::resolve_chain(state, line, col) {
        match res {
            ChainResult::Symbol(sym) => return Some(symbol_hover(sym)),
            ChainResult::Member {
                member,
                parent_name,
            } => {
                return Some(make_lang_hover(format_member_sig(&parent_name, member)));
            }
            ChainResult::DynamicMember {
                member,
                parent_name,
            } => {
                return Some(make_lang_hover(format_member_sig(&parent_name, &member)));
            }
        }
    }

    if let Some((parent_name, parent_kind, member)) = query::member_at(state, line, col) {
        let sig = if parent_kind == SymbolKind::Enum {
            format_enum_member(&parent_name, &member.name, &member.init_value)
        } else {
            format_member_sig(&parent_name, member)
        };
        return Some(make_lang_hover(sig));
    }

    if let Some(sym) = query::symbol_at(state, line, col) {
        return Some(symbol_hover(sym));
    }

    if let Some(param) = query::param_at(state, line, col) {
        let sig = if param.is_type_param {
            format!("(type parameter) {}", param.name)
        } else if param.type_str.is_empty() {
            format!("(param) {}", param.name)
        } else {
            format!("(param) {}: {}", param.name, param.type_str)
        };
        return Some(make_lang_hover(sig));
    }

    None
}

fn make_lang_hover(value: String) -> Hover {
    Hover {
        contents: HoverContents::Array(vec![MarkedString::LanguageString(LanguageString {
            language: "tsn".into(),
            value,
        })]),
        range: None,
    }
}

fn import_path_hover(specifier: &str, doc_uri: &str) -> Option<Hover> {
    let display = if specifier.starts_with("std:") || is_incomplete_import_specifier(specifier) {
        specifier.to_owned()
    } else {
        resolve_import_module_path(specifier, doc_uri)
    };
    Some(make_lang_hover(format!("module \"{}\"", display)))
}

fn is_incomplete_import_specifier(specifier: &str) -> bool {
    let trimmed = specifier.trim();
    trimmed.is_empty() || matches!(trimmed, "." | ".." | "./" | "../") || trimmed.ends_with('/')
}

fn resolve_import_module_path(specifier: &str, doc_uri: &str) -> String {
    let doc_path = uri_to_path(doc_uri);
    let doc_dir = match Path::new(&doc_path).parent() {
        Some(dir) => dir.to_path_buf(),
        None => return specifier.to_owned(),
    };

    let joined = doc_dir.join(specifier);
    let with_ext = if joined.extension().is_none() {
        joined.with_extension("tsn")
    } else {
        joined
    };

    let mut resolved = std::fs::canonicalize(&with_ext).unwrap_or(with_ext);
    if resolved.extension().and_then(|ext| ext.to_str()) == Some("tsn") {
        resolved.set_extension("");
    }

    normalize_display_path(&resolved.to_string_lossy())
}

fn normalize_display_path(path: &str) -> String {
    let without_verbatim = path
        .strip_prefix(r"\\?\")
        .or_else(|| path.strip_prefix("//?/"))
        .unwrap_or(path);
    without_verbatim.replace('\\', "/")
}

pub fn symbol_hover(sym: &SymbolRecord) -> Hover {
    let mut items = vec![MarkedString::LanguageString(LanguageString {
        language: "tsn".into(),
        value: format_signature(sym),
    })];

    if let Some(raw) = &sym.doc {
        let parsed = tsn_core::DocComment::parse(raw);
        let md = parsed.to_markdown();
        if !md.is_empty() {
            items.push(MarkedString::String(md));
        }
    }

    Hover {
        contents: HoverContents::Array(items),
        range: None,
    }
}

pub fn format_member_sig(parent_name: &str, member: &MemberRecord) -> String {
    if member.type_str.contains("Enum") {
        return format_enum_member(parent_name, &member.name, &member.init_value);
    }
    let is_type_like = matches!(
        member.kind,
        MemberKind::Class
            | MemberKind::Interface
            | MemberKind::Namespace
            | MemberKind::Enum
            | MemberKind::Struct
    );
    let static_kw = if member.is_static && !is_type_like {
        "static "
    } else {
        ""
    };
    match member.kind {
        MemberKind::Property | MemberKind::Getter => {
            format!(
                "(property) {}{}.{}: {}",
                static_kw, parent_name, member.name, member.type_str
            )
        }
        MemberKind::Setter => {
            format!(
                "(property) {}{}.{}({})",
                static_kw, parent_name, member.name, member.params_str
            )
        }
        MemberKind::Constructor => {
            format!("(constructor) {}({})", parent_name, member.params_str)
        }
        MemberKind::Method => {
            if member.is_arrow {
                format!(
                    "(property) {}{}.{}: {}",
                    static_kw, parent_name, member.name, member.type_str
                )
            } else {
                format!(
                    "(method) {}{}.{}({}): {}",
                    static_kw, parent_name, member.name, member.params_str, member.type_str
                )
            }
        }
        MemberKind::Class => format_nested_class(parent_name, member),
        MemberKind::Interface => format_nested_interface(parent_name, member),
        MemberKind::Namespace => format_nested_namespace(parent_name, member),
        MemberKind::Enum => {
            format!("(enum) {}{}.{}", static_kw, parent_name, member.name)
        }
        MemberKind::Struct => {
            format!("(struct) {}{}.{}", static_kw, parent_name, member.name)
        }
    }
}

pub fn format_enum_member(enum_name: &str, member_name: &str, init_value: &str) -> String {
    if init_value.is_empty() {
        format!("(enum member) {}.{}", enum_name, member_name)
    } else {
        format!(
            "(enum member) {}.{} = {}",
            enum_name, member_name, init_value
        )
    }
}

fn format_nested_class(parent_name: &str, m: &MemberRecord) -> String {
    let tp = format_type_params_str(&m.ty);
    if m.members.is_empty() {
        return format!("(class) {}.{}{}", parent_name, m.name, tp);
    }
    let mut lines = vec![format!("(class) {}.{}{} {{", parent_name, m.name, tp)];
    for inner in &m.members {
        lines.push(format_inner_member(inner));
    }
    lines.push("}".to_owned());
    lines.join("\n")
}

fn format_nested_interface(parent_name: &str, m: &MemberRecord) -> String {
    let tp = format_type_params_str(&m.ty);
    if m.members.is_empty() {
        return format!("(interface) {}.{}{}", parent_name, m.name, tp);
    }
    let mut lines = vec![format!("(interface) {}.{}{} {{", parent_name, m.name, tp)];
    for inner in &m.members {
        lines.push(format_inner_member(inner));
    }
    lines.push("}".to_owned());
    lines.join("\n")
}

fn format_nested_namespace(parent_name: &str, m: &MemberRecord) -> String {
    if m.members.is_empty() {
        return format!("(namespace) {}.{}", parent_name, m.name);
    }
    let mut lines = vec![format!("(namespace) {}.{} {{", parent_name, m.name)];
    for inner in &m.members {
        lines.push(format_inner_member(inner));
    }
    lines.push("}".to_owned());
    lines.join("\n")
}

fn format_inner_member(m: &MemberRecord) -> String {
    let indent = "  ";
    let static_prefix = if m.is_static { "static " } else { "" };
    match m.kind {
        MemberKind::Constructor => format!("{}constructor({})", indent, m.params_str),
        MemberKind::Method => format!(
            "{}{}{}({}): {}",
            indent, static_prefix, m.name, m.params_str, m.type_str
        ),
        MemberKind::Property => format!("{}{}{}: {}", indent, static_prefix, m.name, m.type_str),
        MemberKind::Getter => format!(
            "{}{}get {}(): {}",
            indent, static_prefix, m.name, m.type_str
        ),
        MemberKind::Setter
        | MemberKind::Class
        | MemberKind::Interface
        | MemberKind::Namespace
        | MemberKind::Enum
        | MemberKind::Struct => {
            format!(
                "{}{}{} {}({})",
                indent,
                static_prefix,
                m.kind.kind_label(),
                m.name,
                m.params_str
            )
        }
    }
}

fn format_type_params_str(ty: &tsn_checker::Type) -> String {
    match &ty.0 {
        tsn_core::TypeKind::Generic(_, args, _) => {
            let names: Vec<String> = args.iter().map(|a| a.to_string()).collect();
            format!("<{}>", names.join(", "))
        }
        _ => String::new(),
    }
}

impl MemberKind {
    fn kind_label(&self) -> &'static str {
        match self {
            MemberKind::Class => "class",
            MemberKind::Interface => "interface",
            MemberKind::Namespace => "namespace",
            MemberKind::Enum => "enum",
            MemberKind::Struct => "struct",
            MemberKind::Property => "prop",
            MemberKind::Method => "method",
            MemberKind::Getter => "get",
            MemberKind::Setter => "set",
            MemberKind::Constructor => "constructor",
        }
    }
}

pub fn format_type_params(type_params: &[String]) -> String {
    if type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", type_params.join(", "))
    }
}

fn format_signature(sym: &SymbolRecord) -> String {
    match sym.kind {
        SymbolKind::Function | SymbolKind::Method => format_fn(sym),
        SymbolKind::Class => format_class(sym),
        SymbolKind::Struct => format!("struct {}", sym.name),
        SymbolKind::Interface => format_interface(sym),
        SymbolKind::TypeAlias => format!("type {} = {}", sym.name, sym.type_str),
        SymbolKind::Enum => format!("enum {}", sym.name),
        SymbolKind::Namespace => format_namespace(sym),
        SymbolKind::Const => format_binding("const", sym),
        SymbolKind::Let => format_binding("let", sym),
        SymbolKind::Var => format_binding("var", sym),
        SymbolKind::Parameter => format_binding("(param)", sym),
        SymbolKind::Property => format_binding("prop", sym),
        SymbolKind::Extension => format!("extension {}", sym.name),
        SymbolKind::TypeParameter => format!("type {}", sym.name),
    }
}

fn format_class(sym: &SymbolRecord) -> String {
    let tp = format_type_params(&sym.type_params);
    if sym.members.is_empty() {
        return format!("class {}{}", sym.name, tp);
    }
    let mut lines = vec![format!("class {}{} {{", sym.name, tp)];
    for m in &sym.members {
        lines.push(format_inner_member(m));
    }
    lines.push("}".to_owned());
    lines.join("\n")
}

fn format_interface(sym: &SymbolRecord) -> String {
    let tp = format_type_params(&sym.type_params);
    if sym.members.is_empty() {
        return format!("interface {}{}", sym.name, tp);
    }
    let mut lines = vec![format!("interface {}{} {{", sym.name, tp)];
    for m in &sym.members {
        lines.push(format_inner_member(m));
    }
    lines.push("}".to_owned());
    lines.join("\n")
}

fn format_namespace(sym: &SymbolRecord) -> String {
    if sym.members.is_empty() {
        return format!("namespace {}", sym.name);
    }
    let mut lines = vec![format!("namespace {} {{", sym.name)];
    for m in &sym.members {
        lines.push(format_inner_member(m));
    }
    lines.push("}".to_owned());
    lines.join("\n")
}

fn format_fn(sym: &SymbolRecord) -> String {
    let async_prefix = if sym.is_async { "async " } else { "" };
    let kw = match sym.kind {
        SymbolKind::Method => "method",
        _ => "function",
    };
    let tp = format_type_params(&sym.type_params);
    if sym.is_arrow {
        return format!(
            "{}{} {}{}: {}",
            async_prefix, kw, sym.name, tp, sym.type_str
        );
    }
    format!(
        "{}{} {}{}({}): {}",
        async_prefix, kw, sym.name, tp, sym.params_str, sym.type_str
    )
}

fn format_binding(keyword: &str, sym: &SymbolRecord) -> String {
    if sym.type_str.is_empty() {
        format!("{} {}", keyword, sym.name)
    } else {
        format!("{} {}: {}", keyword, sym.name, sym.type_str)
    }
}
