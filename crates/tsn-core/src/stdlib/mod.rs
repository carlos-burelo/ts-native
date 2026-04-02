#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeTag {
    Int,
    Float,
    Str,
    Bool,
    Void,
    Null,
    Never,
    Dynamic,
    BigInt,
    Decimal,
    Char,
    Symbol,
    This,
    Named(&'static str),
    Array(&'static TypeTag),
    Union(&'static [TypeTag]),
    Intersection(&'static [TypeTag]),
    Generic(&'static str, &'static [TypeTag]),
    Fn(&'static [ParamSig], &'static TypeTag),
    Object(&'static [PropSig]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Class,
    Namespace,
    Enum,
    Interface,
    Function,
    TypeAlias,
    Const,
    Let,
    Var,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParamSig {
    pub name: &'static str,
    pub ty: TypeTag,
    pub is_optional: bool,
    pub is_rest: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropSig {
    pub name: &'static str,
    pub ty: TypeTag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexSig {
    pub key_name: &'static str,
    pub key_ty: TypeTag,
    pub value_ty: TypeTag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MethodSig {
    pub name: &'static str,
    pub params: &'static [ParamSig],
    pub return_ty: TypeTag,
    pub is_static: bool,
    pub throws: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StdlibExport {
    pub name: &'static str,
    pub kind: ExportKind,
    pub superclass: Option<&'static str>,
    pub members: &'static [MethodSig],
    pub props: &'static [PropSig],
    pub fn_return: TypeTag,
    pub fn_params: &'static [ParamSig],
    pub type_params: &'static [&'static str],
    pub index_sig: Option<IndexSig>,
}

pub struct StdlibModule {
    pub name: &'static str,
    pub exports: &'static [StdlibExport],
}

pub const fn klass_(
    name: &'static str,
    members: &'static [MethodSig],
    props: &'static [PropSig],
) -> StdlibExport {
    StdlibExport {
        name,
        kind: ExportKind::Class,
        superclass: None,
        members,
        props,
        fn_return: TypeTag::Dynamic,
        fn_params: &[],
        type_params: &[],
        index_sig: None,
    }
}

pub const fn error_klass_(
    name: &'static str,
    members: &'static [MethodSig],
    props: &'static [PropSig],
) -> StdlibExport {
    StdlibExport {
        name,
        kind: ExportKind::Class,
        superclass: Some("Error"),
        members,
        props,
        fn_return: TypeTag::Dynamic,
        fn_params: &[],
        type_params: &[],
        index_sig: None,
    }
}

pub const fn namespace_(
    name: &'static str,
    members: &'static [MethodSig],
    props: &'static [PropSig],
) -> StdlibExport {
    StdlibExport {
        name,
        kind: ExportKind::Namespace,
        superclass: None,
        members,
        props,
        fn_return: TypeTag::Dynamic,
        fn_params: &[],
        type_params: &[],
        index_sig: None,
    }
}

pub const fn interface_(
    name: &'static str,
    members: &'static [MethodSig],
    props: &'static [PropSig],
) -> StdlibExport {
    StdlibExport {
        name,
        kind: ExportKind::Interface,
        superclass: None,
        members,
        props,
        fn_return: TypeTag::Dynamic,
        fn_params: &[],
        type_params: &[],
        index_sig: None,
    }
}

pub const fn generic_interface_(
    name: &'static str,
    type_params: &'static [&'static str],
    members: &'static [MethodSig],
    props: &'static [PropSig],
    index_sig: Option<IndexSig>,
) -> StdlibExport {
    StdlibExport {
        name,
        kind: ExportKind::Interface,
        superclass: None,
        members,
        props,
        fn_return: TypeTag::Dynamic,
        fn_params: &[],
        type_params,
        index_sig,
    }
}

pub const fn enum_(
    name: &'static str,
    members: &'static [MethodSig],
    props: &'static [PropSig],
) -> StdlibExport {
    StdlibExport {
        name,
        kind: ExportKind::Enum,
        superclass: None,
        members,
        props,
        fn_return: TypeTag::Dynamic,
        fn_params: &[],
        type_params: &[],
        index_sig: None,
    }
}

pub const fn function_(
    name: &'static str,
    params: &'static [ParamSig],
    return_ty: TypeTag,
) -> StdlibExport {
    StdlibExport {
        name,
        kind: ExportKind::Function,
        superclass: None,
        members: &[],
        props: &[],
        fn_return: return_ty,
        fn_params: params,
        type_params: &[],
        index_sig: None,
    }
}

pub const fn builtin_function_(
    name: &'static str,
    params: &'static [ParamSig],
    return_ty: TypeTag,
) -> StdlibExport {
    function_(name, params, return_ty)
}

pub const fn const_(name: &'static str, ty: TypeTag) -> StdlibExport {
    StdlibExport {
        name,
        kind: ExportKind::Const,
        superclass: None,
        members: &[],
        props: &[],
        fn_return: ty,
        fn_params: &[],
        type_params: &[],
        index_sig: None,
    }
}

pub const fn let_(name: &'static str, ty: TypeTag) -> StdlibExport {
    StdlibExport {
        name,
        kind: ExportKind::Let,
        superclass: None,
        members: &[],
        props: &[],
        fn_return: ty,
        fn_params: &[],
        type_params: &[],
        index_sig: None,
    }
}

pub const fn var_(name: &'static str, ty: TypeTag) -> StdlibExport {
    StdlibExport {
        name,
        kind: ExportKind::Var,
        superclass: None,
        members: &[],
        props: &[],
        fn_return: ty,
        fn_params: &[],
        type_params: &[],
        index_sig: None,
    }
}

pub const fn type_alias_(name: &'static str) -> StdlibExport {
    StdlibExport {
        name,
        kind: ExportKind::TypeAlias,
        superclass: None,
        members: &[],
        props: &[],
        fn_return: TypeTag::Dynamic,
        fn_params: &[],
        type_params: &[],
        index_sig: None,
    }
}

pub const fn property_(name: &'static str, ty: TypeTag) -> PropSig {
    PropSig { name, ty }
}

pub const fn instance_method_(name: &'static str, return_ty: TypeTag) -> MethodSig {
    MethodSig {
        name,
        params: &[],
        return_ty,
        is_static: false,
        throws: &[],
    }
}

pub const fn static_method_(name: &'static str, return_ty: TypeTag) -> MethodSig {
    MethodSig {
        name,
        params: &[],
        return_ty,
        is_static: true,
        throws: &[],
    }
}

pub const fn instance_method_with_descriptor_(
    name: &'static str,
    return_ty: TypeTag,
    params: &'static [ParamSig],
) -> MethodSig {
    MethodSig {
        name,
        params,
        return_ty,
        is_static: false,
        throws: &[],
    }
}

pub const fn static_method_with_descriptor_(
    name: &'static str,
    return_ty: TypeTag,
    params: &'static [ParamSig],
) -> MethodSig {
    MethodSig {
        name,
        params,
        return_ty,
        is_static: true,
        throws: &[],
    }
}

pub const fn instance_method_with_throws_(
    name: &'static str,
    return_ty: TypeTag,
    params: &'static [ParamSig],
    throws: &'static [&'static str],
) -> MethodSig {
    MethodSig {
        name,
        params,
        return_ty,
        is_static: false,
        throws,
    }
}

pub const fn static_method_with_throws_(
    name: &'static str,
    return_ty: TypeTag,
    params: &'static [ParamSig],
    throws: &'static [&'static str],
) -> MethodSig {
    MethodSig {
        name,
        params,
        return_ty,
        is_static: true,
        throws,
    }
}

pub const fn param(name: &'static str, ty: TypeTag) -> ParamSig {
    ParamSig {
        name,
        ty,
        is_rest: false,
        is_optional: false,
    }
}

pub const fn optional_param(name: &'static str, ty: TypeTag) -> ParamSig {
    ParamSig {
        name,
        ty,
        is_rest: false,
        is_optional: true,
    }
}

pub const fn rest_param(name: &'static str, ty: TypeTag) -> ParamSig {
    ParamSig {
        name,
        ty,
        is_rest: true,
        is_optional: false,
    }
}
