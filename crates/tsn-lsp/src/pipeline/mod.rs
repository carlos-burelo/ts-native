mod extensions;
mod format;
mod params;
mod symbols;

use std::collections::HashMap;

use crate::constants::{SEVERITY_ERROR, SEVERITY_HINT, SEVERITY_WARNING};
use crate::document::{
    uri_to_path, DocumentAnalysis, LspDiag, RelatedLocation, SymbolRecord, TokenRecord,
};
use tsn_checker::types::FunctionType;
use tsn_checker::{module_resolver, SymbolKind};
use tsn_core::ast::{Decl, Stmt};
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
            severity: SEVERITY_ERROR,
            related: Vec::new(),
        });
    }

    let result = tsn_checker::Checker::check(&program);

    for d in &result.diagnostics {
        let severity = match d.kind {
            DiagnosticKind::Error => SEVERITY_ERROR,
            DiagnosticKind::Warning => SEVERITY_WARNING,
            DiagnosticKind::Hint => SEVERITY_HINT,
        };
        // Propagate related locations from checker metadata if present.
        // Format: metadata["related_line"] = "42", metadata["related_col"] = "5",
        //         metadata["related_msg"] = "declared here"
        let related = build_related_locations(d, &uri);
        diagnostics.push(LspDiag {
            message: d.message.clone(),
            line: d.range.start.line.saturating_sub(1),
            col: d.range.start.column,
            end_line: d.range.end.line.saturating_sub(1),
            end_col: d.range.end.column,
            severity,
            related,
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
                    .map(|ms| symbols::map_enum_members(ms, &tokens))
                    .or_else(|| {
                        sym.origin_module.as_ref().and_then(|origin| {
                            module_resolver::resolve_module_bind(origin).and_then(|rb| {
                                rb.enum_members
                                    .get(sym.original_name.as_ref().unwrap_or(&sym.name))
                                    .map(|ms| symbols::map_enum_members(ms, &tokens))
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
                end_line: if sym.full_range.end.line > 0 {
                    sym.full_range.end.line.saturating_sub(1)
                } else {
                    sym.line.saturating_sub(1)
                },
                end_col: if sym.full_range.end.line > 0 {
                    sym.full_range.end.column
                } else {
                    sym.col + sym.name.len() as u32
                },
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
    let extension_members = extensions::build_extension_members(&result.bind);

    let param_scopes = params::collect_param_scopes(&tokens);
    let (type_param_map, type_param_names) = params::collect_type_params(&tokens);
    for sym in &mut all_symbols {
        if sym.line != crate::constants::STDLIB_LINE_MARKER {
            if let Some(tps) = type_param_map.get(&sym.name) {
                sym.type_params = tps.clone();
            }
        }
    }

    let import_paths = collect_import_paths(&program.body);

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
        import_paths,
    }
}

fn build_related_locations(d: &tsn_core::Diagnostic, current_uri: &str) -> Vec<RelatedLocation> {
    // Checker can embed related location in metadata with keys:
    // "related_line", "related_col", "related_msg"
    let line_str = d.metadata.get("related_line");
    let col_str = d.metadata.get("related_col");
    let msg = d.metadata.get("related_msg");
    match (line_str, col_str, msg) {
        (Some(ln), Some(col), Some(msg)) => {
            let line = ln.parse::<u32>().ok().map(|l| l.saturating_sub(1));
            let col = col.parse::<u32>().ok();
            match (line, col) {
                (Some(line), Some(col)) => vec![RelatedLocation {
                    message: msg.clone(),
                    uri: current_uri.to_owned(),
                    line,
                    col,
                }],
                _ => Vec::new(),
            }
        }
        _ => Vec::new(),
    }
}

fn collect_import_paths(stmts: &[Stmt]) -> Vec<String> {
    let mut paths = Vec::new();
    for stmt in stmts {
        if let Stmt::Decl(decl) = stmt {
            if let Decl::Import(i) = decl.as_ref() {
                paths.push(i.source.clone());
            }
        }
    }
    paths
}
