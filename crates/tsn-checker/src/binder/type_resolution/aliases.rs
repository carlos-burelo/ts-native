use crate::types::{Type, TypeContext};
use tsn_core::{well_known, TypeKind};

use super::contexts::AliasSubstitutionContext;
use super::resolve_type_node;

pub(super) fn try_stdlib_generic_alias(
    name: &str,
    args: &[Type],
    ctx: Option<&dyn TypeContext>,
) -> Option<Type> {
    use crate::binder::BindResult;
    use std::sync::OnceLock;

    static STD_TYPES: OnceLock<Option<BindResult>> = OnceLock::new();

    let bind = STD_TYPES
        .get_or_init(|| {
            // Bind std:types directly WITHOUT going through MODULE_BIND_CACHE
            // to avoid deadlocking with it (it may already be locked by a parent binding).
            let path = crate::module_resolver::stdlib_path_for("std:types")?;
            let source = std::fs::read_to_string(&path).ok()?;
            let abs = path.to_string_lossy().into_owned();
            let tokens = tsn_lexer::scan(&source, &abs);
            let program = tsn_parser::parse(tokens, &abs).ok()?;
            Some(crate::binder::Binder::bind(&program))
        })
        .as_ref()?;

    let (params, alias_node) = bind.get_alias_node(name)?;
    if params.is_empty() || params.len() != args.len() {
        return None;
    }
    let alias_ctx = AliasSubstitutionContext {
        inner: ctx,
        params,
        args: args.to_vec(),
    };
    Some(resolve_type_node(&alias_node, Some(&alias_ctx)))
}

pub(super) fn is_primitive_type(ty: &Type) -> bool {
    matches!(
        &ty.0,
        TypeKind::Int
            | TypeKind::Float
            | TypeKind::Str
            | TypeKind::Bool
            | TypeKind::Char
            | TypeKind::Null
            | TypeKind::Void
            | TypeKind::Never
    )
}

pub fn resolve_primitive(name: &str, ctx: Option<&dyn TypeContext>) -> Type {
    match name {
        well_known::INT => Type::Int,
        well_known::FLOAT => Type::Float,
        well_known::STR => Type::Str,
        well_known::BOOL => Type::Bool,
        well_known::CHAR => Type::Char,
        well_known::VOID => Type::Void,
        well_known::NULL => Type::Null,
        well_known::NEVER => Type::Never,
        well_known::DYNAMIC => Type::Dynamic,
        _ => Type::named_with_origin(
            name.to_owned(),
            ctx.and_then(|c| c.source_file()).map(|s| s.to_owned()),
        ),
    }
}
