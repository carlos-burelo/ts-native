use crate::binder::{infer_expr_type, pattern_lead_name, BindResult};
use crate::checker::Checker;
use crate::types::{ClassMemberInfo, FunctionParam, FunctionType, Type, TypeContext};
use std::collections::HashMap;
use tsn_core::ast::{Expr, Stmt};
use tsn_core::TypeKind;

pub(crate) fn build_generic_mapping(
    class_name: &str,
    type_args: &[Type],
    checker: &mut Checker,
    bind: &BindResult,
) -> HashMap<String, Type> {
    let type_params = checker.symbol_type_params_any(class_name, bind);
    type_params
        .iter()
        .zip(type_args.iter())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

pub(crate) fn build_call_mapping(
    callee: &Expr,
    type_args: &[tsn_core::ast::TypeNode],
    args: &[tsn_core::ast::Arg],
    ft: &FunctionType,
    checker: &mut Checker,
    bind: &BindResult,
) -> HashMap<String, Type> {
    let fn_type_params: Vec<String> = if !ft.type_params.is_empty() {
        ft.type_params.clone()
    } else if let Expr::Identifier { name, .. } = callee {
        checker.symbol_type_params(name, crate::symbol::SymbolKind::Function, bind)
    } else {
        Vec::new()
    };

    if fn_type_params.is_empty() {
        return HashMap::new();
    }

    if !type_args.is_empty() && type_args.len() == fn_type_params.len() {
        fn_type_params
            .iter()
            .zip(
                type_args
                    .iter()
                    .map(|a| checker.resolve_type_node_cached(a, bind)),
            )
            .map(|(k, v)| (k.clone(), v))
            .collect()
    } else if type_args.is_empty() {
        infer_mapping_from_args(&fn_type_params, &ft.params, args, checker, bind)
    } else {
        HashMap::new()
    }
}

pub(crate) fn infer_mapping_from_args(
    type_params: &[String],
    param_types: &[FunctionParam],
    args: &[tsn_core::ast::Arg],
    checker: &mut Checker,
    bind: &BindResult,
) -> HashMap<String, Type> {
    let mut mapping = HashMap::new();
    for (param, arg) in param_types.iter().zip(args.iter()) {
        let arg_ty = match arg {
            tsn_core::ast::Arg::Positional(e) => {
                // For Fn-typed params, try contextual typing: use the expected param
                // types to infer the concrete return type of untyped arrow callbacks.
                if let TypeKind::Fn(expected_fn) = &param.ty.0 {
                    if let Some(concrete) = infer_arrow_with_context(e, expected_fn, checker, bind)
                    {
                        collect_type_inferences(&param.ty, &concrete, type_params, &mut mapping);
                        continue;
                    }
                }
                checker.infer_type(e, bind)
            }
            tsn_core::ast::Arg::Named { value, .. } => checker.infer_type(value, bind),
            tsn_core::ast::Arg::Spread(_) => continue,
        };
        collect_type_inferences(&param.ty, &arg_ty, type_params, &mut mapping);
    }
    mapping
}

/// Infer a concrete FunctionType for an arrow by providing the expected parameter
/// types as context. This enables type inference for callbacks with untyped params,
/// e.g. `flatMap((a) => a)` where `a: T` is known from the method signature.
fn infer_arrow_with_context(
    expr: &Expr,
    expected_fn: &FunctionType,
    checker: &mut Checker,
    bind: &BindResult,
) -> Option<Type> {
    let Expr::Arrow { params, body, .. } = expr else {
        return None;
    };

    // Map param names → expected types for untyped params only
    let mut locals: HashMap<String, Type> = HashMap::new();
    for (ap, ep) in params.iter().zip(expected_fn.params.iter()) {
        let has_ann = ap.type_ann.is_some()
            || matches!(
                &ap.pattern,
                tsn_core::ast::Pattern::Identifier {
                    type_ann: Some(_),
                    ..
                }
            );
        if has_ann || ep.ty.is_dynamic() {
            continue;
        }
        let name = pattern_lead_name(&ap.pattern).to_owned();
        locals.insert(name, ep.ty.clone());
    }

    if locals.is_empty() {
        return None;
    }

    let ctx = LocalContext { bind, locals };
    let ret_ty = match body.as_ref() {
        tsn_core::ast::ArrowBody::Expr(e) => infer_expr_type(e, Some(&ctx)),
        tsn_core::ast::ArrowBody::Block(block) => {
            let mut return_tys: Vec<Type> = Vec::new();
            collect_block_return_types(block, &ctx, &mut return_tys);
            match return_tys.len() {
                0 => Type::Void,
                1 => return_tys.into_iter().next().unwrap(),
                _ => Type::union(return_tys),
            }
        }
    };

    if ret_ty.is_dynamic() {
        return None;
    }

    // Build a concrete FunctionType with the resolved return type
    let concrete_params: Vec<FunctionParam> = params
        .iter()
        .zip(expected_fn.params.iter())
        .map(|(ap, ep)| {
            let ty = ap
                .type_ann
                .as_ref()
                .map(|ann| checker.resolve_type_node_cached(ann, bind))
                .unwrap_or_else(|| ep.ty.clone());
            FunctionParam {
                name: Some(pattern_lead_name(&ap.pattern).to_owned()),
                ty,
                optional: ap.is_optional,
                is_rest: ap.is_rest,
            }
        })
        .collect();

    Some(Type::fn_(FunctionType {
        params: concrete_params,
        return_type: Box::new(ret_ty),
        is_arrow: true,
        type_params: Vec::new(),
    }))
}

/// Recursively collect inferred return types from all `return` statements
/// within a block, using the given local context for type inference.
/// Does NOT descend into nested function/arrow bodies.
fn collect_block_return_types(stmt: &Stmt, ctx: &LocalContext<'_>, out: &mut Vec<Type>) {
    match stmt {
        Stmt::Block { stmts, .. } => {
            for s in stmts {
                collect_block_return_types(s, ctx, out);
            }
        }
        Stmt::Return {
            argument: Some(e), ..
        } => {
            let ty = infer_expr_type(e, Some(ctx));
            if !ty.is_dynamic() {
                out.push(ty);
            }
        }
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            collect_block_return_types(consequent, ctx, out);
            if let Some(alt) = alternate {
                collect_block_return_types(alt, ctx, out);
            }
        }
        Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
            collect_block_return_types(body, ctx, out);
        }
        Stmt::For { body, .. } | Stmt::ForIn { body, .. } | Stmt::ForOf { body, .. } => {
            collect_block_return_types(body, ctx, out);
        }
        Stmt::Try {
            block,
            catch,
            finally,
            ..
        } => {
            collect_block_return_types(block, ctx, out);
            if let Some(c) = catch {
                collect_block_return_types(c.body.as_ref(), ctx, out);
            }
            if let Some(f) = finally {
                collect_block_return_types(f, ctx, out);
            }
        }
        Stmt::Labeled { body, .. } => collect_block_return_types(body, ctx, out),
        Stmt::Switch { cases, .. } => {
            for case in cases {
                for s in &case.body {
                    collect_block_return_types(s, ctx, out);
                }
            }
        }
        // Nested function/arrow bodies have their own return context — skip
        _ => {}
    }
}

/// A TypeContext that overlays local name→type bindings on top of a BindResult.
struct LocalContext<'a> {
    bind: &'a BindResult,
    locals: HashMap<String, Type>,
}

impl TypeContext for LocalContext<'_> {
    fn get_interface_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<ClassMemberInfo>> {
        self.bind.get_interface_members(name, origin)
    }

    fn get_class_members(&self, name: &str, origin: Option<&str>) -> Option<Vec<ClassMemberInfo>> {
        self.bind.get_class_members(name, origin)
    }

    fn get_namespace_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<ClassMemberInfo>> {
        self.bind.get_namespace_members(name, origin)
    }

    fn resolve_symbol(&self, name: &str) -> Option<Type> {
        self.locals
            .get(name)
            .cloned()
            .or_else(|| self.bind.resolve_symbol(name))
    }

    fn source_file(&self) -> Option<&str> {
        self.bind.source_file()
    }

    fn get_alias_node(&self, name: &str) -> Option<(Vec<String>, tsn_core::ast::TypeNode)> {
        self.bind.get_alias_node(name)
    }
}

pub(crate) fn collect_type_inferences(
    pattern: &Type,
    concrete: &Type,
    params: &[String],
    out: &mut HashMap<String, Type>,
) {
    match &pattern.0 {
        TypeKind::Named(name, _origin) if params.contains(name) => {
            out.entry(name.clone()).or_insert_with(|| concrete.clone());
        }
        TypeKind::Array(inner) => {
            if let TypeKind::Array(c_inner) = &concrete.0 {
                collect_type_inferences(inner, c_inner, params, out);
            }
        }
        TypeKind::Generic(_, inner_args, _origin) => {
            if let TypeKind::Generic(_, c_args, _c_origin) = &concrete.0 {
                for (ip, cp) in inner_args.iter().zip(c_args.iter()) {
                    collect_type_inferences(ip, cp, params, out);
                }
            }
        }
        TypeKind::Fn(ft) => {
            if let TypeKind::Fn(c_ft) = &concrete.0 {
                for (pp, cp) in ft.params.iter().zip(c_ft.params.iter()) {
                    collect_type_inferences(&pp.ty, &cp.ty, params, out);
                }
                collect_type_inferences(&ft.return_type, &c_ft.return_type, params, out);
            }
        }
        _ => {}
    }
}
