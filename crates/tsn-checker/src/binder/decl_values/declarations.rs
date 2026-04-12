use tsn_core::ast::{EnumDecl, Expr, FunctionDecl, Pattern, TypeAliasDecl, VarKind, VariableDecl};

use super::super::type_inference::{infer_expr_type, widen_literal};
use super::super::type_resolution::resolve_type_node;
use crate::binder::{ClassMemberInfo, ClassMemberKind};
use crate::scope::ScopeKind;
use crate::symbol::{Symbol, SymbolKind};
use crate::types::{FunctionType, Type};

impl super::super::Binder {
    pub(crate) fn bind_variable(&mut self, v: &VariableDecl) {
        let sym_kind = match v.kind {
            VarKind::Const => SymbolKind::Const,
            VarKind::Let => SymbolKind::Let,
        };
        for d in &v.declarators {
            let line = d.range.start.line;
            let has_explicit_ann = d.type_ann.is_some()
                || matches!(
                    &d.id,
                    Pattern::Identifier {
                        type_ann: Some(_),
                        ..
                    }
                );
            let ty = d
                .type_ann
                .as_ref()
                .or_else(|| match &d.id {
                    Pattern::Identifier { type_ann, .. } => type_ann.as_ref(),
                    _ => None,
                })
                .map(|ann| resolve_type_node(ann, Some(self)))
                .or_else(|| {
                    d.init
                        .as_ref()
                        .map(|e| infer_expr_type(e, Some(self)))
                        .filter(|t| !t.is_dynamic())
                        // For `let` (mutable) bindings without an explicit annotation,
                        // widen literal types: `let x = "foo"` → type `str`, not `"foo"`.
                        .map(|t| {
                            if sym_kind == SymbolKind::Let && !has_explicit_ann {
                                widen_literal(t)
                            } else {
                                t
                            }
                        })
                });

            self.bind_pattern(&d.id, sym_kind, line, v.doc.clone(), ty);

            if let Pattern::Identifier { name, .. } = &d.id {
                if let Some(Expr::Object { properties, .. }) = &d.init {
                    let fields = self.collect_object_members(properties);
                    if !fields.is_empty() {
                        self.object_members.insert(name.clone(), fields);
                    }
                }
            }

            if let Some(init_expr) = &d.init {
                self.bind_expr(init_expr);
            }
        }
    }

    pub(crate) fn bind_function(&mut self, f: &FunctionDecl) {
        let line = f.range.start.line;
        let params: Vec<crate::types::FunctionParam> = f
            .params
            .iter()
            .map(|p| {
                let name = super::super::type_inference::pattern_to_string(&p.pattern);
                let mut ty = p
                    .type_ann
                    .as_ref()
                    .or_else(|| match &p.pattern {
                        Pattern::Identifier { type_ann, .. } => type_ann.as_ref(),
                        _ => None,
                    })
                    .map(|ann| resolve_type_node(ann, Some(self)))
                    .or_else(|| {
                        p.default
                            .as_ref()
                            .map(|e| infer_expr_type(e, Some(self)))
                            .filter(|t| !t.is_dynamic())
                    })
                    .unwrap_or(Type::Dynamic);

                if p.is_rest {
                    if !matches!(ty.0, tsn_core::TypeKind::Array(_)) {
                        ty = Type::array(ty);
                    }
                }

                crate::types::FunctionParam {
                    name: Some(name),
                    ty,
                    optional: p.is_optional,
                    is_rest: p.is_rest,
                }
            })
            .collect();

        let ret = if f.modifiers.is_generator {
            Type::Dynamic
        } else {
            f.return_type
                .as_ref()
                .map(|ann| resolve_type_node(ann, Some(self)))
                .unwrap_or(Type::Void)
        };
        let fn_type = Type::fn_(FunctionType {
            params: params.clone(),
            return_type: Box::new(ret),
            is_arrow: false,
            type_params: f.type_params.iter().map(|t| t.name.clone()).collect(),
        });

        let mut sym = Symbol::new(SymbolKind::Function, f.id.clone(), line).with_type(fn_type);
        sym.col = f.range.start.column;
        sym.offset = f.range.start.offset;
        sym.has_explicit_type = f.return_type.is_some();
        sym.is_async = f.modifiers.is_async;
        sym.is_generator = f.modifiers.is_generator;
        sym.doc = f.doc.clone();
        sym.type_params = f.type_params.iter().map(|t| t.name.clone()).collect();
        sym.type_param_constraints = f
            .type_params
            .iter()
            .map(|t| {
                t.constraint
                    .as_ref()
                    .map(|c| resolve_type_node(c, Some(self)))
            })
            .collect();

        let sym_id = self.define(f.id.clone(), sym);

        let child = self.scopes.child(ScopeKind::Function, self.current);
        let saved = self.current;
        self.current = child;

        for p in f.params.iter() {
            let mut ty = p
                .type_ann
                .as_ref()
                .or_else(|| match &p.pattern {
                    Pattern::Identifier { type_ann, .. } => type_ann.as_ref(),
                    _ => None,
                })
                .map(|ann| resolve_type_node(ann, Some(self)))
                .or_else(|| {
                    p.default
                        .as_ref()
                        .map(|e| infer_expr_type(e, Some(self)))
                        .filter(|t| !t.is_dynamic())
                })
                .unwrap_or(Type::Dynamic);

            if p.is_rest {
                if !matches!(ty.0, tsn_core::TypeKind::Array(_)) {
                    ty = Type::array(ty);
                }
            }

            self.bind_pattern(
                &p.pattern,
                SymbolKind::Parameter,
                line,
                f.doc.clone(),
                Some(ty),
            );
        }

        if !f.modifiers.is_declare {
            self.bind_stmt(&f.body);
        }
        self.current = saved;

        let _ = sym_id;
    }

    pub(crate) fn bind_type_alias(&mut self, t: &TypeAliasDecl) {
        let has_type_params = !t.type_params.is_empty();
        // Generic aliases can't be eagerly resolved (type params unbound); store body for lazy use.
        let ty = if has_type_params {
            crate::types::Type::Dynamic
        } else {
            resolve_type_node(&t.alias, Some(self))
        };
        let mut sym =
            Symbol::new(SymbolKind::TypeAlias, t.id.clone(), t.range.start.line).with_type(ty);
        sym.doc = t.doc.clone();
        sym.type_params = t.type_params.iter().map(|tp| tp.name.clone()).collect();
        if has_type_params {
            sym.alias_node = Some(Box::new(t.alias.clone()));
        }
        self.define(t.id.clone(), sym);
    }

    pub(crate) fn bind_enum(&mut self, e: &EnumDecl) {
        let mut sym = Symbol::new(SymbolKind::Enum, e.id.clone(), e.range.start.line).with_type(
            Type::named_with_origin(e.id.clone(), Some(self.source_file.clone())),
        );
        sym.doc = e.doc.clone();
        self.define(e.id.clone(), sym);

        if !e.members.is_empty() {
            let variants: Vec<ClassMemberInfo> = e
                .members
                .iter()
                .map(|v| ClassMemberInfo {
                    name: v.id.clone(),
                    kind: ClassMemberKind::Property,
                    is_async: false,
                    is_static: true,
                    is_optional: false,
                    line: v.range.start.line.saturating_sub(1),
                    col: v.range.start.column,
                    ty: Type::named(e.id.clone()),
                    members: Vec::new(),
                    visibility: None,
                    is_abstract: false,
                    is_readonly: false,
                    is_override: false,
                })
                .collect();
            self.enum_members.insert(e.id.clone(), variants);
        }
    }
}
