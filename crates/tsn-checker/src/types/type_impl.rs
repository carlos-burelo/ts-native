use super::*;

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
