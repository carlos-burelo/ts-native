use tsn_checker::{types::FunctionType, ClassMemberKind, SymbolKind, Type};
use tsn_core::ast::{Decl, Expr, Program, Stmt};
use tsn_core::{well_known, TypeKind};

pub fn debug_types(program: &Program, range: Option<(u32, u32)>) {
    use super::super::{footer, header, BOLD, C_TYPES, DIM, R};

    // Only class/interface/enum/struct inherit annotations — built from AST
    // (extends/implements/variants/fields). Member types come from bind below.
    let mut ast_annotations: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    fn expr_name(e: &Expr) -> Option<&str> {
        if let Expr::Identifier { name, .. } = e {
            Some(name)
        } else {
            None
        }
    }

    fn collect_annotations(stmts: &[Stmt], map: &mut std::collections::HashMap<String, String>) {
        for stmt in stmts {
            let Stmt::Decl(decl) = stmt else { continue };
            match decl.as_ref() {
                Decl::Class(c) => {
                    let name = c.id.as_deref().unwrap_or("<anon>");
                    let mut parts: Vec<String> = Vec::new();
                    if let Some(super_expr) = &c.super_class {
                        if let Some(n) = expr_name(super_expr) {
                            parts.push(format!("extends {n}"));
                        }
                    }
                    if !c.implements.is_empty() {
                        let ifaces: Vec<&str> = c
                            .implements
                            .iter()
                            .filter_map(|t| {
                                if let TypeKind::Named(n, _origin)
                                | TypeKind::Generic(n, _, _origin) = &t.kind
                                {
                                    Some(n.as_str())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if !ifaces.is_empty() {
                            parts.push(format!("implements {}", ifaces.join(", ")));
                        }
                    }
                    map.insert(name.to_owned(), parts.join("  "));
                }
                Decl::Interface(i) => {
                    if !i.extends.is_empty() {
                        let bases: Vec<&str> = i
                            .extends
                            .iter()
                            .filter_map(|t| {
                                if let TypeKind::Named(n, _origin)
                                | TypeKind::Generic(n, _, _origin) = &t.kind
                                {
                                    Some(n.as_str())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if !bases.is_empty() {
                            map.insert(i.id.clone(), format!("extends {}", bases.join(", ")));
                        }
                    }
                }
                Decl::Enum(e) => {
                    let variants: Vec<&str> =
                        e.members.iter().map(|m| m.id.as_str()).take(6).collect();
                    let ellipsis = if e.members.len() > 6 { " | ..." } else { "" };
                    map.insert(
                        e.id.clone(),
                        format!("{{ {}{ellipsis} }}", variants.join(" | ")),
                    );
                }
                Decl::Namespace(_) => {} // filled from bind.namespace_members
                Decl::Struct(s) => {
                    map.insert(s.id.clone(), format!("({} fields)", s.fields.len()));
                }
                _ => {}
            }
        }
    }
    collect_annotations(&program.body, &mut ast_annotations);

    /* Removed STDLIB static annotations - stdlib is now dynamic
    for module in STDLIB.iter() {
        ...
    }
    */

    let result = tsn_checker::Checker::check(program);
    let bind = &result.bind;

    let in_range = |line: u32| match range {
        Some((lo, hi)) => line >= lo && line <= hi,
        None => true,
    };

    header(C_TYPES, "types", &program.filename);

    let all_symbols: Vec<_> = bind.global_symbols().collect();
    let symbols: Vec<_> = all_symbols.iter().filter(|s| in_range(s.line)).collect();
    for sym in &symbols {
        let ty_str = match &sym.ty {
            Some(ty) => format!("{ty}"),
            None => well_known::DYNAMIC.to_owned(),
        };

        let annotation = match sym.kind {
            SymbolKind::Function => {
                let display_type = if sym.is_async {
                    if let Some(Type(TypeKind::Fn(ft))) = &sym.ty {
                        let inner = ft.return_type.as_ref();
                        let already_future = matches!(inner.0,
                            TypeKind::Generic(ref n, _, _) if n == "Future");
                        if already_future {
                            ty_str.clone()
                        } else {
                            let wrapped = Type::generic("Future".to_owned(), vec![inner.clone()]);
                            format!(
                                "{}",
                                Type::fn_(FunctionType {
                                    params: ft.params.clone(),
                                    return_type: Box::new(wrapped),
                                    is_arrow: ft.is_arrow,
                                    type_params: ft.type_params.clone(),
                                })
                            )
                        }
                    } else {
                        ty_str.clone()
                    }
                } else {
                    ty_str.clone()
                };
                let gen = if sym.is_generator { " gen" } else { "" };
                format!("{display_type}{gen}")
            }

            SymbolKind::Class => {
                // type params suffix for return type: Dog<T>
                let return_ty = if sym.type_params.is_empty() {
                    sym.name.clone()
                } else {
                    format!("{}<{}>", sym.name, sym.type_params.join(", "))
                };
                // constructor signature from bind
                let ctor_sig = bind
                    .class_members
                    .get(&sym.name)
                    .and_then(|ms| {
                        ms.iter()
                            .find(|m| matches!(m.kind, ClassMemberKind::Constructor))
                    })
                    .map(|ctor| {
                        let ps = ctor.params_str();
                        if ps.is_empty() {
                            format!("new() => {return_ty}")
                        } else {
                            format!("new({ps}) => {return_ty}")
                        }
                    });
                let mut parts: Vec<String> = Vec::new();
                // type params display
                if !sym.type_params.is_empty() {
                    parts.push(format!("<{}>", sym.type_params.join(", ")));
                }
                // extends / implements from AST
                if let Some(inh) = ast_annotations.get(&sym.name) {
                    if !inh.is_empty() {
                        parts.push(inh.clone());
                    }
                }
                if let Some(sig) = ctor_sig {
                    parts.push(sig);
                }
                parts.join("  ")
            }

            SymbolKind::Interface => {
                let members = bind
                    .interface_members
                    .get(&sym.name)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);
                let mut parts: Vec<String> = Vec::new();
                if !sym.type_params.is_empty() {
                    parts.push(format!("<{}>", sym.type_params.join(", ")));
                }
                if let Some(ext) = ast_annotations.get(&sym.name) {
                    if !ext.is_empty() {
                        parts.push(ext.clone());
                    }
                }
                if !members.is_empty() {
                    let props: Vec<String> = members
                        .iter()
                        .take(5)
                        .map(|m| {
                            let opt = if m.is_optional { "?" } else { "" };
                            match m.kind {
                                ClassMemberKind::Method => {
                                    format!("{}{opt}(): {}", m.name, m.return_type_str())
                                }
                                _ => format!("{}{opt}: {}", m.name, m.ty),
                            }
                        })
                        .collect();
                    let ellipsis = if members.len() > 5 { ", ..." } else { "" };
                    parts.push(format!("{{ {}{ellipsis} }}", props.join(", ")));
                }
                parts.join("  ")
            }

            SymbolKind::Namespace => {
                let members = bind
                    .namespace_members
                    .get(&sym.name)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);
                if !members.is_empty() {
                    let names: Vec<&str> =
                        members.iter().map(|m| m.name.as_str()).take(6).collect();
                    let ellipsis = if members.len() > 6 { ", ..." } else { "" };
                    format!("{{ {}{ellipsis} }}", names.join(", "))
                } else {
                    ast_annotations.get(&sym.name).cloned().unwrap_or_default()
                }
            }

            SymbolKind::Enum | SymbolKind::Struct => {
                ast_annotations.get(&sym.name).cloned().unwrap_or_default()
            }

            SymbolKind::Extension => String::new(),
            _ => format!(": {ty_str}"),
        };

        let ann_display = if annotation.is_empty() {
            String::new()
        } else {
            format!("  {C_TYPES}{DIM}{annotation}{R}")
        };
        eprintln!(
            "  {C_TYPES}{}{R}  {BOLD}{:<24}{R}  {DIM}ln:{}{R}{}",
            sym.kind.label(),
            sym.name,
            sym.line,
            ann_display
        );
    }

    if !result.diagnostics.is_empty() {
        eprintln!();
        for diag in &result.diagnostics {
            eprintln!(
                "  {}[type error]{R} {}  (ln:{}:{})",
                "\x1b[91m", diag.message, diag.range.start.line, diag.range.start.column
            );
        }
    }

    let sym_footer = if range.is_some() && symbols.len() < all_symbols.len() {
        format!(
            "{} symbol(s) shown ({} total), {} diagnostic(s)",
            symbols.len(),
            all_symbols.len(),
            result.diagnostics.len()
        )
    } else {
        format!(
            "{} symbol(s), {} diagnostic(s)",
            symbols.len(),
            result.diagnostics.len()
        )
    };
    footer(C_TYPES, &sym_footer);
}
