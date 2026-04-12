use crate::checker::ExprInfo;
use crate::types::Type;
use rustc_hash::FxHashMap;
use tsn_core::ast::{Expr, Stmt};

/// Walk a block statement and collect the inferred types of all `return` expressions,
/// using the already-checked `expr_types` map. Does NOT descend into nested function bodies.
pub(crate) fn collect_checked_return_types(
    stmt: &Stmt,
    expr_types: &FxHashMap<u32, ExprInfo>,
) -> Vec<Type> {
    let mut out = Vec::new();
    collect_returns(stmt, expr_types, &mut out);
    out
}

fn collect_returns(stmt: &Stmt, expr_types: &FxHashMap<u32, ExprInfo>, out: &mut Vec<Type>) {
    match stmt {
        Stmt::Block { stmts, .. } => {
            for s in stmts {
                collect_returns(s, expr_types, out);
            }
        }
        Stmt::Return {
            argument: Some(e), ..
        } => {
            let offset = e.range().start.offset;
            if let Some(info) = expr_types.get(&offset) {
                if !info.ty.is_dynamic() {
                    out.push(info.ty.clone());
                }
            }
        }
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            collect_returns(consequent, expr_types, out);
            if let Some(alt) = alternate {
                collect_returns(alt, expr_types, out);
            }
        }
        Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
            collect_returns(body, expr_types, out);
        }
        Stmt::For { body, .. } | Stmt::ForIn { body, .. } | Stmt::ForOf { body, .. } => {
            collect_returns(body, expr_types, out);
        }
        Stmt::Try {
            block,
            catch,
            finally,
            ..
        } => {
            collect_returns(block, expr_types, out);
            if let Some(c) = catch {
                collect_returns(c.body.as_ref(), expr_types, out);
            }
            if let Some(f) = finally {
                collect_returns(f, expr_types, out);
            }
        }
        Stmt::Labeled { body, .. } => collect_returns(body, expr_types, out),
        Stmt::Switch { cases, .. } => {
            for case in cases {
                for s in &case.body {
                    collect_returns(s, expr_types, out);
                }
            }
        }
        // Nested Decl::Function / Expr::Arrow have their own return context — skip
        _ => {}
    }
}

/// Walk a block statement and collect the inferred types of all `yield` expressions,
/// using the already-checked `expr_types` map. Does NOT descend into nested function bodies.
pub(crate) fn collect_yield_types(stmt: &Stmt, expr_types: &FxHashMap<u32, ExprInfo>) -> Vec<Type> {
    let mut out = Vec::new();
    collect_yields(stmt, expr_types, &mut out);
    out
}

fn collect_yields(stmt: &Stmt, expr_types: &FxHashMap<u32, ExprInfo>, out: &mut Vec<Type>) {
    match stmt {
        Stmt::Block { stmts, .. } => {
            for s in stmts {
                collect_yields(s, expr_types, out);
            }
        }
        Stmt::Expr { expression, .. } => {
            collect_yields_expr(expression, expr_types, out);
        }
        Stmt::Return {
            argument: Some(e), ..
        } => {
            collect_yields_expr(e, expr_types, out);
        }
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            collect_yields(consequent, expr_types, out);
            if let Some(alt) = alternate {
                collect_yields(alt, expr_types, out);
            }
        }
        Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
            collect_yields(body, expr_types, out);
        }
        Stmt::For { body, .. } | Stmt::ForIn { body, .. } | Stmt::ForOf { body, .. } => {
            collect_yields(body, expr_types, out);
        }
        Stmt::Try {
            block,
            catch,
            finally,
            ..
        } => {
            collect_yields(block, expr_types, out);
            if let Some(c) = catch {
                collect_yields(c.body.as_ref(), expr_types, out);
            }
            if let Some(f) = finally {
                collect_yields(f, expr_types, out);
            }
        }
        Stmt::Labeled { body, .. } => collect_yields(body, expr_types, out),
        Stmt::Switch { cases, .. } => {
            for case in cases {
                for s in &case.body {
                    collect_yields(s, expr_types, out);
                }
            }
        }
        _ => {}
    }
}

fn collect_yields_expr(expr: &Expr, expr_types: &FxHashMap<u32, ExprInfo>, out: &mut Vec<Type>) {
    if let Expr::Yield {
        argument: Some(_),
        range,
        ..
    } = expr
    {
        let offset = range.start.offset;
        if let Some(info) = expr_types.get(&offset) {
            if !info.ty.is_dynamic() {
                out.push(info.ty.clone());
            }
        }
    }
}
