use crate::binder::{pattern_lead_name, BindResult};
use crate::checker_call_types::infer_call_type;
use crate::symbol::SymbolId;
use crate::symbol::SymbolKind;
use crate::types::FunctionType;
use crate::types::Type;
use std::collections::HashMap;
use tsn_core::ast::{Decl, ExportDecl, ExportDefaultDecl, Stmt};
use tsn_core::TypeKind;

pub fn enrich_call_returns(bind: &mut BindResult, program: &tsn_core::ast::Program) {
    let EnrichIndexes {
        fn_map,
        mut sym_map,
        fn_type_params,
        decl_ids,
    } = build_enrich_indexes(bind);
    let class_methods = bind.class_methods.clone();
    enrich_stmts(
        &fn_map,
        &fn_type_params,
        &class_methods,
        &decl_ids,
        &mut sym_map,
        &program.body,
        bind,
    );
}

fn enrich_stmts(
    fn_map: &HashMap<String, Type>,
    fn_type_params: &HashMap<String, Vec<String>>,
    class_methods: &HashMap<String, HashMap<String, Type>>,
    decl_ids: &HashMap<(String, u32), SymbolId>,
    sym_map: &mut HashMap<String, Type>,
    stmts: &[Stmt],
    bind: &mut BindResult,
) {
    for stmt in stmts {
        enrich_stmt(
            fn_map,
            fn_type_params,
            class_methods,
            decl_ids,
            sym_map,
            stmt,
            bind,
        );
    }
}

fn enrich_stmt(
    fn_map: &HashMap<String, Type>,
    fn_type_params: &HashMap<String, Vec<String>>,
    class_methods: &HashMap<String, HashMap<String, Type>>,
    decl_ids: &HashMap<(String, u32), SymbolId>,
    sym_map: &mut HashMap<String, Type>,
    stmt: &Stmt,
    bind: &mut BindResult,
) {
    match stmt {
        Stmt::Decl(decl) => enrich_decl(
            fn_map,
            fn_type_params,
            class_methods,
            decl_ids,
            sym_map,
            decl,
            bind,
        ),
        Stmt::Block { stmts, .. } => enrich_stmts(
            fn_map,
            fn_type_params,
            class_methods,
            decl_ids,
            sym_map,
            stmts,
            bind,
        ),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            enrich_stmt(
                fn_map,
                fn_type_params,
                class_methods,
                decl_ids,
                sym_map,
                consequent,
                bind,
            );
            if let Some(alt) = alternate {
                enrich_stmt(
                    fn_map,
                    fn_type_params,
                    class_methods,
                    decl_ids,
                    sym_map,
                    alt,
                    bind,
                );
            }
        }
        Stmt::While { body, .. }
        | Stmt::DoWhile { body, .. }
        | Stmt::For { body, .. }
        | Stmt::ForIn { body, .. }
        | Stmt::ForOf { body, .. }
        | Stmt::Labeled { body, .. } => {
            enrich_stmt(
                fn_map,
                fn_type_params,
                class_methods,
                decl_ids,
                sym_map,
                body,
                bind,
            );
        }
        Stmt::Switch { cases, .. } => {
            for case in cases {
                enrich_stmts(
                    fn_map,
                    fn_type_params,
                    class_methods,
                    decl_ids,
                    sym_map,
                    &case.body,
                    bind,
                );
            }
        }
        Stmt::Try {
            block,
            catch,
            finally,
            ..
        } => {
            enrich_stmt(
                fn_map,
                fn_type_params,
                class_methods,
                decl_ids,
                sym_map,
                block,
                bind,
            );
            if let Some(c) = catch {
                enrich_stmt(
                    fn_map,
                    fn_type_params,
                    class_methods,
                    decl_ids,
                    sym_map,
                    &c.body,
                    bind,
                );
            }
            if let Some(f) = finally {
                enrich_stmt(
                    fn_map,
                    fn_type_params,
                    class_methods,
                    decl_ids,
                    sym_map,
                    f,
                    bind,
                );
            }
        }
        _ => {}
    }
}

fn enrich_decl(
    fn_map: &HashMap<String, Type>,
    fn_type_params: &HashMap<String, Vec<String>>,
    class_methods: &HashMap<String, HashMap<String, Type>>,
    decl_ids: &HashMap<(String, u32), SymbolId>,
    sym_map: &mut HashMap<String, Type>,
    decl: &Decl,
    bind: &mut BindResult,
) {
    match decl {
        Decl::Variable(v) => {
            for d in &v.declarators {
                let name = pattern_lead_name(&d.id);
                let decl_line = d.range.start.line;

                let sym_id = if let Some(id) = decl_ids.get(&(name.to_string(), decl_line)) {
                    *id
                } else {
                    let scope = bind.scopes.get(bind.global_scope);
                    match scope.lookup(name) {
                        Some(id) => id,
                        None => continue,
                    }
                };

                if bind.arena.get(sym_id).ty.is_some() {
                    if let Some(existing) = &bind.arena.get(sym_id).ty {
                        sym_map
                            .entry(name.to_string())
                            .or_insert_with(|| existing.clone());
                    }
                    continue;
                }
                let init = match &d.init {
                    Some(e) => e,
                    None => continue,
                };
                let ty = infer_call_type(
                    fn_map,
                    fn_type_params,
                    class_methods,
                    sym_map,
                    init,
                    Some(bind),
                );
                if let Some(t) = ty {
                    bind.arena.get_mut(sym_id).ty = Some(t.clone());
                    sym_map.insert(name.to_string(), t);
                }
            }
        }
        Decl::Function(f) => {
            if let Stmt::Block { stmts, .. } = &f.body {
                enrich_stmts(
                    fn_map,
                    fn_type_params,
                    class_methods,
                    decl_ids,
                    sym_map,
                    stmts,
                    bind,
                );
            }
        }
        Decl::Export(e) => match e {
            ExportDecl::Decl { declaration, .. } => enrich_decl(
                fn_map,
                fn_type_params,
                class_methods,
                decl_ids,
                sym_map,
                declaration,
                bind,
            ),
            ExportDecl::Default { declaration, .. } => match declaration.as_ref() {
                ExportDefaultDecl::Function(_) => {}
                _ => {}
            },
            _ => {}
        },
        _ => {}
    }
}

struct EnrichIndexes {
    fn_map: HashMap<String, Type>,
    sym_map: HashMap<String, Type>,
    fn_type_params: HashMap<String, Vec<String>>,
    decl_ids: HashMap<(String, u32), SymbolId>,
}

fn build_enrich_indexes(bind: &BindResult) -> EnrichIndexes {
    let symbols = bind.arena.all();
    let mut fn_map = HashMap::with_capacity(symbols.len());
    let mut sym_map = HashMap::with_capacity(symbols.len());
    let mut fn_type_params = HashMap::new();
    let mut decl_ids = HashMap::new();

    for (id, sym) in symbols.iter().enumerate() {
        decl_ids.insert((sym.name.clone(), sym.line), id);

        if let Some(ty) = &sym.ty {
            sym_map.insert(sym.name.clone(), ty.clone());

            if sym.kind == SymbolKind::Function {
                if let Type(TypeKind::Fn(FunctionType { return_type, .. })) = ty {
                    let raw = return_type.as_ref().clone();
                    if !raw.is_dynamic() {
                        let call_ret = if sym.is_async
                            && !matches!(&raw.0, TypeKind::Generic(name, _, _) if name == tsn_core::well_known::FUTURE)
                        {
                            Type::generic(tsn_core::well_known::FUTURE.to_owned(), vec![raw])
                        } else {
                            raw
                        };
                        fn_map.insert(sym.name.clone(), call_ret);
                    }
                }
            }
        }

        if sym.kind == SymbolKind::Function && !sym.type_params.is_empty() {
            fn_type_params.insert(sym.name.clone(), sym.type_params.clone());
        }
    }

    EnrichIndexes {
        fn_map,
        sym_map,
        fn_type_params,
        decl_ids,
    }
}
