use crate::symbol::{Symbol, SymbolId};
use crate::types::Type;
use tsn_core::ast::pattern::MatchPattern;
use tsn_core::ast::{Arg, Decl, Expr, Param};

impl super::Binder {
    pub(super) fn bind_decl(&mut self, decl: &tsn_core::ast::Decl) {
        match decl {
            Decl::Variable(v) => self.bind_variable(v),
            Decl::Function(f) => self.bind_function(f),
            Decl::Class(c) => self.bind_class(c),
            Decl::Interface(i) => self.bind_interface(i),
            Decl::TypeAlias(t) => self.bind_type_alias(t),
            Decl::Enum(e) => self.bind_enum(e),
            Decl::Namespace(n) => self.bind_namespace(n),
            Decl::Struct(s) => self.bind_struct(s),
            Decl::Extension(e) => self.bind_extension(e),
            Decl::Import(i) => self.bind_import(i),
            Decl::Export(e) => self.bind_export(e),
            Decl::SumType(t) => self.bind_sum_type(t),
        }
    }

    pub(super) fn define(&mut self, name: String, sym: Symbol) -> SymbolId {
        let id = self.arena.push(sym);
        self.scopes.get_mut(self.current).define(name, id);
        id
    }

    pub(super) fn bind_expr(&mut self, expr: &tsn_core::ast::Expr) {
        match expr {
            Expr::Arrow {
                params,
                return_type,
                body,
                is_async: _,
                ..
            } => match body.as_ref() {
                tsn_core::ast::ArrowBody::Block(stmt) => {
                    self.bind_inline_function(params, return_type.as_ref(), stmt, expr.range());
                }
                tsn_core::ast::ArrowBody::Expr(e) => {
                    self.bind_inline_function_expr(params, e, expr.range());
                }
            },
            Expr::Function {
                params,
                return_type,
                body,
                is_async: _,
                ..
            } => {
                self.bind_inline_function(params, return_type.as_ref(), body, expr.range());
            }
            Expr::As { expression, .. } | Expr::Satisfies { expression, .. } => {
                self.bind_expr(expression)
            }
            Expr::Await { argument, .. } => self.bind_expr(argument),
            Expr::Yield { argument, .. } => {
                if let Some(arg) = argument {
                    self.bind_expr(arg);
                }
            }
            Expr::Unary { operand, .. } => self.bind_expr(operand),
            Expr::Binary { left, right, .. } | Expr::Logical { left, right, .. } => {
                self.bind_expr(left);
                self.bind_expr(right);
            }
            Expr::Assign { target, value, .. } => {
                self.bind_expr(target);
                self.bind_expr(value);
            }
            Expr::Call { callee, args, .. } => {
                self.bind_expr(callee);
                self.bind_args(args);
            }
            Expr::New { callee, args, .. } => {
                self.bind_expr(callee);
                self.bind_args(args);
            }
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => {
                self.bind_expr(test);
                self.bind_expr(consequent);
                self.bind_expr(alternate);
            }
            Expr::Member {
                object,
                property,
                computed,
                ..
            } => {
                self.bind_expr(object);
                if *computed {
                    self.bind_expr(property);
                }
            }
            Expr::Paren { expression, .. } => self.bind_expr(expression),
            Expr::NonNull { expression, .. } => self.bind_expr(expression),
            Expr::Array { elements, .. } => {
                for el in elements {
                    match el {
                        tsn_core::ast::ArrayEl::Expr(e) => self.bind_expr(e),
                        tsn_core::ast::ArrayEl::Spread(e) => self.bind_expr(e),
                        tsn_core::ast::ArrayEl::Hole => {}
                    }
                }
            }
            Expr::Object { properties, .. } => {
                for prop in properties {
                    match prop {
                        tsn_core::ast::ObjectProp::Property { value, .. } => self.bind_expr(value),
                        tsn_core::ast::ObjectProp::Method {
                            params,
                            return_type,
                            body,
                            range,
                            ..
                        } => {
                            self.bind_inline_function(params, return_type.as_ref(), body, range);
                        }
                        tsn_core::ast::ObjectProp::Getter { body, .. } => {
                            self.bind_stmt(body);
                        }
                        tsn_core::ast::ObjectProp::Setter { body, .. } => {
                            self.bind_stmt(body);
                        }
                        tsn_core::ast::ObjectProp::Spread { argument, .. } => {
                            self.bind_expr(argument)
                        }
                    }
                }
            }
            Expr::Template { parts, .. } => {
                for p in parts {
                    if let tsn_core::ast::TemplatePart::Interpolation(e) = p {
                        self.bind_expr(e);
                    }
                }
            }
            Expr::Sequence { expressions, .. } => {
                for e in expressions {
                    self.bind_expr(e);
                }
            }
            Expr::ClassExpr { declaration, .. } => {
                self.bind_class(declaration);
            }
            Expr::Match { subject, cases, .. } => {
                self.bind_expr(subject);
                for case in cases {
                    use crate::scope::ScopeKind;

                    let child = self.scopes.child(ScopeKind::Block, self.current);
                    let saved = self.current;
                    self.current = child;

                    bind_match_pattern_vars(self, &case.pattern);

                    if let Some(g) = &case.guard {
                        self.bind_expr(g);
                    }
                    match &case.body {
                        tsn_core::ast::MatchBody::Expr(e) => self.bind_expr(e),
                        tsn_core::ast::MatchBody::Block(stmt) => self.bind_stmt(stmt),
                    }
                    self.current = saved;
                }
            }
            Expr::Update { operand, .. } => self.bind_expr(operand),
            Expr::Spread { argument, .. } => self.bind_expr(argument),
            Expr::Pipeline { left, right, .. } => {
                self.bind_expr(left);
                self.bind_expr(right);
            }
            Expr::Range { start, end, .. } => {
                self.bind_expr(start);
                self.bind_expr(end);
            }
            Expr::TaggedTemplate { tag, template, .. } => {
                self.bind_expr(tag);
                self.bind_expr(template);
            }

            Expr::Identifier { .. }
            | Expr::IntLiteral { .. }
            | Expr::FloatLiteral { .. }
            | Expr::BigIntLiteral { .. }
            | Expr::DecimalLiteral { .. }
            | Expr::StrLiteral { .. }
            | Expr::CharLiteral { .. }
            | Expr::BoolLiteral { .. }
            | Expr::RegexLiteral { .. }
            | Expr::NullLiteral { .. }
            | Expr::Super { .. }
            | Expr::This { .. } => {}
        }
    }

    pub(super) fn bind_inline_function(
        &mut self,
        params: &[tsn_core::ast::Param],
        _return_type: Option<&tsn_core::ast::TypeNode>,
        body: &tsn_core::ast::Stmt,
        range: &tsn_core::SourceRange,
    ) {
        use crate::scope::ScopeKind;

        let line = range.start.line;

        let child = self.scopes.child(ScopeKind::Function, self.current);
        let saved = self.current;
        self.current = child;

        self.bind_function_params(params, line);

        self.bind_stmt(body);
        self.current = saved;
    }

    fn bind_inline_function_expr(
        &mut self,
        params: &[Param],
        body: &Expr,
        range: &tsn_core::SourceRange,
    ) {
        use crate::scope::ScopeKind;

        let child = self.scopes.child(ScopeKind::Function, self.current);
        let saved = self.current;
        self.current = child;
        self.bind_function_params(params, range.start.line);
        self.bind_expr(body);
        self.current = saved;
    }

    fn bind_function_params(&mut self, params: &[Param], line: u32) {
        use super::type_inference::infer_expr_type;
        use super::type_resolution::resolve_type_node;
        use crate::symbol::SymbolKind;
        use crate::types::Type;

        for p in params {
            let ty = p
                .type_ann
                .as_ref()
                .or_else(|| match &p.pattern {
                    tsn_core::ast::Pattern::Identifier { type_ann, .. } => type_ann.as_ref(),
                    _ => None,
                })
                .map(|m| resolve_type_node(m, Some(self)))
                .or_else(|| {
                    p.default
                        .as_ref()
                        .map(|e| infer_expr_type(e, Some(self)))
                        .filter(|t| !t.is_dynamic())
                })
                .unwrap_or(Type::Dynamic);

            self.bind_pattern(&p.pattern, SymbolKind::Parameter, line, None, Some(ty));

            if let Some(default_value) = &p.default {
                self.bind_expr(default_value);
            }
        }
    }

    fn bind_args(&mut self, args: &[Arg]) {
        for arg in args {
            match arg {
                Arg::Positional(expr) | Arg::Spread(expr) => self.bind_expr(expr),
                Arg::Named { value, .. } => self.bind_expr(value),
            }
        }
    }
}

fn bind_match_pattern_vars(b: &mut super::Binder, pattern: &MatchPattern) {
    use crate::symbol::SymbolKind;
    match pattern {
        MatchPattern::Wildcard | MatchPattern::Literal(_) | MatchPattern::Type { .. } => {}
        MatchPattern::Identifier(name) => {
            if name != "_" {
                let sym = Symbol::new(SymbolKind::Let, name.clone(), 0).with_type(Type::Dynamic);
                b.define(name.clone(), sym);
            }
        }
        MatchPattern::Record { fields, .. } => {
            let variant_name = fields.first().and_then(|(key, sub)| {
                if key == "__variant__" {
                    if let Some(MatchPattern::Identifier(n)) = sub {
                        return Some(n.clone());
                    }
                }
                None
            });

            if let Some(vname) = variant_name {
                let field_types: Vec<(String, Type)> = b
                    .sum_variant_fields
                    .get(&vname)
                    .cloned()
                    .unwrap_or_default();

                for (field_key, sub_pat) in fields.iter().skip(1) {
                    let binding_name = match sub_pat {
                        Some(MatchPattern::Identifier(n)) => n.clone(),
                        _ => field_key.clone(),
                    };
                    if binding_name == "_" {
                        continue;
                    }

                    let ty = field_types
                        .iter()
                        .find(|(fname, _)| fname == field_key)
                        .map(|(_, t)| t.clone())
                        .unwrap_or(Type::Dynamic);
                    let sym = Symbol::new(SymbolKind::Let, binding_name.clone(), 0).with_type(ty);
                    b.define(binding_name, sym);

                    if let Some(sub) = sub_pat {
                        if !matches!(sub, MatchPattern::Identifier(_)) {
                            bind_match_pattern_vars(b, sub);
                        }
                    }
                }
            } else {
                for (field_name, sub_pat) in fields {
                    let binding_name = match sub_pat {
                        Some(MatchPattern::Identifier(n)) => n.clone(),
                        _ => field_name.clone(),
                    };
                    if binding_name != "_" {
                        let sym = Symbol::new(SymbolKind::Let, binding_name.clone(), 0)
                            .with_type(Type::Dynamic);
                        b.define(binding_name, sym);
                    }
                    if let Some(sub) = sub_pat {
                        if !matches!(sub, MatchPattern::Identifier(_)) {
                            bind_match_pattern_vars(b, sub);
                        }
                    }
                }
            }
        }
        MatchPattern::Sequence(pats) => {
            for p in pats {
                bind_match_pattern_vars(b, p);
            }
        }
    }
}
