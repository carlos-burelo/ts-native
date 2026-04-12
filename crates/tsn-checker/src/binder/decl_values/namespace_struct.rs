use tsn_core::ast::{NamespaceDecl, StructDecl};

use super::super::type_resolution::resolve_type_node;
use crate::binder::{infer_expr_type, pattern_lead_name, ClassMemberInfo, ClassMemberKind};
use crate::scope::ScopeKind;
use crate::symbol::{Symbol, SymbolKind};
use crate::types::Type;

impl super::super::Binder {
    pub(crate) fn bind_namespace(&mut self, n: &NamespaceDecl) {
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

    pub(crate) fn bind_struct(&mut self, s: &StructDecl) {
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
}
