use tsn_core::ast::{Expr, ObjectProp, PropKey};

use super::super::type_inference::infer_expr_type;
use super::super::type_resolution::resolve_type_node;
use crate::binder::{pattern_lead_name, ClassMemberInfo, ClassMemberKind};
use crate::types::Type;

impl super::super::Binder {
    pub(crate) fn collect_object_members(&self, props: &[ObjectProp]) -> Vec<ClassMemberInfo> {
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
}
