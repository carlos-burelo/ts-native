use crate::binder::BindResult;
use crate::checker::Checker;
use crate::types::Type;
use tsn_core::ast::pattern::MatchPattern;
use tsn_core::ast::{Expr, MatchCase};
use tsn_core::source::SourceRange;
use tsn_core::{well_known, Diagnostic, TypeKind};

impl Checker {
    pub(super) fn check_match_exhaustiveness(
        &mut self,
        subject_ty: &Type,
        cases: &[MatchCase],
        range: &SourceRange,
        bind: &BindResult,
    ) {
        let has_catch_all = cases
            .iter()
            .any(|c| matches!(c.pattern, MatchPattern::Wildcard) && c.guard.is_none());
        if has_catch_all {
            return;
        }

        if let TypeKind::Union(members) = &subject_ty.0 {
            let uncovered: Vec<String> = members
                .iter()
                .filter(|m| {
                    !cases
                        .iter()
                        .any(|c| c.guard.is_none() && pattern_covers_type(&c.pattern, m))
                })
                .map(|m| m.to_string())
                .collect();
            if !uncovered.is_empty() {
                self.diagnostics.push(Diagnostic::warning(
                    format!(
                        "non-exhaustive match: missing cases for {}",
                        uncovered.join(", ")
                    ),
                    range.clone(),
                ));
            }
            return;
        }

        let TypeKind::Named(type_name, _) = &subject_ty.0 else {
            return;
        };

        if let Some(variants) = bind.sum_type_variants.get(type_name.as_str()) {
            let uncovered: Vec<String> = variants
                .iter()
                .filter(|vname| {
                    !cases.iter().any(|c| {
                        if c.guard.is_some() {
                            return false;
                        }
                        match &c.pattern {
                            MatchPattern::Wildcard => true,
                            MatchPattern::Identifier(name) => name == *vname,
                            MatchPattern::Record { fields, .. } => {
                                fields.first().is_some_and(|(key, sub)| {
                                    key == "__variant__"
                                        && matches!(sub, Some(MatchPattern::Identifier(n)) if n == *vname)
                                })
                            }
                            _ => false,
                        }
                    })
                })
                .map(|v| v.to_string())
                .collect();
            if !uncovered.is_empty() {
                self.diagnostics.push(Diagnostic::warning(
                    format!(
                        "non-exhaustive match: missing cases for {}",
                        uncovered.join(", ")
                    ),
                    range.clone(),
                ));
            }
            return;
        }

        if let Some(variants) = bind.enum_members.get(type_name.as_str()) {
            let uncovered: Vec<String> = variants
                .iter()
                .filter(|v| {
                    !cases.iter().any(|c| {
                        if c.guard.is_some() {
                            return false;
                        }
                        match &c.pattern {
                            MatchPattern::Wildcard => true,
                            MatchPattern::Identifier(name) => name == &v.name,
                            MatchPattern::Literal(Expr::Member {
                                object,
                                property,
                                computed: false,
                                ..
                            }) => {
                                matches!(object.as_ref(), Expr::Identifier { name, .. } if name == type_name)
                                    && matches!(property.as_ref(), Expr::Identifier { name, .. } if name == &v.name)
                            }
                            _ => false,
                        }
                    })
                })
                .map(|v| format!("{}.{}", type_name, v.name))
                .collect();
            if !uncovered.is_empty() {
                self.diagnostics.push(Diagnostic::warning(
                    format!(
                        "non-exhaustive match: missing cases for {}",
                        uncovered.join(", ")
                    ),
                    range.clone(),
                ));
            }
        }
    }
}

fn pattern_covers_type(pattern: &MatchPattern, ty: &Type) -> bool {
    match pattern {
        MatchPattern::Wildcard => true,
        MatchPattern::Identifier(name) => match (&ty.0, name.as_str()) {
            (TypeKind::Int, well_known::INT) => true,
            (TypeKind::Float, well_known::FLOAT) => true,
            (TypeKind::Str, well_known::STR) => true,
            (TypeKind::Bool, well_known::BOOL) => true,
            (TypeKind::Null, well_known::NULL) => true,
            (TypeKind::Void, well_known::VOID) => true,
            (TypeKind::LiteralInt(_), well_known::INT) => true,
            (TypeKind::LiteralFloat(_), well_known::FLOAT) => true,
            (TypeKind::LiteralStr(_), well_known::STR) => true,
            (TypeKind::LiteralBool(_), well_known::BOOL) => true,
            (TypeKind::Named(n, _), label) => n == label,
            _ => false,
        },
        MatchPattern::Literal(Expr::NullLiteral { .. }) => matches!(ty.0, TypeKind::Null),
        MatchPattern::Literal(Expr::BoolLiteral { value, .. }) => {
            matches!(&ty.0, TypeKind::LiteralBool(b) if b == value)
                || matches!(ty.0, TypeKind::Bool)
        }
        MatchPattern::Type { type_name, .. } => {
            matches!(&ty.0, TypeKind::Named(n, _) if n == type_name)
        }
        _ => false,
    }
}
