use crate::binder::{infer_expr_type, resolve_type_node};
use crate::types::{well_known, FunctionType, Type, TypeContext};
use std::collections::HashMap;
use tsn_core::ast::{Arg, Expr};
use tsn_core::TypeKind;

const STATIC_NEW: &str = "new";

pub(crate) fn infer_call_type(
    fn_map: &HashMap<String, Type>,
    fn_type_params: &HashMap<String, Vec<String>>,
    class_methods: &HashMap<String, HashMap<String, Type>>,
    sym_map: &HashMap<String, Type>,
    expr: &Expr,
    ctx: Option<&dyn TypeContext>,
) -> Option<Type> {
    match expr {
        Expr::Call {
            callee,
            type_args,
            args,
            ..
        } => match callee.as_ref() {
            Expr::Identifier { name, .. } => {
                let ret = fn_map.get(name.as_str())?.clone();

                if let Some(tparams) = fn_type_params.get(name.as_str()) {
                    if !tparams.is_empty() {
                        let mapping: HashMap<String, Type> = if !type_args.is_empty()
                            && type_args.len() == tparams.len()
                        {
                            tparams
                                .iter()
                                .zip(type_args.iter().map(|a| resolve_type_node(a, ctx)))
                                .map(|(k, v)| (k.clone(), v))
                                .collect()
                        } else if type_args.is_empty() {
                            tparams
                                .iter()
                                .zip(args.iter())
                                .filter_map(|(tp, arg)| {
                                    let expr = match arg {
                                        Arg::Positional(e) => e,
                                        Arg::Named { value, .. } => value,
                                        Arg::Spread(_) => return None,
                                    };
                                    let ty = infer_expr_type(expr, ctx);
                                    if !ty.is_dynamic() {
                                        Some((tp.clone(), ty))
                                    } else if let Expr::Identifier { name: arg_name, .. } = expr {
                                        sym_map
                                            .get(arg_name.as_str())
                                            .map(|t| (tp.clone(), t.clone()))
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        } else {
                            HashMap::new()
                        };
                        if !mapping.is_empty() {
                            return Some(ret.substitute(&mapping));
                        }
                    }
                }
                Some(ret)
            }

            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let method_name = match property.as_ref() {
                    Expr::Identifier { name, .. } => name.as_str(),
                    _ => return None,
                };
                let receiver_ty = infer_receiver_type(
                    fn_map,
                    fn_type_params,
                    class_methods,
                    sym_map,
                    object,
                    ctx,
                )?;
                let (class_name, origin): (&str, Option<&str>) = match &receiver_ty.0 {
                    TypeKind::Named(n, origin) => (n.as_str(), origin.as_deref()),
                    TypeKind::Generic(name, _, origin) => (name.as_str(), origin.as_deref()),
                    _ => (receiver_ty.stdlib_key()?, None),
                };

                let method_ty = if let Some(m) = class_methods.get(class_name) {
                    m.get(method_name).cloned()
                } else if let Some(ctx) = ctx {
                    ctx.get_class_members(class_name, origin)
                        .and_then(|members| {
                            members
                                .iter()
                                .find(|m| m.name == method_name)
                                .map(|m| m.ty.clone())
                        })
                } else {
                    None
                };

                match method_ty {
                    Some(t) => match &t.0 {
                        TypeKind::Fn(FunctionType { return_type, .. }) => {
                            Some(*return_type.clone())
                        }
                        _ => Some(t),
                    },
                    None => None,
                }
            }
            _ => None,
        },
        Expr::Paren { expression, .. } => infer_call_type(
            fn_map,
            fn_type_params,
            class_methods,
            sym_map,
            expression,
            ctx,
        ),

        Expr::Await { argument, .. } => {
            let inner = infer_call_type(
                fn_map,
                fn_type_params,
                class_methods,
                sym_map,
                argument,
                ctx,
            )?;
            match &inner.0 {
                TypeKind::Generic(name, args, _origin)
                    if name == well_known::FUTURE && args.len() == 1 =>
                {
                    Some(args[0].clone())
                }
                _ => Some(inner),
            }
        }

        Expr::New { callee, .. } => match callee.as_ref() {
            Expr::Identifier { name, .. } => Some(Type::named(name.clone())),

            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => match (object.as_ref(), property.as_ref()) {
                (Expr::Identifier { name, .. }, Expr::Identifier { name: method, .. })
                    if method == STATIC_NEW =>
                {
                    Some(Type::named(name.clone()))
                }
                _ => None,
            },
            _ => None,
        },

        _ => None,
    }
}

pub(crate) fn infer_receiver_type(
    fn_map: &HashMap<String, Type>,
    fn_type_params: &HashMap<String, Vec<String>>,
    class_methods: &HashMap<String, HashMap<String, Type>>,
    sym_map: &HashMap<String, Type>,
    expr: &Expr,
    ctx: Option<&dyn TypeContext>,
) -> Option<Type> {
    match expr {
        Expr::Identifier { name, .. } => sym_map.get(name.as_str()).cloned(),
        Expr::Call { .. } | Expr::Member { .. } => {
            infer_call_type(fn_map, fn_type_params, class_methods, sym_map, expr, ctx)
        }
        Expr::Paren { expression, .. } => infer_receiver_type(
            fn_map,
            fn_type_params,
            class_methods,
            sym_map,
            expression,
            ctx,
        ),
        _ => None,
    }
}
