mod chain_queries;
pub mod import;
mod symbol_queries;

use rustc_hash::FxHashMap;
use std::collections::{HashMap, HashSet};

use tsn_checker::{SymbolKind, Type};
use tsn_core::TokenKind;

pub use import::{import_path_at, named_import_module_at, named_imported_names_at, uri_to_path};

#[derive(Clone, Debug)]
pub struct LspDiag {
    pub message: String,
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub severity: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberKind {
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
pub struct MemberRecord {
    pub name: String,
    pub type_str: String,
    pub params_str: String,
    pub is_static: bool,
    pub is_optional: bool,
    pub kind: MemberKind,
    pub is_arrow: bool,
    pub line: u32,
    pub col: u32,
    pub init_value: String,
    pub ty: Type,
    pub members: Vec<MemberRecord>,
}

#[derive(Clone, Debug)]
pub struct SymbolRecord {
    pub name: String,
    pub kind: SymbolKind,
    pub type_str: String,
    pub params_str: String,
    pub line: u32,
    pub col: u32,
    pub has_explicit_type: bool,
    pub is_async: bool,
    pub is_arrow: bool,
    pub doc: Option<String>,
    pub members: Vec<MemberRecord>,
    pub type_params: Vec<String>,
    pub ty: Type,
    pub symbol_id: Option<tsn_checker::symbol::SymbolId>,
    pub full_range: tsn_core::SourceRange,
    pub is_from_stdlib: bool,
}

#[derive(Clone, Debug)]
pub struct TokenRecord {
    pub kind: TokenKind,
    pub line: u32,
    pub col: u32,
    pub length: u32,
    pub offset: u32,
    pub lexeme: String,
}

#[derive(Clone, Debug)]
pub struct ParamScope {
    pub body_start_line: u32,
    pub body_end_line: u32,
    pub params: Vec<(String, String)>,
}

#[derive(Debug)]
pub enum ChainResult<'a> {
    Symbol(&'a SymbolRecord),
    Member {
        member: &'a MemberRecord,
        parent_name: String,
    },
    DynamicMember {
        member: MemberRecord,
        parent_name: String,
    },
}

impl<'a> ChainResult<'a> {
    pub fn name(&self) -> &str {
        match self {
            ChainResult::Symbol(s) => &s.name,
            ChainResult::Member { member, .. } => &member.name,
            ChainResult::DynamicMember { member, .. } => &member.name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MethodHoverInfo {
    pub receiver: String,
    pub class_name: String,
    pub method_name: String,
    pub return_type: String,
    pub params_str: String,
    pub is_static: bool,
    pub parent_kind: SymbolKind,
    pub init_value: String,
}

#[derive(Debug, Clone)]
pub struct ImportPathContext {
    pub prefix: String,
    pub specifier: String,
    pub content_start_col: u32,
}

pub struct DocumentState {
    pub source: String,
    pub uri: String,
    pub diagnostics: Vec<LspDiag>,
    pub symbols: Vec<SymbolRecord>,
    pub tokens: Vec<TokenRecord>,
    pub symbol_map: HashMap<String, SymbolKind>,
    pub param_scopes: Vec<ParamScope>,
    pub type_param_names: HashSet<String>,
    pub flattened_members: HashMap<String, Vec<tsn_checker::types::ClassMemberInfo>>,
    pub extension_members: HashMap<String, Vec<MemberRecord>>,
    pub expr_types: FxHashMap<u32, tsn_checker::ExprInfo>,
}

/// Type alias kept for backwards compatibility across the codebase.
pub type DocumentAnalysis = DocumentState;
