use std::collections::HashMap;

use tsn_checker::types::FunctionType;
use tsn_core::TypeKind;

use crate::document::{MemberKind, MemberRecord};

pub fn build_extension_members(
    bind: &tsn_checker::BindResult,
) -> HashMap<String, Vec<MemberRecord>> {
    let mut out: HashMap<String, Vec<MemberRecord>> = HashMap::new();
    let scope = bind.scopes.get(bind.global_scope);

    build_method_members(bind, scope, &mut out);
    build_accessor_members(bind, scope, &mut out);

    out
}

fn build_method_members(
    bind: &tsn_checker::BindResult,
    scope: &tsn_checker::Scope,
    out: &mut HashMap<String, Vec<MemberRecord>>,
) {
    for (type_name, methods) in &bind.extension_methods {
        let records = out.entry(type_name.clone()).or_default();
        for (method_name, mangled) in methods {
            let Some(sid) = scope.resolve(mangled, &bind.scopes) else {
                continue;
            };
            let sym = bind.arena.get(sid);
            let Some(tsn_checker::Type(TypeKind::Fn(ft))) = &sym.ty else {
                continue;
            };

            let mut params = ft.params.clone();
            if params.first().and_then(|p| p.name.as_deref()) == Some("this") {
                params.remove(0);
            }

            let params_str = params
                .iter()
                .map(|p| format!("{}: {}", p.name.as_deref().unwrap_or("arg"), p.ty))
                .collect::<Vec<_>>()
                .join(", ");

            records.push(MemberRecord {
                name: method_name.clone(),
                type_str: ft.return_type.to_string(),
                params_str,
                is_static: false,
                is_optional: false,
                kind: MemberKind::Method,
                is_arrow: ft.is_arrow,
                line: sym.line.saturating_sub(1),
                col: sym.col,
                init_value: String::new(),
                ty: tsn_checker::Type(TypeKind::Fn(FunctionType {
                    params,
                    return_type: ft.return_type.clone(),
                    is_arrow: ft.is_arrow,
                    type_params: ft.type_params.clone(),
                })),
                members: Vec::new(),
            });
        }
    }
}

fn build_accessor_members(
    bind: &tsn_checker::BindResult,
    scope: &tsn_checker::Scope,
    out: &mut HashMap<String, Vec<MemberRecord>>,
) {
    for (type_name, getters) in &bind.extension_getters {
        let records = out.entry(type_name.clone()).or_default();
        for (getter_name, mangled) in getters {
            let Some(sid) = scope.resolve(mangled, &bind.scopes) else {
                continue;
            };
            let sym = bind.arena.get(sid);
            let Some(tsn_checker::Type(TypeKind::Fn(ft))) = &sym.ty else {
                continue;
            };
            records.push(MemberRecord {
                name: getter_name.clone(),
                type_str: ft.return_type.to_string(),
                params_str: String::new(),
                is_static: false,
                is_optional: false,
                kind: MemberKind::Getter,
                is_arrow: false,
                line: sym.line.saturating_sub(1),
                col: sym.col,
                init_value: String::new(),
                ty: ft.return_type.as_ref().clone(),
                members: Vec::new(),
            });
        }
    }

    for (type_name, setters) in &bind.extension_setters {
        let records = out.entry(type_name.clone()).or_default();
        for (setter_name, mangled) in setters {
            let Some(sid) = scope.resolve(mangled, &bind.scopes) else {
                continue;
            };
            let sym = bind.arena.get(sid);
            let Some(tsn_checker::Type(TypeKind::Fn(ft))) = &sym.ty else {
                continue;
            };
            let params = ft
                .params
                .iter()
                .skip_while(|p| p.name.as_deref() == Some("this"))
                .map(|p| format!("{}: {}", p.name.as_deref().unwrap_or("arg"), p.ty))
                .collect::<Vec<_>>()
                .join(", ");
            records.push(MemberRecord {
                name: setter_name.clone(),
                type_str: ft.return_type.to_string(),
                params_str: params,
                is_static: false,
                is_optional: false,
                kind: MemberKind::Setter,
                is_arrow: false,
                line: sym.line.saturating_sub(1),
                col: sym.col,
                init_value: String::new(),
                ty: ft.return_type.as_ref().clone(),
                members: Vec::new(),
            });
        }
    }
}
