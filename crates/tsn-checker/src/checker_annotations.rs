use crate::binder::infer_expr_type;
use crate::symbol::SymbolKind;
use crate::BindResult;
use tsn_core::ast::operators::BinaryOp;
use tsn_core::ast::ImportSpecifier;
use tsn_core::ast::{Decl, Expr, Program, Stmt};
use tsn_core::TypeKind;
use tsn_core::{NumericKind, TypeAnnotations};

pub fn collect_type_annotations(program: &Program, bind: &BindResult) -> TypeAnnotations {
    let mut ann = TypeAnnotations::new();
    for stmt in &program.body {
        annotate_stmt(stmt, &mut ann, bind);
    }
    ann
}

fn annotate_stmt(stmt: &Stmt, ann: &mut TypeAnnotations, bind: &BindResult) {
    match stmt {
        Stmt::Expr { expression, .. } => annotate_expr(expression, ann, bind),
        Stmt::Return { argument, .. } => {
            if let Some(arg) = argument {
                annotate_expr(arg, ann, bind);
            }
        }
        Stmt::Block { stmts, .. } => {
            for s in stmts {
                annotate_stmt(s, ann, bind);
            }
        }
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            annotate_expr(test, ann, bind);
            annotate_stmt(consequent, ann, bind);
            if let Some(alt) = alternate {
                annotate_stmt(alt, ann, bind);
            }
        }
        Stmt::While { test, body, .. } => {
            annotate_expr(test, ann, bind);
            annotate_stmt(body, ann, bind);
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            if let Some(init_box) = init {
                match init_box.as_ref() {
                    tsn_core::ast::ForInit::Expr(e) => annotate_expr(e, ann, bind),
                    tsn_core::ast::ForInit::Var { declarators, .. } => {
                        for d in declarators {
                            if let Some(init_expr) = &d.init {
                                annotate_expr(init_expr, ann, bind);
                            }
                        }
                    }
                }
            }
            if let Some(t) = test {
                annotate_expr(t, ann, bind);
            }
            if let Some(u) = update {
                annotate_expr(u, ann, bind);
            }
            annotate_stmt(body, ann, bind);
        }
        Stmt::Decl(decl) => annotate_decl(decl, ann, bind),
        _ => {}
    }
}

fn annotate_decl(decl: &Decl, ann: &mut TypeAnnotations, bind: &BindResult) {
    match decl {
        Decl::Variable(v) => {
            for d in &v.declarators {
                if let Some(init) = &d.init {
                    annotate_expr(init, ann, bind);
                }
            }
        }
        Decl::Function(f) => {
            if !f.modifiers.is_declare {
                annotate_stmt(&f.body, ann, bind);
            }
        }
        Decl::Import(i) => {
            for spec in &i.specifiers {
                let name = match spec {
                    ImportSpecifier::Default { local, .. } => local,
                    ImportSpecifier::Named { local, .. } => local,
                    ImportSpecifier::Namespace { local, .. } => local,
                };
                let scope = bind.scopes.get(bind.global_scope);
                if let Some(id) = scope.resolve(name, &bind.scopes) {
                    let sym = bind.arena.get(id);
                    if matches!(
                        sym.kind,
                        SymbolKind::Interface
                            | SymbolKind::TypeAlias
                            | SymbolKind::Struct
                            | SymbolKind::Extension
                    ) {
                        ann.record_type_only(spec.range().start.offset);
                    }
                }
            }
        }
        _ => {}
    }
}

fn annotate_expr(expr: &Expr, ann: &mut TypeAnnotations, _bind: &BindResult) {
    match expr {
        Expr::Binary {
            op,
            left,
            right,
            range,
        } => {
            annotate_expr(left, ann, _bind);
            annotate_expr(right, ann, _bind);

            let is_arithmetic = matches!(
                op,
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div
            );
            if !is_arithmetic {
                return;
            }

            let l = infer_expr_type(left, None);
            let r = infer_expr_type(right, None);

            let kind = match (op, &l.0, &r.0) {
                (BinaryOp::Div, TypeKind::Int, TypeKind::Int) => Some(NumericKind::Float),
                (_, TypeKind::Int, TypeKind::Int) => Some(NumericKind::Int),
                (_, TypeKind::Float, _) | (_, _, TypeKind::Float) => Some(NumericKind::Float),
                _ => None,
            };
            if let Some(k) = kind {
                ann.record_numeric(range.start.offset, k);
            }
        }
        Expr::Paren { expression, .. } => annotate_expr(expression, ann, _bind),
        Expr::Unary { operand, .. } => annotate_expr(operand, ann, _bind),
        Expr::Logical { left, right, .. } => {
            annotate_expr(left, ann, _bind);
            annotate_expr(right, ann, _bind);
        }
        Expr::Assign { value, target, .. } => {
            annotate_expr(target, ann, _bind);
            annotate_expr(value, ann, _bind);
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            annotate_expr(test, ann, _bind);
            annotate_expr(consequent, ann, _bind);
            annotate_expr(alternate, ann, _bind);
        }
        Expr::Call { callee, args, .. } => {
            annotate_expr(callee, ann, _bind);
            for arg in args {
                let e = match arg {
                    tsn_core::ast::Arg::Positional(e) => e,
                    tsn_core::ast::Arg::Spread(e) => e,
                    tsn_core::ast::Arg::Named { value, .. } => value,
                };
                annotate_expr(e, ann, _bind);
            }
        }
        Expr::Member { object, .. } => annotate_expr(object, ann, _bind),
        Expr::As { expression, .. } | Expr::Satisfies { expression, .. } => {
            annotate_expr(expression, ann, _bind)
        }
        Expr::Array { elements, .. } => {
            for el in elements {
                if let tsn_core::ast::ArrayEl::Expr(e) = el {
                    annotate_expr(e, ann, _bind);
                }
            }
        }
        _ => {}
    }
}
