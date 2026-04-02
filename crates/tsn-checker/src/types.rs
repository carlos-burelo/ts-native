use std::collections::HashMap;
use std::fmt;
use tsn_core::ast::operators::Visibility;
use tsn_core::TypeKind;

// Re-export from tsn-core so all checker modules use the shared source of truth.
pub(crate) use tsn_core::well_known;

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionParam {
    pub name: Option<String>,
    pub ty: Type,
    pub optional: bool,
    pub is_rest: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionType {
    pub params: Vec<FunctionParam>,
    pub return_type: Box<Type>,
    pub is_arrow: bool,
    pub type_params: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Type(
    pub TypeKind<Box<Type>, String, Vec<Type>, FunctionType, Vec<ObjectTypeMember>, ()>,
);

#[derive(Clone, Debug, PartialEq)]
pub enum ObjectTypeMember {
    Property {
        name: String,
        ty: Type,
        optional: bool,
        readonly: bool,
    },
    Method {
        name: String,
        params: Vec<FunctionParam>,
        return_type: Box<Type>,
        optional: bool,
        is_arrow: bool,
    },
    Index {
        param_name: String,
        key_ty: Box<Type>,
        value_ty: Box<Type>,
    },
    Callable {
        params: Vec<FunctionParam>,
        return_type: Box<Type>,
        is_arrow: bool,
    },
}

impl ObjectTypeMember {
    pub fn substitute(&self, mapping: &HashMap<String, Type>) -> Self {
        match self {
            ObjectTypeMember::Property {
                name,
                ty,
                optional,
                readonly,
            } => ObjectTypeMember::Property {
                name: name.clone(),
                ty: ty.substitute(mapping),
                optional: *optional,
                readonly: *readonly,
            },
            ObjectTypeMember::Method {
                name,
                params,
                return_type,
                optional,
                is_arrow,
            } => ObjectTypeMember::Method {
                name: name.clone(),
                params: params
                    .iter()
                    .map(|p| FunctionParam {
                        name: p.name.clone(),
                        ty: p.ty.substitute(mapping),
                        optional: p.optional,
                        is_rest: p.is_rest,
                    })
                    .collect(),
                return_type: Box::new(return_type.substitute(mapping)),
                optional: *optional,
                is_arrow: *is_arrow,
            },
            ObjectTypeMember::Index {
                param_name,
                key_ty,
                value_ty,
            } => ObjectTypeMember::Index {
                param_name: param_name.clone(),
                key_ty: Box::new(key_ty.substitute(mapping)),
                value_ty: Box::new(value_ty.substitute(mapping)),
            },
            ObjectTypeMember::Callable {
                params,
                return_type,
                is_arrow,
            } => ObjectTypeMember::Callable {
                params: params
                    .iter()
                    .map(|p| FunctionParam {
                        name: p.name.clone(),
                        ty: p.ty.substitute(mapping),
                        optional: p.optional,
                        is_rest: p.is_rest,
                    })
                    .collect(),
                return_type: Box::new(return_type.substitute(mapping)),
                is_arrow: *is_arrow,
            },
        }
    }
}

#[allow(non_upper_case_globals)]
impl Type {
    pub const Int: Type = Type(TypeKind::Int);
    pub const Float: Type = Type(TypeKind::Float);
    pub const Decimal: Type = Type(TypeKind::Decimal);
    pub const BigInt: Type = Type(TypeKind::BigInt);
    pub const Str: Type = Type(TypeKind::Str);
    pub const Char: Type = Type(TypeKind::Char);
    pub const Bool: Type = Type(TypeKind::Bool);
    pub const Symbol: Type = Type(TypeKind::Symbol);
    pub const Void: Type = Type(TypeKind::Void);
    pub const Null: Type = Type(TypeKind::Null);
    pub const Never: Type = Type(TypeKind::Never);
    pub const Dynamic: Type = Type(TypeKind::Dynamic);
    pub const This: Type = Type(TypeKind::This);

    pub fn fn_(f: FunctionType) -> Self {
        Type(TypeKind::Fn(f))
    }

    pub fn named(name: String) -> Self {
        Type(TypeKind::Named(name, None))
    }

    pub fn named_with_origin(name: String, origin: Option<String>) -> Self {
        Type(TypeKind::Named(name, origin))
    }

    pub fn array(inner: Type) -> Self {
        Type(TypeKind::Array(Box::new(inner)))
    }

    pub fn generic(name: String, args: Vec<Type>) -> Self {
        Type(TypeKind::Generic(name, args, None))
    }

    pub fn generic_with_origin(name: String, args: Vec<Type>, origin: Option<String>) -> Self {
        Type(TypeKind::Generic(name, args, origin))
    }

    pub fn object(members: Vec<ObjectTypeMember>) -> Self {
        Type(TypeKind::Object(members))
    }

    pub fn union(members: Vec<Type>) -> Self {
        Type(TypeKind::Union(members))
    }

    pub fn literal_int(v: i64) -> Self {
        Type(TypeKind::LiteralInt(v))
    }

    pub fn literal_float(v: u64) -> Self {
        Type(TypeKind::LiteralFloat(v))
    }

    pub fn literal_str(v: String) -> Self {
        Type(TypeKind::LiteralStr(v))
    }

    pub fn literal_bool(v: bool) -> Self {
        Type(TypeKind::LiteralBool(v))
    }

    pub fn is_dynamic(&self) -> bool {
        matches!(&self.0, TypeKind::Dynamic)
    }

    /// Returns the canonical stdlib lookup key for builtin primitive/array types.
    /// This is the single authoritative mapping from a `TypeKind` variant to its
    /// string name — use it everywhere instead of duplicating match arms.
    /// Returns `None` for Named / Generic / user-defined types.
    pub fn stdlib_key(&self) -> Option<&'static str> {
        use well_known::*;
        match &self.0 {
            TypeKind::Int | TypeKind::LiteralInt(_) => Some(INT),
            TypeKind::Float | TypeKind::LiteralFloat(_) => Some(FLOAT),
            TypeKind::Decimal => Some(DECIMAL),
            TypeKind::BigInt => Some(BIGINT),
            TypeKind::Str | TypeKind::LiteralStr(_) => Some(STR),
            TypeKind::Char => Some(CHAR),
            TypeKind::Bool | TypeKind::LiteralBool(_) => Some(BOOL),
            TypeKind::Symbol => Some(SYMBOL),
            TypeKind::Array(_) => Some(ARRAY),
            _ => None,
        }
    }

    /// Like `stdlib_key()` but also includes Named and Generic type names.
    /// Use this for member / extension-method descriptor lookups where user
    /// types are valid keys alongside builtin primitives.
    pub fn descriptor_key(&self) -> Option<&str> {
        if let Some(k) = self.stdlib_key() {
            return Some(k);
        }
        match &self.0 {
            TypeKind::Named(n, _) | TypeKind::Generic(n, _, _) => Some(n.as_str()),
            _ => None,
        }
    }

    pub fn is_int(&self) -> bool {
        match &self.0 {
            TypeKind::Int => true,
            _ => false,
        }
    }

    pub fn is_float(&self) -> bool {
        match &self.0 {
            TypeKind::Float => true,
            _ => false,
        }
    }

    pub fn is_str(&self) -> bool {
        match &self.0 {
            TypeKind::Str => true,
            _ => false,
        }
    }

    pub fn is_bool(&self) -> bool {
        match &self.0 {
            TypeKind::Bool => true,
            _ => false,
        }
    }

    pub fn is_void(&self) -> bool {
        match &self.0 {
            TypeKind::Void => true,
            _ => false,
        }
    }

    pub fn is_nullable(&self) -> bool {
        match &self.0 {
            TypeKind::Null => true,
            TypeKind::Union(members) => members.iter().any(|m| m.is_nullable()),
            _ => false,
        }
    }

    pub fn non_nullified(&self) -> Type {
        match &self.0 {
            TypeKind::Null => Type::Never,
            TypeKind::Union(members) => {
                let mut new_members: Vec<Type> = members
                    .iter()
                    .filter(|m| !m.is_nullable())
                    .cloned()
                    .collect();

                if new_members.is_empty() {
                    Type::Never
                } else if new_members.len() == 1 {
                    new_members.pop().unwrap()
                } else {
                    Type(TypeKind::Union(new_members))
                }
            }
            _ => self.clone(),
        }
    }

    pub fn minus_named(&self, name: &str) -> Type {
        match &self.0 {
            TypeKind::Named(n, _) if n == name => Type::Never,
            TypeKind::Union(members) => {
                let mut kept: Vec<Type> = members
                    .iter()
                    .filter(|m| !matches!(&m.0, TypeKind::Named(n, _) if n == name))
                    .cloned()
                    .collect();
                if kept.is_empty() {
                    Type::Never
                } else if kept.len() == 1 {
                    kept.pop().unwrap()
                } else {
                    Type(TypeKind::Union(kept))
                }
            }
            _ => self.clone(),
        }
    }

    pub fn with_origin_recursive(&self, origin: &str) -> Type {
        match &self.0 {
            TypeKind::Named(name, existing_origin) => {
                if existing_origin.is_none() {
                    Type::named_with_origin(name.clone(), Some(origin.to_owned()))
                } else {
                    self.clone()
                }
            }
            TypeKind::Generic(name, args, existing_origin) => {
                let new_args = args
                    .iter()
                    .map(|a| a.with_origin_recursive(origin))
                    .collect();
                if existing_origin.is_none() {
                    Type::generic_with_origin(name.clone(), new_args, Some(origin.to_owned()))
                } else {
                    Type::generic_with_origin(name.clone(), new_args, existing_origin.clone())
                }
            }
            TypeKind::Array(inner) => Type::array(inner.with_origin_recursive(origin)),
            TypeKind::Union(members) => Type::union(
                members
                    .iter()
                    .map(|m| m.with_origin_recursive(origin))
                    .collect(),
            ),
            TypeKind::TemplateLiteral(parts) => Type(TypeKind::TemplateLiteral(
                parts
                    .iter()
                    .map(|p| p.with_origin_recursive(origin))
                    .collect(),
            )),
            TypeKind::Object(members) => {
                let new_members = members
                    .iter()
                    .map(|m| match m {
                        ObjectTypeMember::Property {
                            name,
                            ty,
                            optional,
                            readonly,
                        } => ObjectTypeMember::Property {
                            name: name.clone(),
                            ty: ty.with_origin_recursive(origin),
                            optional: *optional,
                            readonly: *readonly,
                        },
                        ObjectTypeMember::Method {
                            name,
                            params,
                            return_type,
                            optional,
                            is_arrow,
                        } => ObjectTypeMember::Method {
                            name: name.clone(),
                            params: params
                                .iter()
                                .map(|p| FunctionParam {
                                    name: p.name.clone(),
                                    ty: p.ty.with_origin_recursive(origin),
                                    optional: p.optional,
                                    is_rest: p.is_rest,
                                })
                                .collect(),
                            return_type: Box::new(return_type.with_origin_recursive(origin)),
                            optional: *optional,
                            is_arrow: *is_arrow,
                        },
                        ObjectTypeMember::Index {
                            param_name,
                            key_ty,
                            value_ty,
                        } => ObjectTypeMember::Index {
                            param_name: param_name.clone(),
                            key_ty: Box::new(key_ty.with_origin_recursive(origin)),
                            value_ty: Box::new(value_ty.with_origin_recursive(origin)),
                        },
                        ObjectTypeMember::Callable {
                            params,
                            return_type,
                            is_arrow,
                        } => ObjectTypeMember::Callable {
                            params: params
                                .iter()
                                .map(|p| FunctionParam {
                                    name: p.name.clone(),
                                    ty: p.ty.with_origin_recursive(origin),
                                    optional: p.optional,
                                    is_rest: p.is_rest,
                                })
                                .collect(),
                            return_type: Box::new(return_type.with_origin_recursive(origin)),
                            is_arrow: *is_arrow,
                        },
                    })
                    .collect();
                Type::object(new_members)
            }
            TypeKind::Fn(ft) => {
                let params = ft
                    .params
                    .iter()
                    .map(|p| FunctionParam {
                        name: p.name.clone(),
                        ty: p.ty.with_origin_recursive(origin),
                        optional: p.optional,
                        is_rest: p.is_rest,
                    })
                    .collect();
                Type::fn_(FunctionType {
                    params,
                    return_type: Box::new(ft.return_type.with_origin_recursive(origin)),
                    is_arrow: ft.is_arrow,
                    type_params: ft.type_params.clone(),
                })
            }
            _ => self.clone(),
        }
    }

    pub fn substitute(&self, mapping: &HashMap<String, Type>) -> Type {
        match &self.0 {
            TypeKind::Named(name, origin) => {
                if let Some(ty) = mapping.get(name) {
                    ty.clone()
                } else {
                    Type::named_with_origin(name.clone(), origin.clone())
                }
            }
            TypeKind::Generic(name, args, origin) => {
                if let Some(ty) = mapping.get(name) {
                    ty.clone()
                } else {
                    let new_args = args.iter().map(|a| a.substitute(mapping)).collect();
                    Type::generic_with_origin(name.clone(), new_args, origin.clone())
                }
            }
            TypeKind::Array(inner) => Type::array(inner.substitute(mapping)),
            TypeKind::Union(members) => {
                Type::union(members.iter().map(|m| m.substitute(mapping)).collect())
            }
            TypeKind::TemplateLiteral(parts) => Type(TypeKind::TemplateLiteral(
                parts.iter().map(|p| p.substitute(mapping)).collect(),
            )),
            TypeKind::Object(members) => {
                Type::object(members.iter().map(|m| m.substitute(mapping)).collect())
            }
            TypeKind::Fn(ft) => {
                let params = ft
                    .params
                    .iter()
                    .map(|p| FunctionParam {
                        name: p.name.clone(),
                        ty: p.ty.substitute(mapping),
                        optional: p.optional,
                        is_rest: p.is_rest,
                    })
                    .collect();
                Type::fn_(FunctionType {
                    params,
                    return_type: Box::new(ft.return_type.substitute(mapping)),
                    is_arrow: ft.is_arrow,
                    type_params: ft.type_params.clone(),
                })
            }
            _ => self.clone(),
        }
    }

    pub fn make_nullable(inner: Type) -> Type {
        if inner.is_nullable() {
            return inner;
        }
        match inner.0 {
            TypeKind::Union(mut members) => {
                members.push(Type::Null);
                Type(TypeKind::Union(members))
            }
            _ => Type::union(vec![inner, Type::Null]),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            TypeKind::Int => write!(f, "int"),
            TypeKind::Float => write!(f, "float"),
            TypeKind::Str => write!(f, "str"),
            TypeKind::Bool => write!(f, "bool"),
            TypeKind::Void => write!(f, "void"),
            TypeKind::Null => write!(f, "null"),
            TypeKind::Never => write!(f, "never"),
            TypeKind::Dynamic => write!(f, "dynamic"),
            TypeKind::BigInt => write!(f, "bigint"),
            TypeKind::Decimal => write!(f, "decimal"),
            TypeKind::Char => write!(f, "char"),
            TypeKind::Symbol => write!(f, "symbol"),
            TypeKind::This => write!(f, "this"),
            TypeKind::Array(t) => write!(f, "{}[]", t),
            TypeKind::Union(members) => {
                let non_null: Vec<_> = members.iter().filter(|m| !m.is_nullable()).collect();
                if non_null.len() == 1 && non_null.len() < members.len() {
                    return write!(f, "{}?", non_null[0]);
                }
                for (i, m) in members.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", m)?;
                }
                Ok(())
            }
            TypeKind::Named(name, _origin) => write!(f, "{}", name),
            TypeKind::Generic(name, args, _origin) => {
                write!(f, "{}<", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ">")
            }
            TypeKind::LiteralInt(v) => write!(f, "{}", v),
            TypeKind::LiteralFloat(v) => write!(f, "{:?}", v),
            TypeKind::LiteralStr(v) => write!(f, "\"{}\"", v),
            TypeKind::LiteralBool(v) => write!(f, "{}", v),
            TypeKind::TemplateLiteral(parts) => {
                write!(f, "`")?;
                for (i, part) in parts.iter().enumerate() {
                    if i % 2 == 0 {
                        if let TypeKind::LiteralStr(s) = &part.0 {
                            write!(f, "{}", s)?;
                        }
                    } else {
                        write!(f, "${{{}}}", part)?;
                    }
                }
                write!(f, "`")
            }
            TypeKind::Fn(ft) => {
                write!(f, "(")?;
                for (i, p) in ft.params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if let Some(name) = &p.name {
                        let prefix = if p.is_rest { "..." } else { "" };
                        write!(f, "{}{}: ", prefix, name)?;
                    } else if p.is_rest {
                        write!(f, "...")?;
                    }
                    write!(f, "{}", p.ty)?;
                    if p.optional {
                        write!(f, "?")?;
                    }
                }
                write!(f, ") => {}", ft.return_type)
            }
            TypeKind::Object(members) => {
                write!(f, "{{ ")?;
                for (i, m) in members.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    match m {
                        ObjectTypeMember::Property {
                            name,
                            ty,
                            optional,
                            readonly,
                        } => {
                            if *readonly {
                                write!(f, "readonly ")?;
                            }
                            write!(f, "{}{}: {}", name, if *optional { "?" } else { "" }, ty)?;
                        }
                        ObjectTypeMember::Method {
                            name,
                            params,
                            return_type,
                            optional,
                            is_arrow: _,
                        } => {
                            write!(f, "{}{}(", name, if *optional { "?" } else { "" })?;
                            for (j, p) in params.iter().enumerate() {
                                if j > 0 {
                                    write!(f, ", ")?;
                                }
                                if let Some(pname) = &p.name {
                                    write!(f, "{}: ", pname)?;
                                }
                                write!(f, "{}", p.ty)?;
                                if p.optional {
                                    write!(f, "?")?;
                                }
                            }
                            write!(f, "): {}", return_type)?;
                        }
                        ObjectTypeMember::Index {
                            param_name,
                            key_ty,
                            value_ty,
                        } => {
                            write!(f, "[{}: {}]: {}", param_name, key_ty, value_ty)?;
                        }
                        ObjectTypeMember::Callable {
                            params,
                            return_type,
                            is_arrow: _,
                        } => {
                            write!(f, "(")?;
                            for (j, p) in params.iter().enumerate() {
                                if j > 0 {
                                    write!(f, ", ")?;
                                }
                                write!(f, "{}", p.ty)?;
                            }
                            write!(f, "): {}", return_type)?;
                        }
                    }
                }
                write!(f, " }}")
            }
            TypeKind::Intersection(members) => {
                for (i, m) in members.iter().enumerate() {
                    if i > 0 {
                        write!(f, " & ")?;
                    }
                    match &m.0 {
                        TypeKind::Union(_) => write!(f, "({})", m)?,
                        _ => write!(f, "{}", m)?,
                    }
                }
                Ok(())
            }
            TypeKind::Tuple(members) => {
                write!(f, "[")?;
                for (i, m) in members.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", m)?;
                }
                write!(f, "]")
            }
            TypeKind::Typeof(_) => write!(f, "typeof <expr>"),

            TypeKind::Nullable(t) => write!(f, "{}?", t),

            TypeKind::KeyOf(t) => write!(f, "keyof {}", t),
            TypeKind::IndexedAccess { object, index } => write!(f, "{}[{}]", object, index),
            TypeKind::Mapped {
                key_var,
                source,
                value,
                optional,
                readonly,
            } => {
                write!(
                    f,
                    "{{ {}[{} in {}]{}: {} }}",
                    if *readonly { "readonly " } else { "" },
                    key_var,
                    source,
                    if *optional { "?" } else { "" },
                    value
                )
            }
            TypeKind::Conditional {
                check,
                extends,
                true_type,
                false_type,
            } => {
                write!(
                    f,
                    "{} extends {} ? {} : {}",
                    check, extends, true_type, false_type
                )
            }
            TypeKind::Infer(name) => write!(f, "infer {}", name),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClassMemberKind {
    Constructor,
    Method,
    Property,
    Getter,
    Setter,
    Class,
    Interface,
    Namespace,
    Enum,
    Struct,
}

#[derive(Clone, Debug)]
pub struct ClassMemberInfo {
    pub name: String,
    pub kind: ClassMemberKind,
    pub is_async: bool,
    pub is_static: bool,
    pub is_optional: bool,
    pub line: u32,
    pub col: u32,
    pub ty: Type,
    pub members: Vec<ClassMemberInfo>,
    pub visibility: Option<Visibility>,
    pub is_abstract: bool,
    pub is_readonly: bool,
    pub is_override: bool,
}

impl ClassMemberInfo {
    /// Derive params display string from `ty`. Returns empty string for non-function members.
    pub fn params_str(&self) -> String {
        match &self.ty.0 {
            TypeKind::Fn(ft) => format_fn_params(&ft.params),
            _ => String::new(),
        }
    }

    /// Derive return type display string from `ty`.
    /// For methods returns the return type; for properties/getters returns the type itself.
    pub fn return_type_str(&self) -> String {
        match &self.ty.0 {
            TypeKind::Fn(ft) => ft.return_type.to_string(),
            _ => self.ty.to_string(),
        }
    }
}

fn format_fn_params(params: &[FunctionParam]) -> String {
    params
        .iter()
        .map(|p| {
            let rest = if p.is_rest { "..." } else { "" };
            let opt = if p.optional { "?" } else { "" };
            match &p.name {
                Some(n) => format!("{rest}{n}{opt}: {}", p.ty),
                None => format!("{rest}{}", p.ty),
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub trait TypeContext {
    fn get_interface_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<ClassMemberInfo>>;
    fn get_class_members(&self, name: &str, origin: Option<&str>) -> Option<Vec<ClassMemberInfo>>;
    fn get_namespace_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<ClassMemberInfo>>;
    fn resolve_symbol(&self, name: &str) -> Option<Type>;
    fn source_file(&self) -> Option<&str>;
    /// Returns (type_params, alias_body) for a generic type alias, enabling lazy substitution.
    fn get_alias_node(&self, _name: &str) -> Option<(Vec<String>, tsn_core::ast::TypeNode)> {
        None
    }
}
