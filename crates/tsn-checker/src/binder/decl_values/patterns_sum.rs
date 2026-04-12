use tsn_core::ast::{Pattern, SumTypeDecl};

use super::super::type_resolution::resolve_type_node;
use crate::symbol::{Symbol, SymbolKind};
use crate::types::Type;

impl super::super::Binder {
    pub(crate) fn bind_pattern(
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

    pub(crate) fn bind_sum_type(&mut self, t: &SumTypeDecl) {
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
