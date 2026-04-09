use tsn_core::ast::{
    EnumDecl, Expr, ExtensionDecl, ExtensionMember, FunctionDecl, NamespaceDecl, ObjectProp,
    Pattern, PropKey, StructDecl, SumTypeDecl, TypeAliasDecl, TypeNode, VarKind, VariableDecl,
};

use super::type_inference::{infer_expr_type, pattern_lead_name, widen_literal};
use super::type_resolution::resolve_type_node;
use crate::binder::{ClassMemberInfo, ClassMemberKind};
use crate::scope::ScopeKind;
use crate::symbol::{Symbol, SymbolKind};
use crate::types::{FunctionType, Type};

impl super::Binder {
    pub(super) fn bind_variable(&mut self, v: &VariableDecl) {
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

    pub(super) fn bind_function(&mut self, f: &FunctionDecl) {
        let line = f.range.start.line;
        let params: Vec<crate::types::FunctionParam> = f
            .params
            .iter()
            .map(|p| {
                let name = super::type_inference::pattern_to_string(&p.pattern);
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

        self.bind_stmt(&f.body);
        self.current = saved;

        let _ = sym_id;
    }

    pub(super) fn bind_type_alias(&mut self, t: &TypeAliasDecl) {
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

    pub(super) fn bind_enum(&mut self, e: &EnumDecl) {
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

    pub(super) fn bind_namespace(&mut self, n: &NamespaceDecl) {
        let mut sym =
            Symbol::new(SymbolKind::Namespace, n.id.clone(), n.range.start.line).with_type(
                Type::named_with_origin(n.id.clone(), Some(self.source_file.clone())),
            );
        sym.doc = n.doc.clone();
        self.define(n.id.clone(), sym);

        let child = self.scopes.child(ScopeKind::Namespace, self.current);
        let saved = self.current;
        self.current = child;

        for d in &n.body {
            self.bind_decl(d);
        }

        let members = self.collect_namespace_members(&n.body);
        if !members.is_empty() {
            self.namespace_members.insert(n.id.clone(), members);
        }

        self.current = saved;
    }

    fn collect_namespace_members(&self, body: &[tsn_core::ast::Decl]) -> Vec<ClassMemberInfo> {
        use tsn_core::ast::Decl;
        let mut members = Vec::new();
        for decl in body {
            match decl {
                Decl::Function(f) => {
                    let ret = f
                        .return_type
                        .as_ref()
                        .map(|m| resolve_type_node(m, Some(self)))
                        .unwrap_or(Type::Void);
                    let params_list = f
                        .params
                        .iter()
                        .map(|p| {
                            let mut ty = p
                                .type_ann
                                .as_ref()
                                .or_else(|| match &p.pattern {
                                    tsn_core::ast::Pattern::Identifier { type_ann, .. } => {
                                        type_ann.as_ref()
                                    }
                                    _ => None,
                                })
                                .map(|ann| resolve_type_node(ann, Some(self)))
                                .unwrap_or(Type::Dynamic);
                            if p.is_rest {
                                if !matches!(ty.0, tsn_core::TypeKind::Array(_)) {
                                    ty = Type::array(ty);
                                }
                            }
                            crate::types::FunctionParam {
                                name: Some(pattern_lead_name(&p.pattern).to_owned()),
                                ty,
                                optional: p.is_optional,
                                is_rest: p.is_rest,
                            }
                        })
                        .collect::<Vec<_>>();
                    let fn_type = Type::fn_(crate::types::FunctionType {
                        params: params_list,
                        return_type: Box::new(ret.clone()),
                        is_arrow: false,
                        type_params: f.type_params.iter().map(|t| t.name.clone()).collect(),
                    });
                    members.push(ClassMemberInfo {
                        name: f.id.clone(),
                        kind: ClassMemberKind::Method,
                        is_async: f.modifiers.is_async,
                        is_static: true,
                        is_optional: false,
                        line: f.range.start.line.saturating_sub(1),
                        col: f.range.start.column,
                        ty: fn_type,
                        members: Vec::new(),
                        visibility: None,
                        is_abstract: false,
                        is_readonly: false,
                        is_override: false,
                    });
                }
                Decl::Class(c) => {
                    let name = c.id.clone().unwrap_or_default();
                    let class_members = self.class_members.get(&name).cloned().unwrap_or_default();
                    members.push(ClassMemberInfo {
                        name: name.clone(),
                        kind: ClassMemberKind::Class,
                        is_async: false,
                        is_static: false,
                        is_optional: false,
                        line: c.range.start.line.saturating_sub(1),
                        col: c.range.start.column,
                        ty: Type::named_with_origin(name, Some(self.source_file.clone())),
                        members: class_members,
                        visibility: None,
                        is_abstract: false,
                        is_readonly: false,
                        is_override: false,
                    });
                }
                Decl::Variable(v) => {
                    for d in &v.declarators {
                        let name = pattern_lead_name(&d.id).to_owned();
                        let ty = d
                            .type_ann
                            .as_ref()
                            .map(|ann| resolve_type_node(ann, Some(self)))
                            .or_else(|| {
                                d.init
                                    .as_ref()
                                    .map(|e| infer_expr_type(e, Some(self)))
                                    .filter(|t| !t.is_dynamic())
                            })
                            .unwrap_or(Type::Dynamic);
                        members.push(ClassMemberInfo {
                            name,
                            kind: ClassMemberKind::Property,
                            is_async: false,
                            is_static: true,
                            is_optional: false,
                            line: d.range.start.line.saturating_sub(1),
                            col: d.range.start.column,
                            ty,
                            members: Vec::new(),
                            visibility: None,
                            is_abstract: false,
                            is_readonly: false,
                            is_override: false,
                        });
                    }
                }
                Decl::Namespace(n) => {
                    let inner_members = self.collect_namespace_members(&n.body);
                    members.push(ClassMemberInfo {
                        name: n.id.clone(),
                        kind: ClassMemberKind::Namespace,
                        is_async: false,
                        is_static: false,
                        is_optional: false,
                        line: n.range.start.line.saturating_sub(1),
                        col: n.range.start.column,
                        ty: Type::named(n.id.clone()),
                        members: inner_members,
                        visibility: None,
                        is_abstract: false,
                        is_readonly: false,
                        is_override: false,
                    });
                }
                Decl::TypeAlias(t) => {
                    members.push(ClassMemberInfo {
                        name: t.id.clone(),
                        kind: ClassMemberKind::Property,
                        is_async: false,
                        is_static: false,
                        is_optional: false,
                        line: t.range.start.line.saturating_sub(1),
                        col: t.range.start.column,
                        ty: Type::named_with_origin(t.id.clone(), Some(self.source_file.clone())),
                        members: Vec::new(),
                        visibility: None,
                        is_abstract: false,
                        is_readonly: false,
                        is_override: false,
                    });
                }
                Decl::Enum(e) => {
                    let variants = self.enum_members.get(&e.id).cloned().unwrap_or_default();
                    members.push(ClassMemberInfo {
                        name: e.id.clone(),
                        kind: ClassMemberKind::Enum,
                        is_async: false,
                        is_static: false,
                        is_optional: false,
                        line: e.range.start.line.saturating_sub(1),
                        col: e.range.start.column,
                        ty: Type::named(e.id.clone()),
                        members: variants,
                        visibility: None,
                        is_abstract: false,
                        is_readonly: false,
                        is_override: false,
                    });
                }
                Decl::Struct(s) => {
                    let struct_members =
                        self.object_members.get(&s.id).cloned().unwrap_or_default();
                    members.push(ClassMemberInfo {
                        name: s.id.clone(),
                        kind: ClassMemberKind::Struct,
                        is_async: false,
                        is_static: false,
                        is_optional: false,
                        line: s.range.start.line.saturating_sub(1),
                        col: s.range.start.column,
                        ty: Type::named(s.id.clone()),
                        members: struct_members,
                        visibility: None,
                        is_abstract: false,
                        is_readonly: false,
                        is_override: false,
                    });
                }
                Decl::Interface(i) => {
                    let interface_members = self
                        .interface_members
                        .get(&i.id)
                        .cloned()
                        .unwrap_or_default();
                    members.push(ClassMemberInfo {
                        name: i.id.clone(),
                        kind: ClassMemberKind::Interface,
                        is_async: false,
                        is_static: false,
                        is_optional: false,
                        line: i.range.start.line.saturating_sub(1),
                        col: i.range.start.column,
                        ty: Type::named(i.id.clone()),
                        members: interface_members,
                        visibility: None,
                        is_abstract: false,
                        is_readonly: false,
                        is_override: false,
                    });
                }
                Decl::Export(e) => {
                    use tsn_core::ast::ExportDecl;
                    match e {
                        ExportDecl::Decl { declaration, .. } => {
                            let inner =
                                self.collect_namespace_members(std::slice::from_ref(declaration));
                            members.extend(inner);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        members
    }

    pub(super) fn bind_struct(&mut self, s: &StructDecl) {
        let mut sym = Symbol::new(SymbolKind::Struct, s.id.clone(), s.range.start.line).with_type(
            Type::named_with_origin(s.id.clone(), Some(self.source_file.clone())),
        );
        sym.doc = s.doc.clone();
        self.define(s.id.clone(), sym);

        let mut members = Vec::new();
        for field in &s.fields {
            let ty = resolve_type_node(&field.type_ann, Some(self));
            members.push(ClassMemberInfo {
                name: field.name.clone(),
                kind: ClassMemberKind::Property,
                is_async: false,
                is_static: false,
                is_optional: false,
                line: field.range.start.line.saturating_sub(1),
                col: field.range.start.column,
                ty,
                members: Vec::new(),
                visibility: None,
                is_abstract: false,
                is_readonly: false,
                is_override: false,
            });
        }
        self.class_members.insert(s.id.clone(), members);
    }

    pub(super) fn bind_extension(&mut self, e: &ExtensionDecl) {
        let type_name = type_node_to_name(&e.target);

        let receiver_ty = resolve_type_node(&e.target, Some(self));

        for member in &e.members {
            match member {
                ExtensionMember::Method(method) => {
                    let mangled = format!("__ext_{}_{}", type_name, method.id);
                    let mut param_types: Vec<crate::types::FunctionParam> =
                        vec![crate::types::FunctionParam {
                            name: Some("this".to_owned()),
                            ty: receiver_ty.clone(),
                            optional: false,
                            is_rest: false,
                        }];
                    for p in &method.params {
                        let mut ty = p
                            .type_ann
                            .as_ref()
                            .or_else(|| match &p.pattern {
                                Pattern::Identifier { type_ann, .. } => type_ann.as_ref(),
                                _ => None,
                            })
                            .map(|ann| resolve_type_node(ann, Some(self)))
                            .unwrap_or(Type::Dynamic);
                        if p.is_rest && !matches!(ty.0, tsn_core::TypeKind::Array(_)) {
                            ty = Type::array(ty);
                        }
                        param_types.push(crate::types::FunctionParam {
                            name: Some(super::type_inference::pattern_to_string(&p.pattern)),
                            ty,
                            optional: p.is_optional,
                            is_rest: p.is_rest,
                        });
                    }
                    let ret_ty = method
                        .return_type
                        .as_ref()
                        .map(|rt| resolve_type_node(rt, Some(self)))
                        .unwrap_or(Type::Void);
                    let fn_type = Type::fn_(crate::types::FunctionType {
                        params: param_types,
                        return_type: Box::new(ret_ty),
                        is_arrow: false,
                        type_params: method.type_params.iter().map(|t| t.name.clone()).collect(),
                    });
                    let line = method.range.start.line;
                    let mut sym =
                        Symbol::new(SymbolKind::Function, mangled.clone(), line).with_type(fn_type);
                    sym.col = method.range.start.column;
                    sym.offset = method.range.start.offset;
                    sym.is_async = method.modifiers.is_async;
                    sym.is_generator = method.modifiers.is_generator;
                    self.define(mangled.clone(), sym);
                    self.extension_methods
                        .entry(type_name.clone())
                        .or_default()
                        .insert(method.id.clone(), mangled.clone());
                    self.bind_extension_function_scope(
                        line,
                        receiver_ty.clone(),
                        &method.params,
                        &method.body,
                    );
                }
                ExtensionMember::Getter {
                    key,
                    return_type,
                    body,
                    range,
                    ..
                } => {
                    let mangled = format!("__extget_{}_{}", type_name, key);
                    let ret_ty = return_type
                        .as_ref()
                        .map(|rt| resolve_type_node(rt, Some(self)))
                        .unwrap_or(Type::Dynamic);
                    let fn_type = Type::fn_(crate::types::FunctionType {
                        params: vec![crate::types::FunctionParam {
                            name: Some("this".to_owned()),
                            ty: receiver_ty.clone(),
                            optional: false,
                            is_rest: false,
                        }],
                        return_type: Box::new(ret_ty),
                        is_arrow: false,
                        type_params: vec![],
                    });
                    let mut sym =
                        Symbol::new(SymbolKind::Function, mangled.clone(), range.start.line)
                            .with_type(fn_type);
                    sym.col = range.start.column;
                    sym.offset = range.start.offset;
                    self.define(mangled.clone(), sym);
                    self.extension_getters
                        .entry(type_name.clone())
                        .or_default()
                        .insert(key.clone(), mangled);
                    self.bind_extension_function_scope(
                        range.start.line,
                        receiver_ty.clone(),
                        &[],
                        body,
                    );
                }
                ExtensionMember::Setter {
                    key,
                    param,
                    body,
                    range,
                    ..
                } => {
                    let mangled = format!("__extset_{}_{}", type_name, key);
                    let param_ty = param
                        .type_ann
                        .as_ref()
                        .or_else(|| match &param.pattern {
                            Pattern::Identifier { type_ann, .. } => type_ann.as_ref(),
                            _ => None,
                        })
                        .map(|ann| resolve_type_node(ann, Some(self)))
                        .unwrap_or(Type::Dynamic);
                    let fn_type = Type::fn_(crate::types::FunctionType {
                        params: vec![
                            crate::types::FunctionParam {
                                name: Some("this".to_owned()),
                                ty: receiver_ty.clone(),
                                optional: false,
                                is_rest: false,
                            },
                            crate::types::FunctionParam {
                                name: Some(super::type_inference::pattern_to_string(
                                    &param.pattern,
                                )),
                                ty: param_ty.clone(),
                                optional: param.is_optional,
                                is_rest: param.is_rest,
                            },
                        ],
                        return_type: Box::new(Type::Void),
                        is_arrow: false,
                        type_params: vec![],
                    });
                    let mut sym =
                        Symbol::new(SymbolKind::Function, mangled.clone(), range.start.line)
                            .with_type(fn_type);
                    sym.col = range.start.column;
                    sym.offset = range.start.offset;
                    self.define(mangled.clone(), sym);
                    self.extension_setters
                        .entry(type_name.clone())
                        .or_default()
                        .insert(key.clone(), mangled);
                    self.bind_extension_function_scope(
                        range.start.line,
                        receiver_ty.clone(),
                        std::slice::from_ref(param),
                        body,
                    );
                }
            }
        }
    }

    fn bind_extension_function_scope(
        &mut self,
        line: u32,
        receiver_ty: Type,
        params: &[tsn_core::ast::Param],
        body: &tsn_core::ast::Stmt,
    ) {
        let child = self.scopes.child(ScopeKind::Function, self.current);
        let saved = self.current;
        self.current = child;

        let this_sym =
            Symbol::new(SymbolKind::Parameter, "this".to_owned(), line).with_type(receiver_ty);
        self.define("this".to_owned(), this_sym);

        for p in params {
            let mut ty = p
                .type_ann
                .as_ref()
                .or_else(|| match &p.pattern {
                    Pattern::Identifier { type_ann, .. } => type_ann.as_ref(),
                    _ => None,
                })
                .map(|ann| resolve_type_node(ann, Some(self)))
                .unwrap_or(Type::Dynamic);
            if p.is_rest && !matches!(ty.0, tsn_core::TypeKind::Array(_)) {
                ty = Type::array(ty);
            }
            self.bind_pattern(&p.pattern, SymbolKind::Parameter, line, None, Some(ty));
            if let Some(def) = &p.default {
                self.bind_expr(def);
            }
        }

        self.bind_stmt(body);
        self.current = saved;
    }
    pub(super) fn collect_object_members(&self, props: &[ObjectProp]) -> Vec<ClassMemberInfo> {
        use crate::types::FunctionType;
        use tsn_core::TypeKind;

        props
            .iter()
            .filter_map(|prop| {
                let range = prop.range();
                match prop {
                    ObjectProp::Property { key, value, .. } => {
                        let name = match key {
                            PropKey::Identifier(s) | PropKey::Str(s) => s.clone(),
                            _ => return None,
                        };
                        let ty = infer_expr_type(value, Some(self));
                        let nested_members = if let Expr::Object { properties, .. } = value {
                            self.collect_object_members(properties)
                        } else {
                            Vec::new()
                        };

                        let kind = if matches!(&ty.0, TypeKind::Fn(_)) {
                            ClassMemberKind::Method
                        } else {
                            ClassMemberKind::Property
                        };

                        Some(ClassMemberInfo {
                            name,
                            kind,
                            is_async: false,
                            is_static: false,
                            is_optional: false,
                            line: range.start.line.saturating_sub(1),
                            col: range.start.column,
                            ty,
                            members: nested_members,
                            visibility: None,
                            is_abstract: false,
                            is_readonly: false,
                            is_override: false,
                        })
                    }
                    ObjectProp::Method {
                        key,
                        params,
                        return_type: ret_ann,
                        body: _,
                        ..
                    } => {
                        let name = match key {
                            PropKey::Identifier(s) | PropKey::Str(s) => s.clone(),
                            _ => return None,
                        };
                        let ret_ty = ret_ann
                            .as_ref()
                            .map(|ann| resolve_type_node(ann, Some(self)))
                            .unwrap_or(Type::Dynamic);

                        let fn_params: Vec<_> = params
                            .iter()
                            .map(|p| crate::types::FunctionParam {
                                name: Some(pattern_lead_name(&p.pattern).to_owned()),
                                ty: p
                                    .type_ann
                                    .as_ref()
                                    .map(|ann| resolve_type_node(ann, Some(self)))
                                    .unwrap_or(Type::Dynamic),
                                optional: p.is_optional,
                                is_rest: p.is_rest,
                            })
                            .collect();

                        let ty = Type::fn_(FunctionType {
                            params: fn_params,
                            return_type: Box::new(ret_ty.clone()),
                            is_arrow: false,
                            type_params: Vec::new(),
                        });

                        Some(ClassMemberInfo {
                            name,
                            kind: ClassMemberKind::Method,
                            is_async: false,
                            is_static: false,
                            is_optional: false,
                            line: range.start.line.saturating_sub(1),
                            col: range.start.column,
                            ty,
                            members: Vec::new(),
                            visibility: None,
                            is_abstract: false,
                            is_readonly: false,
                            is_override: false,
                        })
                    }
                    ObjectProp::Getter { key, .. } => {
                        let name = match key {
                            PropKey::Identifier(s) | PropKey::Str(s) => s.clone(),
                            _ => return None,
                        };
                        Some(ClassMemberInfo {
                            name,
                            kind: ClassMemberKind::Getter,
                            is_async: false,
                            is_static: false,
                            is_optional: false,
                            line: range.start.line.saturating_sub(1),
                            col: range.start.column,
                            ty: Type::Dynamic,
                            members: Vec::new(),
                            visibility: None,
                            is_abstract: false,
                            is_readonly: false,
                            is_override: false,
                        })
                    }
                    ObjectProp::Setter { key, .. } => {
                        let name = match key {
                            PropKey::Identifier(s) | PropKey::Str(s) => s.clone(),
                            _ => return None,
                        };
                        Some(ClassMemberInfo {
                            name,
                            kind: ClassMemberKind::Setter,
                            is_async: false,
                            is_static: false,
                            is_optional: false,
                            line: range.start.line.saturating_sub(1),
                            col: range.start.column,
                            ty: Type::Dynamic,
                            members: Vec::new(),
                            visibility: None,
                            is_abstract: false,
                            is_readonly: false,
                            is_override: false,
                        })
                    }
                    _ => None,
                }
            })
            .collect()
    }

    pub(super) fn bind_pattern(
        &mut self,
        pattern: &Pattern,
        kind: SymbolKind,
        line: u32,
        doc: Option<String>,
        ty: Option<Type>,
    ) {
        match pattern {
            Pattern::Identifier {
                name,
                type_ann,
                range,
            } => {
                let mut sym = Symbol::new(kind, name.clone(), line);
                sym.doc = doc;
                sym.col = range.start.column;
                sym.offset = range.start.offset;
                sym.has_explicit_type = type_ann.is_some() || ty.is_some();
                if let Some(ann) = type_ann {
                    sym.ty = Some(resolve_type_node(ann, Some(self)));
                } else {
                    sym.ty = ty;
                }
                self.define(name.clone(), sym);
            }
            Pattern::Array { elements, rest, .. } => {
                for el in elements.iter().flatten() {
                    self.bind_pattern(&el.pattern, kind, line, doc.clone(), None);
                }
                if let Some(r) = rest {
                    self.bind_pattern(r, kind, line, doc.clone(), None);
                }
            }
            Pattern::Object {
                properties, rest, ..
            } => {
                for prop in properties {
                    let prop_ty =
                        ty.as_ref().and_then(|t| match &t.0 {
                            tsn_core::TypeKind::Object(members) => {
                                members.iter().find_map(|m| match m {
                                    crate::types::ObjectTypeMember::Property {
                                        name, ty, ..
                                    } if name == &prop.key => Some(ty.clone()),
                                    crate::types::ObjectTypeMember::Method {
                                        name,
                                        params,
                                        return_type,
                                        is_arrow,
                                        ..
                                    } if name == &prop.key => {
                                        Some(crate::types::Type::fn_(crate::types::FunctionType {
                                            params: params.clone(),
                                            return_type: return_type.clone(),
                                            is_arrow: *is_arrow,
                                            type_params: vec![],
                                        }))
                                    }
                                    _ => None,
                                })
                            }
                            tsn_core::TypeKind::Named(name, origin)
                            | tsn_core::TypeKind::Generic(name, _, origin) => {
                                use crate::types::TypeContext;
                                self.get_class_members(name, origin.as_deref())
                                    .or_else(|| self.get_interface_members(name, origin.as_deref()))
                                    .and_then(|members| {
                                        members
                                            .iter()
                                            .find(|m| m.name == prop.key)
                                            .map(|m| m.ty.clone())
                                    })
                            }
                            _ => None,
                        });
                    self.bind_pattern(&prop.value, kind, line, doc.clone(), prop_ty);
                }
                if let Some(r) = rest {
                    self.bind_pattern(r, kind, line, doc.clone(), None);
                }
            }
            Pattern::Assignment { left, .. } => {
                self.bind_pattern(left, kind, line, doc, ty);
            }
            Pattern::Rest { argument, .. } => {
                self.bind_pattern(argument, kind, line, doc, None);
            }
        }
    }

    pub(super) fn bind_sum_type(&mut self, t: &SumTypeDecl) {
        let pe_sym =
            Symbol::new(SymbolKind::TypeAlias, t.id.clone(), t.range.start.line).with_type(
                Type::named_with_origin(t.id.clone(), Some(self.source_file.clone())),
            );
        self.define(t.id.clone(), pe_sym);

        let mut variant_names = Vec::new();

        for v in &t.variants {
            variant_names.push(v.name.clone());

            let fields: Vec<(String, Type)> = v
                .fields
                .iter()
                .map(|f| {
                    let ty = resolve_type_node(&f.ty, Some(self));
                    (f.name.clone(), ty)
                })
                .collect();

            self.sum_variant_parent.insert(v.name.clone(), t.id.clone());
            self.sum_variant_fields
                .insert(v.name.clone(), fields.clone());

            if v.fields.is_empty() {
                let sym =
                    Symbol::new(SymbolKind::Const, v.name.clone(), v.range.start.line).with_type(
                        Type::named_with_origin(t.id.clone(), Some(self.source_file.clone())),
                    );
                self.define(v.name.clone(), sym);
            } else {
                let params: Vec<crate::types::FunctionParam> = fields
                    .iter()
                    .map(|(fname, fty)| crate::types::FunctionParam {
                        name: Some(fname.clone()),
                        ty: fty.clone(),
                        optional: false,
                        is_rest: false,
                    })
                    .collect();
                let fn_ty = Type::fn_(crate::types::FunctionType {
                    params,
                    return_type: Box::new(Type::named_with_origin(
                        t.id.clone(),
                        Some(self.source_file.clone()),
                    )),
                    is_arrow: false,
                    type_params: t.type_params.iter().map(|tp| tp.name.clone()).collect(),
                });
                let sym = Symbol::new(SymbolKind::Function, v.name.clone(), v.range.start.line)
                    .with_type(fn_ty);
                self.define(v.name.clone(), sym);
            }
        }

        self.sum_type_variants.insert(t.id.clone(), variant_names);
    }
}

pub(super) fn type_node_to_name(node: &TypeNode) -> String {
    use crate::types::well_known;
    use tsn_core::TypeKind;
    match &node.kind {
        TypeKind::Int => well_known::INT.to_owned(),
        TypeKind::Float => well_known::FLOAT.to_owned(),
        TypeKind::Str => well_known::STR.to_owned(),
        TypeKind::Bool => well_known::BOOL.to_owned(),
        TypeKind::Char => well_known::CHAR.to_owned(),
        TypeKind::Named(n, _) => n.clone(),
        TypeKind::Generic(n, _, _) => n.clone(),
        TypeKind::Array(_) => well_known::ARRAY.to_owned(),
        _ => well_known::DYNAMIC.to_owned(),
    }
}
