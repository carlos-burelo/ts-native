use super::expr::Expr;
use super::operators::Modifiers;
use super::pattern::Param;
use super::stmt::{Stmt, VariableDecl};
use super::types::{Decorator, TypeNode, TypeParam};
use crate::source::SourceRange;

#[derive(Clone, Debug)]
pub enum Decl {
    Variable(VariableDecl),
    Function(FunctionDecl),
    Class(ClassDecl),
    Interface(InterfaceDecl),
    TypeAlias(TypeAliasDecl),
    Enum(EnumDecl),
    Namespace(NamespaceDecl),
    Import(ImportDecl),
    Export(ExportDecl),
    Extension(ExtensionDecl),
    Struct(StructDecl),
    SumType(SumTypeDecl),
}

impl Decl {
    pub fn range(&self) -> &SourceRange {
        match self {
            Decl::Variable(d) => &d.range,
            Decl::Function(d) => &d.range,
            Decl::Class(d) => &d.range,
            Decl::Interface(d) => &d.range,
            Decl::TypeAlias(d) => &d.range,
            Decl::Enum(d) => &d.range,
            Decl::Namespace(d) => &d.range,
            Decl::Import(d) => &d.range,
            Decl::Export(d) => d.range(),
            Decl::Extension(d) => &d.range,
            Decl::Struct(d) => &d.range,
            Decl::SumType(d) => &d.range,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SumTypeDecl {
    pub id: String,
    pub type_params: Vec<TypeParam>,
    pub variants: Vec<SumVariant>,
    pub doc: Option<String>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct SumVariant {
    pub name: String,
    pub fields: Vec<SumField>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct SumField {
    pub name: String,
    pub ty: TypeNode,
}

#[derive(Clone, Debug)]
pub struct FunctionDecl {
    pub id: String,
    /// Byte offset of the function name identifier token (for rename/go-to-def).
    pub id_offset: u32,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeNode>,
    pub body: Stmt,
    pub modifiers: Modifiers,
    pub decorators: Vec<Decorator>,
    pub doc: Option<String>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct ClassDecl {
    pub id: Option<String>,
    /// Byte offset of the class name identifier token (for rename/go-to-def). 0 if anonymous.
    pub id_offset: u32,
    pub type_params: Vec<TypeParam>,
    pub super_class: Option<Expr>,
    pub super_type_args: Vec<TypeNode>,
    pub implements: Vec<TypeNode>,
    pub body: Vec<ClassMember>,
    pub modifiers: Modifiers,
    pub decorators: Vec<Decorator>,
    pub doc: Option<String>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub enum ClassMember {
    Constructor {
        params: Vec<Param>,
        body: Stmt,
        range: SourceRange,
    },
    Destructor {
        body: Stmt,
        range: SourceRange,
    },
    Method {
        key: String,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        return_type: Option<TypeNode>,
        body: Option<Stmt>,
        modifiers: Modifiers,
        decorators: Vec<Decorator>,
        range: SourceRange,
    },
    Property {
        key: String,
        type_ann: Option<TypeNode>,
        init: Option<Expr>,
        modifiers: Modifiers,
        decorators: Vec<Decorator>,
        range: SourceRange,
    },
    Getter {
        key: String,
        return_type: Option<TypeNode>,
        body: Option<Stmt>,
        modifiers: Modifiers,
        range: SourceRange,
    },
    Setter {
        key: String,
        param: Param,
        body: Option<Stmt>,
        modifiers: Modifiers,
        range: SourceRange,
    },
    StaticBlock {
        body: Stmt,
        range: SourceRange,
    },
}

#[derive(Clone, Debug)]
pub struct InterfaceDecl {
    pub id: String,
    pub type_params: Vec<TypeParam>,
    pub extends: Vec<TypeNode>,
    pub body: Vec<InterfaceMember>,
    pub doc: Option<String>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub enum InterfaceMember {
    Property {
        key: String,
        type_ann: TypeNode,
        optional: bool,
        readonly: bool,
        range: SourceRange,
    },
    Method {
        key: String,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        return_type: Option<TypeNode>,
        optional: bool,
        range: SourceRange,
    },
    Index {
        param: Param,
        return_type: TypeNode,
        range: SourceRange,
    },
    Callable {
        params: Vec<Param>,
        return_type: TypeNode,
        range: SourceRange,
    },
}

#[derive(Clone, Debug)]
pub struct TypeAliasDecl {
    pub id: String,
    pub type_params: Vec<TypeParam>,
    pub alias: TypeNode,
    pub doc: Option<String>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct EnumDecl {
    pub id: String,
    pub members: Vec<EnumMember>,
    pub doc: Option<String>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct EnumMember {
    pub id: String,
    pub init: Option<Expr>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct NamespaceDecl {
    pub id: String,
    pub body: Vec<Decl>,
    pub doc: Option<String>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct ImportDecl {
    pub specifiers: Vec<ImportSpecifier>,
    pub source: String,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub enum ImportSpecifier {
    Named {
        local: String,
        imported: String,
        range: SourceRange,
    },
    Default {
        local: String,
        range: SourceRange,
    },
    Namespace {
        local: String,
        range: SourceRange,
    },
}

impl ImportSpecifier {
    pub fn range(&self) -> &SourceRange {
        match self {
            ImportSpecifier::Named { range, .. } => range,
            ImportSpecifier::Default { range, .. } => range,
            ImportSpecifier::Namespace { range, .. } => range,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ExportDecl {
    Named {
        specifiers: Vec<ExportSpecifier>,
        source: Option<String>,
        range: SourceRange,
    },
    Default {
        declaration: Box<ExportDefaultDecl>,
        range: SourceRange,
    },
    All {
        source: String,
        alias: Option<String>,
        range: SourceRange,
    },
    Decl {
        declaration: Box<Decl>,
        range: SourceRange,
    },
}

impl ExportDecl {
    pub fn range(&self) -> &SourceRange {
        match self {
            ExportDecl::Named { range, .. } => range,
            ExportDecl::Default { range, .. } => range,
            ExportDecl::All { range, .. } => range,
            ExportDecl::Decl { range, .. } => range,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExportSpecifier {
    pub local: String,
    pub exported: String,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub enum ExportDefaultDecl {
    Function(FunctionDecl),
    Class(ClassDecl),
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub struct ExtensionDecl {
    pub id: Option<String>,
    pub target: TypeNode,
    pub members: Vec<ExtensionMember>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub enum ExtensionMember {
    Method(FunctionDecl),
    Getter {
        key: String,
        return_type: Option<TypeNode>,
        body: Stmt,
        modifiers: Modifiers,
        range: SourceRange,
    },
    Setter {
        key: String,
        param: Param,
        body: Stmt,
        modifiers: Modifiers,
        range: SourceRange,
    },
}

#[derive(Clone, Debug)]
pub struct StructDecl {
    pub id: String,
    pub fields: Vec<StructField>,
    pub doc: Option<String>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct StructField {
    pub name: String,
    pub type_ann: TypeNode,
    pub default: Option<Expr>,
    pub range: SourceRange,
}
