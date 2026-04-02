mod format;
mod params;
mod symbols;

use std::collections::HashMap;

use crate::document::{
    uri_to_path, DocumentAnalysis, LspDiag, MemberKind, MemberRecord, SymbolRecord, TokenRecord,
};
use tsn_checker::types::FunctionType;
use tsn_checker::{module_resolver, SymbolKind};
use tsn_core::{DiagnosticKind, TokenKind, TypeKind};

pub fn run_pipeline(source: String, uri: String) -> DocumentAnalysis {
    let path = uri_to_path(&uri);
    let raw_tokens = tsn_lexer::scan(&source, &path);
    let tokens: Vec<TokenRecord> = raw_tokens
        .iter()
        .filter(|t| {
            !matches!(
                t.kind,
                TokenKind::Whitespace | TokenKind::Newline | TokenKind::EOF | TokenKind::DocComment
            )
        })
        .map(|t| TokenRecord {
            kind: t.kind,

            line: t.range.start.line.saturating_sub(1),
            col: t.range.start.column,
            length: t.lexeme.chars().count() as u32,
            offset: t.range.start.offset,
            lexeme: t.lexeme.to_string(),
        })
        .collect();

    let mut diagnostics: Vec<LspDiag> = Vec::new();

    let (program, parse_errs) = tsn_parser::parse_partial(raw_tokens, &path);
    for e in parse_errs {
        diagnostics.push(LspDiag {
            message: e.message,
            line: e.range.start.line.saturating_sub(1),
            col: e.range.start.column,
            end_line: e.range.end.line.saturating_sub(1),
            end_col: e.range.end.column,
            severity: 1,
        });
    }

    let result = tsn_checker::Checker::check(&program);

    for d in &result.diagnostics {
        let severity = match d.kind {
            DiagnosticKind::Error => 1,
            DiagnosticKind::Warning => 2,
            DiagnosticKind::Hint => 3,
        };
        diagnostics.push(LspDiag {
            message: d.message.clone(),
            line: d.range.start.line.saturating_sub(1),
            col: d.range.start.column,
            end_line: d.range.end.line.saturating_sub(1),
            end_col: d.range.end.column,
            severity,
        });
    }

    let sym_records: Vec<SymbolRecord> = result
        .bind
        .arena
        .all()
        .iter()
        .enumerate()
        .map(|(id, sym)| {
            let members = if sym.kind == SymbolKind::Enum {
                result
                    .bind
                    .enum_members
                    .get(&sym.name)
                    .map(|ms| symbols::map_members(ms, &tokens))
                    .or_else(|| {
                        sym.origin_module.as_ref().and_then(|origin| {
                            module_resolver::resolve_module_bind(origin).and_then(|rb| {
                                rb.enum_members
                                    .get(sym.original_name.as_ref().unwrap_or(&sym.name))
                                    .map(|ms| symbols::map_members(ms, &tokens))
                            })
                        })
                    })
                    .unwrap_or_default()
            } else if sym.kind == SymbolKind::Class || sym.kind == SymbolKind::Interface {
                result
                    .flattened_members
                    .get(&sym.name)
                    .map(|ms| symbols::map_members(ms, &tokens))
                    .or_else(|| {
                        sym.origin_module.as_ref().and_then(|origin| {
                            module_resolver::resolve_module_bind(origin).and_then(|rb| {
                                let name = sym.original_name.as_ref().unwrap_or(&sym.name);
                                rb.flattened_members
                                    .get(name)
                                    .or_else(|| rb.class_members.get(name))
                                    .or_else(|| rb.interface_members.get(name))
                                    .map(|ms| symbols::map_members(ms, &tokens))
                            })
                        })
                    })
                    .unwrap_or_default()
            } else if matches!(
                sym.kind,
                SymbolKind::Let | SymbolKind::Var | SymbolKind::Const
            ) {
                result
                    .bind
                    .object_members
                    .get(&sym.name)
                    .map(|ms| symbols::map_members(ms, &tokens))
                    .unwrap_or_default()
            } else if sym.kind == SymbolKind::Namespace {
                result
                    .bind
                    .namespace_members
                    .get(&sym.name)
                    .map(|ms| symbols::map_members(ms, &tokens))
                    .or_else(|| {
                        sym.origin_module.as_ref().and_then(|origin| {
                            module_resolver::resolve_module_bind(origin).and_then(|rb| {
                                rb.namespace_members
                                    .get(sym.original_name.as_ref().unwrap_or(&sym.name))
                                    .map(|ms| symbols::map_members(ms, &tokens))
                            })
                        })
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            let expr_info = result.expr_types.get(&sym.offset);
            // Prefer checker's expr_info (has generic substitution) over binder's sym.ty
            let inferred_ty = match expr_info.map(|i| i.ty.clone()) {
                Some(ct) if !ct.is_dynamic() => Some(ct),
                _ => sym.ty.clone(),
            };
            let symbol_id = expr_info.and_then(|i| i.symbol_id).or(Some(id));

            SymbolRecord {
                name: sym.name.clone(),
                kind: sym.kind,
                type_str: match inferred_ty.as_ref() {
                    Some(t) => {
                        if let TypeKind::Fn(FunctionType {
                            return_type,
                            is_arrow: false,
                            ..
                        }) = &t.0
                        {
                            return_type.to_string()
                        } else {
                            t.to_string()
                        }
                    }
                    None => String::new(),
                },
                params_str: inferred_ty
                    .as_ref()
                    .map(|t| format::format_type_params(t))
                    .unwrap_or_default(),
                line: sym.line.saturating_sub(1),
                col: sym.col,
                has_explicit_type: sym.has_explicit_type,
                is_async: sym.is_async,
                is_arrow: if let Some(TypeKind::Fn(FunctionType { is_arrow, .. })) =
                    inferred_ty.as_ref().map(|t| &t.0)
                {
                    *is_arrow
                } else {
                    false
                },
                doc: sym.doc.clone(),
                members,
                type_params: sym.type_params.clone(),
                ty: inferred_ty.unwrap_or(tsn_checker::types::Type::Dynamic),
                symbol_id,
                full_range: sym.full_range.clone(),
                is_from_stdlib: sym.origin_module.is_some(),
            }
        })
        .collect();

    let mut symbol_map: HashMap<String, SymbolKind> =
        HashMap::with_capacity(sym_records.len() + 64);
    for sym in &sym_records {
        symbol_map.entry(sym.name.clone()).or_insert(sym.kind);
    }

    let mut all_symbols = sym_records;
    symbols::inject_stdlib_symbols(&mut all_symbols, &mut symbol_map, &result.bind);
    let extension_members = build_extension_members(&result.bind);

    let param_scopes = params::collect_param_scopes(&tokens);
    let (type_param_map, type_param_names) = params::collect_type_params(&tokens);
    for sym in &mut all_symbols {
        if sym.line != u32::MAX {
            if let Some(tps) = type_param_map.get(&sym.name) {
                sym.type_params = tps.clone();
            }
        }
    }

    DocumentAnalysis {
        source,
        uri,
        diagnostics,
        symbols: all_symbols,
        tokens,
        symbol_map,
        param_scopes,
        type_param_names,
        flattened_members: result.flattened_members,
        extension_members,
        expr_types: result.expr_types,
    }
}

fn build_extension_members(bind: &tsn_checker::BindResult) -> HashMap<String, Vec<MemberRecord>> {
    let mut out: HashMap<String, Vec<MemberRecord>> = HashMap::new();
    let scope = bind.scopes.get(bind.global_scope);

    build_extension_method_members(bind, scope, &mut out);
    build_extension_accessor_members(bind, scope, &mut out);

    out
}

fn build_extension_method_members(
    bind: &tsn_checker::BindResult,
    scope: &tsn_checker::Scope,
    out: &mut HashMap<String, Vec<MemberRecord>>,
) {
    for (type_name, methods) in &bind.extension_methods {
        let records = out.entry(type_name.clone()).or_default();
        for (method_name, mangled) in methods {
            let Some(sid) = scope.resolve(mangled, &bind.scopes) else {
                continue;
            };
            let sym = bind.arena.get(sid);
            let Some(tsn_checker::Type(TypeKind::Fn(ft))) = &sym.ty else {
                continue;
            };

            let mut params = ft.params.clone();
            if params.first().and_then(|p| p.name.as_deref()) == Some("this") {
                params.remove(0);
            }

            let params_str = params
                .iter()
                .map(|p| format!("{}: {}", p.name.as_deref().unwrap_or("arg"), p.ty))
                .collect::<Vec<_>>()
                .join(", ");

            records.push(MemberRecord {
                name: method_name.clone(),
                type_str: ft.return_type.to_string(),
                params_str,
                is_static: false,
                is_optional: false,
                kind: MemberKind::Method,
                is_arrow: ft.is_arrow,
                line: sym.line.saturating_sub(1),
                col: sym.col,
                init_value: String::new(),
                ty: tsn_checker::Type(TypeKind::Fn(FunctionType {
                    params,
                    return_type: ft.return_type.clone(),
                    is_arrow: ft.is_arrow,
                    type_params: ft.type_params.clone(),
                })),
                members: Vec::new(),
            });
        }
    }
}

fn build_extension_accessor_members(
    bind: &tsn_checker::BindResult,
    scope: &tsn_checker::Scope,
    out: &mut HashMap<String, Vec<MemberRecord>>,
) {
    for (type_name, getters) in &bind.extension_getters {
        let records = out.entry(type_name.clone()).or_default();
        for (getter_name, mangled) in getters {
            let Some(sid) = scope.resolve(mangled, &bind.scopes) else {
                continue;
            };
            let sym = bind.arena.get(sid);
            let Some(tsn_checker::Type(TypeKind::Fn(ft))) = &sym.ty else {
                continue;
            };

            records.push(MemberRecord {
                name: getter_name.clone(),
                type_str: ft.return_type.to_string(),
                params_str: String::new(),
                is_static: false,
                is_optional: false,
                kind: MemberKind::Getter,
                is_arrow: false,
                line: sym.line.saturating_sub(1),
                col: sym.col,
                init_value: String::new(),
                ty: ft.return_type.as_ref().clone(),
                members: Vec::new(),
            });
        }
    }

    for (type_name, setters) in &bind.extension_setters {
        let records = out.entry(type_name.clone()).or_default();
        for (setter_name, mangled) in setters {
            let Some(sid) = scope.resolve(mangled, &bind.scopes) else {
                continue;
            };
            let sym = bind.arena.get(sid);
            let Some(tsn_checker::Type(TypeKind::Fn(ft))) = &sym.ty else {
                continue;
            };

            let params = ft
                .params
                .iter()
                .skip_while(|p| p.name.as_deref() == Some("this"))
                .map(|p| format!("{}: {}", p.name.as_deref().unwrap_or("arg"), p.ty))
                .collect::<Vec<_>>()
                .join(", ");

            records.push(MemberRecord {
                name: setter_name.clone(),
                type_str: ft.return_type.to_string(),
                params_str: params,
                is_static: false,
                is_optional: false,
                kind: MemberKind::Setter,
                is_arrow: false,
                line: sym.line.saturating_sub(1),
                col: sym.col,
                init_value: String::new(),
                ty: ft.return_type.as_ref().clone(),
                members: Vec::new(),
            });
        }
    }
}
