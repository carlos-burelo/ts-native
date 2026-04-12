mod class_member_impl;
mod context;
mod display;
mod object_member_impl;
mod type_impl;

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

pub use context::TypeContext;
