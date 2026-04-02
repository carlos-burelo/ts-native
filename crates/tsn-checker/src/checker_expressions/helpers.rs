use crate::checker::ExprInfo;
use crate::types::Type;
use crate::{checker::Checker, SymbolId};
use std::collections::HashMap;
use tsn_core::ast::operators::BinaryOp;
use tsn_core::TypeKind;

pub(super) fn base_type(ty: &Type) -> &Type {
    match &ty.0 {
        TypeKind::LiteralInt(_) => &Type::Int,
        TypeKind::LiteralFloat(_) => &Type::Float,
        TypeKind::LiteralStr(_) => &Type::Str,
        TypeKind::LiteralBool(_) => &Type::Bool,
        _ => ty,
    }
}

pub(super) fn op_str(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Pow => "**",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
        BinaryOp::UShr => ">>>",
        BinaryOp::Eq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Gt => ">",
        BinaryOp::LtEq => "<=",
        BinaryOp::GtEq => ">=",
        BinaryOp::In => "in",
        BinaryOp::Instanceof => "instanceof",
    }
}

impl Checker {
    pub(crate) fn is_subclass_or_same(
        &self,
        candidate: &str,
        target: &str,
        parents: &HashMap<String, String>,
    ) -> bool {
        let mut current = candidate;
        loop {
            if current == target {
                return true;
            }
            match parents.get(current) {
                Some(parent) => current = parent.as_str(),
                None => return false,
            }
        }
    }

    pub(crate) fn record_type(&mut self, offset: u32, ty: Type) {
        self.expr_types.insert(
            offset,
            ExprInfo {
                ty,
                symbol_id: None,
            },
        );
    }

    pub(crate) fn record_type_with_symbol(&mut self, offset: u32, ty: Type, symbol_id: SymbolId) {
        self.var_types.insert(symbol_id, ty.clone());
        self.mark_infer_env_dirty();
        self.expr_types.insert(
            offset,
            ExprInfo {
                ty,
                symbol_id: Some(symbol_id),
            },
        );
    }
}
