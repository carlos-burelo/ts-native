#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TypeKind<T, N, C, F, O, E = ()> {
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
    Array(T),
    Union(C),
    Intersection(C),
    Tuple(C),
    Named(N, Option<N>),
    Generic(N, C, Option<N>),
    LiteralInt(i64),
    LiteralFloat(u64),
    LiteralStr(N),
    LiteralBool(bool),
    /// `` `prefix${T}suffix` `` in type position.
    /// Parts alternate literal strings and interpolation types, encoded as `C`.
    TemplateLiteral(C),
    Fn(F),
    Object(O),
    Nullable(T),
    Typeof(E),

    /// `keyof T` — produce un union de string literals con los nombres de propiedades de T
    KeyOf(T),

    /// `T[K]` — tipo de la propiedad K en T (indexed access)
    IndexedAccess {
        object: T,
        index: T,
    },

    /// `{ [K in Source]?: Value }` — mapped type
    /// key_var: nombre de la variable de iteración (ej: "P")
    /// source: tipo fuente (normalmente keyof T o union de string literals)
    /// value: tipo del valor (puede referenciar key_var via Named(key_var))
    Mapped {
        key_var: N,
        source: T,
        value: T,
        optional: bool,
        readonly: bool,
    },

    /// `Check extends Extends ? TrueType : FalseType` — conditional type
    Conditional {
        check: T,
        extends: T,
        true_type: T,
        false_type: T,
    },

    /// `infer R` — type variable binding in conditional type extends clause
    Infer(N),
}

impl<T, N, C, F, O, E> TypeKind<T, N, C, F, O, E> {
    pub fn is_primitive(&self) -> bool {
        match self {
            TypeKind::Int
            | TypeKind::Float
            | TypeKind::Str
            | TypeKind::Bool
            | TypeKind::Void
            | TypeKind::Null
            | TypeKind::Never
            | TypeKind::Dynamic
            | TypeKind::BigInt
            | TypeKind::Decimal
            | TypeKind::Char
            | TypeKind::Symbol
            | TypeKind::This => true,
            _ => false,
        }
    }
}
