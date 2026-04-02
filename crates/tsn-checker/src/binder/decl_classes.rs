use super::type_inference::pattern_lead_name;
use super::type_resolution::resolve_type_node;
use crate::binder::{ClassMemberInfo, ClassMemberKind};
use crate::symbol::{Symbol, SymbolKind};
use crate::types::Type;
use std::collections::HashMap;
use tsn_core::ast::{ClassDecl, ClassMember, InterfaceDecl, InterfaceMember};

impl super::Binder {
    pub(super) fn bind_class(&mut self, c: &ClassDecl) {
        let name = c.id.clone().unwrap_or_else(|| "<anon>".to_owned());
        let line = c.range.start.line;
        let cls_type = Type::named_with_origin(name.clone(), Some(self.source_file.clone()));
        let mut sym = Symbol::new(SymbolKind::Class, name.clone(), line).with_type(cls_type);
        sym.col = c.range.start.column;
        sym.offset = c.range.start.offset;
        sym.doc = c.doc.clone();
        sym.type_params = c.type_params.iter().map(|t| t.name.clone()).collect();
        sym.type_param_constraints = c
            .type_params
            .iter()
            .map(|t| {
                t.constraint
                    .as_ref()
                    .map(|con| resolve_type_node(con, Some(self)))
            })
            .collect();
        self.define(name.clone(), sym);
        let mut methods: HashMap<String, Type> = HashMap::new();
        let mut members: Vec<ClassMemberInfo> = Vec::new();
        for member in &c.body {
            match member {
                ClassMember::Constructor { params, range, .. } => {
                    let params_list = params
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
                                .map(|m| resolve_type_node(m, Some(self)))
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
                    let ctor_fn_type = Type::fn_(crate::types::FunctionType {
                        params: params_list,
                        return_type: Box::new(Type::named(name.clone())),
                        is_arrow: false,
                        type_params: vec![],
                    });
                    members.push(ClassMemberInfo {
                        name: "constructor".to_owned(),
                        kind: ClassMemberKind::Constructor,
                        is_async: false,
                        is_static: false,
                        is_optional: false,
                        line: range.start.line.saturating_sub(1),
                        col: range.start.column,
                        ty: ctor_fn_type,
                        members: Vec::new(),
                        visibility: None,
                        is_abstract: false,
                        is_readonly: false,
                        is_override: false,
                    });
                }
                ClassMember::Method {
                    key,
                    params,
                    return_type,
                    type_params: method_type_params,
                    modifiers,
                    range,
                    ..
                } => {
                    let ret = return_type
                        .as_ref()
                        .map(|m| resolve_type_node(m, Some(self)))
                        .unwrap_or(Type::Void);
                    let effective_ret = wrap_async_return(ret.clone(), modifiers.is_async);
                    let params_list = params
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
                                .map(|m| resolve_type_node(m, Some(self)))
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
                        return_type: Box::new(effective_ret),
                        is_arrow: false,
                        type_params: method_type_params.iter().map(|t| t.name.clone()).collect(),
                    });

                    methods.insert(key.clone(), fn_type.clone());
                    members.push(ClassMemberInfo {
                        name: key.clone(),
                        kind: ClassMemberKind::Method,
                        is_async: modifiers.is_async,
                        is_static: modifiers.is_static,
                        is_optional: false,
                        line: range.start.line.saturating_sub(1),
                        col: range.start.column,
                        ty: fn_type,
                        members: Vec::new(),
                        visibility: modifiers.visibility,
                        is_abstract: modifiers.is_abstract,
                        is_readonly: false,
                        is_override: modifiers.is_override,
                    });
                }
                ClassMember::Property {
                    key,
                    type_ann,
                    modifiers,
                    range,
                    ..
                } => {
                    let ty = type_ann
                        .as_ref()
                        .map(|m| resolve_type_node(m, Some(self)))
                        .unwrap_or(Type::Dynamic);
                    members.push(ClassMemberInfo {
                        name: key.clone(),
                        kind: ClassMemberKind::Property,
                        is_async: false,
                        is_static: modifiers.is_static,
                        is_optional: false,
                        line: range.start.line.saturating_sub(1),
                        col: range.start.column,
                        ty,
                        members: Vec::new(),
                        visibility: modifiers.visibility,
                        is_abstract: false,
                        is_readonly: modifiers.is_readonly,
                        is_override: false,
                    });
                }
                ClassMember::Getter {
                    key,
                    return_type,
                    modifiers,
                    range,
                    ..
                } => {
                    let ty = return_type
                        .as_ref()
                        .map(|m| resolve_type_node(m, Some(self)))
                        .unwrap_or(Type::Void);
                    members.push(ClassMemberInfo {
                        name: key.clone(),
                        kind: ClassMemberKind::Getter,
                        is_async: false,
                        is_static: modifiers.is_static,
                        is_optional: false,
                        line: range.start.line.saturating_sub(1),
                        col: range.start.column,
                        ty,
                        members: Vec::new(),
                        visibility: modifiers.visibility,
                        is_abstract: modifiers.is_abstract,
                        is_readonly: false,
                        is_override: modifiers.is_override,
                    });
                }
                ClassMember::Setter {
                    key,
                    modifiers,
                    range,
                    ..
                } => {
                    members.push(ClassMemberInfo {
                        name: key.clone(),
                        kind: ClassMemberKind::Setter,
                        is_async: false,
                        is_static: modifiers.is_static,
                        is_optional: false,
                        line: range.start.line.saturating_sub(1),
                        col: range.start.column,
                        ty: Type::Void,
                        members: Vec::new(),
                        visibility: modifiers.visibility,
                        is_abstract: false,
                        is_readonly: false,
                        is_override: modifiers.is_override,
                    });
                }
                ClassMember::Destructor { .. } | ClassMember::StaticBlock { .. } => {}
            }
        }

        for member in &c.body {
            match member {
                ClassMember::Constructor {
                    params,
                    body,
                    range,
                } => {
                    self.bind_inline_function(params, None, body, range);
                }
                ClassMember::Method {
                    params,
                    return_type,
                    body: Some(body),
                    range,
                    ..
                } => {
                    self.bind_inline_function(params, return_type.as_ref(), body, range);
                }
                ClassMember::Getter {
                    return_type,
                    body: Some(body),
                    range,
                    ..
                } => {
                    self.bind_inline_function(&[], return_type.as_ref(), body, range);
                }
                ClassMember::Setter {
                    param,
                    body: Some(body),
                    range,
                    ..
                } => {
                    self.bind_inline_function(std::slice::from_ref(param), None, body, range);
                }
                _ => {}
            }
        }

        if !methods.is_empty() {
            self.class_methods.insert(name.clone(), methods);
        }
        if !members.is_empty() {
            self.class_members.insert(name.clone(), members.clone());
        }

        if let Some(tsn_core::ast::Expr::Identifier {
            name: parent_name, ..
        }) = &c.super_class
        {
            self.class_parents.insert(name.clone(), parent_name.clone());
        }

        if let Some(tsn_core::ast::Expr::Identifier {
            name: parent_name, ..
        }) = &c.super_class
        {
            if let Some(parent_members) = self.class_members.get(parent_name.as_str()).cloned() {
                for m in &members {
                    if m.is_override {
                        let exists = parent_members
                            .iter()
                            .any(|pm| pm.name == m.name && !pm.is_static);
                        if !exists {
                            self.override_errors.push((
                                m.name.clone(),
                                name.clone(),
                                parent_name.clone(),
                                m.line,
                                m.col,
                            ));
                        }
                    }
                }
            }
        }

        if let Some(tsn_core::ast::Expr::Identifier {
            name: parent_name, ..
        }) = &c.super_class
        {
            if !c.super_type_args.is_empty() {
                let resolved_args: Vec<Type> = c
                    .super_type_args
                    .iter()
                    .map(|ta| resolve_type_node(ta, Some(self)))
                    .collect();
                let parent_tp: Vec<String> = self
                    .scopes
                    .get(self.current)
                    .resolve(parent_name, &self.scopes)
                    .map(|sid| self.arena.get(sid).type_params.clone())
                    .unwrap_or_default();
                if parent_tp.len() == resolved_args.len() && !parent_tp.is_empty() {
                    let mapping: HashMap<String, Type> = parent_tp
                        .iter()
                        .zip(resolved_args.iter())
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    if let Some(parent_members) =
                        self.class_members.get(parent_name.as_str()).cloned()
                    {
                        let own_members = self.class_members.entry(name.clone()).or_default();
                        for m in &parent_members {
                            if !own_members.iter().any(|om| om.name == m.name) {
                                let mut inherited = m.clone();
                                inherited.ty = m.ty.substitute(&mapping);
                                own_members.push(inherited);
                            }
                        }
                    }
                }
            } else if let Some(parent_members) =
                self.class_members.get(parent_name.as_str()).cloned()
            {
                let own_members = self.class_members.entry(name.clone()).or_default();
                for m in &parent_members {
                    if !own_members.iter().any(|om| om.name == m.name) {
                        own_members.push(m.clone());
                    }
                }
            }
        }

        let own_members = self.class_members.get(&name).cloned().unwrap_or_default();
        self.flattened_members.insert(name.clone(), own_members);
    }

    pub(super) fn bind_interface(&mut self, i: &InterfaceDecl) {
        let mut sym = Symbol::new(SymbolKind::Interface, i.id.clone(), i.range.start.line);
        sym.col = i.range.start.column;
        sym.offset = i.range.start.offset;
        sym.doc = i.doc.clone();
        sym.type_params = i.type_params.iter().map(|t| t.name.clone()).collect();
        sym.type_param_constraints = i
            .type_params
            .iter()
            .map(|t| {
                t.constraint
                    .as_ref()
                    .map(|con| resolve_type_node(con, Some(self)))
            })
            .collect();
        self.define(i.id.clone(), sym);

        let mut members: Vec<ClassMemberInfo> = Vec::new();
        for member in &i.body {
            match member {
                InterfaceMember::Property {
                    key,
                    type_ann,
                    optional,
                    range,
                    ..
                } => {
                    let ty = resolve_type_node(type_ann, Some(self));
                    members.push(ClassMemberInfo {
                        name: key.clone(),
                        kind: ClassMemberKind::Property,
                        is_async: false,
                        is_static: false,
                        is_optional: *optional,
                        line: range.start.line.saturating_sub(1),
                        col: range.start.column,
                        ty,
                        members: Vec::new(),
                        visibility: None,
                        is_abstract: false,
                        is_readonly: false,
                        is_override: false,
                    });
                }
                InterfaceMember::Method {
                    key,
                    params,
                    return_type,
                    optional,
                    range,
                    ..
                } => {
                    let ret = return_type
                        .as_ref()
                        .map(|m| resolve_type_node(m, Some(self)))
                        .unwrap_or(Type::Dynamic);
                    let params_list = params
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
                                .map(|m| resolve_type_node(m, Some(self)))
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
                        return_type: Box::new(ret),
                        is_arrow: false,
                        type_params: vec![],
                    });

                    members.push(ClassMemberInfo {
                        name: key.clone(),
                        kind: ClassMemberKind::Method,
                        is_async: false,
                        is_static: false,
                        is_optional: *optional,
                        line: range.start.line.saturating_sub(1),
                        col: range.start.column,
                        ty: fn_type,
                        members: Vec::new(),
                        visibility: None,
                        is_abstract: false,
                        is_readonly: false,
                        is_override: false,
                    });
                }
                InterfaceMember::Index {
                    param,
                    return_type,
                    range,
                    ..
                } => {
                    let ret = resolve_type_node(return_type, Some(self));
                    let param_name = pattern_lead_name(&param.pattern).to_owned();
                    let key_ty = param
                        .type_ann
                        .as_ref()
                        .map(|m| resolve_type_node(m, Some(self)))
                        .unwrap_or(Type::Dynamic);
                    members.push(ClassMemberInfo {
                        name: format!("[{}: {}]", param_name, key_ty),
                        kind: ClassMemberKind::Property,
                        is_async: false,
                        is_static: false,
                        is_optional: false,
                        line: range.start.line.saturating_sub(1),
                        col: range.start.column,
                        ty: ret,
                        members: Vec::new(),
                        visibility: None,
                        is_abstract: false,
                        is_readonly: false,
                        is_override: false,
                    });
                }
                InterfaceMember::Callable { .. } => {}
            }
        }

        if !members.is_empty() {
            self.interface_members.insert(i.id.clone(), members.clone());
            self.flattened_members.insert(i.id.clone(), members);
        }
    }
}

fn wrap_async_return(ret: Type, is_async: bool) -> Type {
    if !is_async {
        return ret;
    }
    let already_future = matches!(&ret.0, tsn_core::TypeKind::Generic(name, _, _) if name == tsn_core::well_known::FUTURE);
    if already_future || ret.is_dynamic() || ret == Type::Void {
        ret
    } else {
        Type::generic(tsn_core::well_known::FUTURE.to_owned(), vec![ret])
    }
}
