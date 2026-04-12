mod async_members;
mod collectors;
mod infer_impl;
mod member_binary;

use crate::binder::BindResult;
use crate::checker::Checker;
use crate::types::Type;
use tsn_core::ast::Expr;

pub(crate) use self::collectors::{collect_checked_return_types, collect_yield_types};
use self::member_binary::is_atomic_expr;

impl Checker {
    pub(crate) fn infer_type(&mut self, expr: &Expr, bind: &BindResult) -> Type {
        let expr_id = expr as *const Expr as usize;
        let key = (expr_id, self.current_scope, self.infer_env_rev);
        if let Some(cached) = self.infer_cache.get(&key) {
            return cached.clone();
        }

        if is_atomic_expr(expr) {
            let start = expr.range().start.offset;
            if let Some(info) = self.expr_types.get(&start) {
                return info.ty.clone();
            }
        }

        if let Expr::NonNull { expression, .. } = expr {
            return self.infer_type(expression, bind).non_nullified();
        }

        let ty = self.infer_type_impl(expr, bind);
        let is_opt_call = matches!(
            expr,
            Expr::Call {
                callee,
                optional: false,
                ..
            } if matches!(callee.as_ref(), Expr::Member { optional: true, .. })
        );

        let resolved = match expr {
            Expr::Member { optional: true, .. } => Type::make_nullable(ty),
            Expr::Call { optional: true, .. } => Type::make_nullable(ty),
            _ if is_opt_call => Type::make_nullable(ty),
            _ => ty,
        };
        self.infer_cache.insert(key, resolved.clone());
        resolved
    }
}
