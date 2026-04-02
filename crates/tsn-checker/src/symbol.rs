use crate::types::Type;
pub type SymbolId = usize;
use tsn_core::ast::TypeNode;
use tsn_core::SourceRange;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Var,
    Let,
    Const,
    Function,
    Class,
    Interface,
    TypeAlias,
    Enum,
    Parameter,
    Property,
    Method,
    TypeParameter,
    Namespace,
    Struct,
    Extension,
}

impl SymbolKind {
    pub fn label(self) -> &'static str {
        match self {
            SymbolKind::Var => "var  ",
            SymbolKind::Let => "let  ",
            SymbolKind::Const => "const",
            SymbolKind::Function => "fn   ",
            SymbolKind::Class => "class  ",
            SymbolKind::Interface => "interface",
            SymbolKind::TypeAlias => "type ",
            SymbolKind::Enum => "enum ",
            SymbolKind::Parameter => "param",
            SymbolKind::Property => "prop ",
            SymbolKind::Method => "method ",
            SymbolKind::TypeParameter => "type_param",
            SymbolKind::Namespace => "namespace   ",
            SymbolKind::Struct => "struct",
            SymbolKind::Extension => "extension  ",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub kind: SymbolKind,
    pub name: String,
    pub ty: Option<Type>,
    pub line: u32,
    pub col: u32,
    pub has_explicit_type: bool,
    pub is_async: bool,
    pub is_generator: bool,
    pub doc: Option<String>,
    pub type_params: Vec<String>,
    pub type_param_constraints: Vec<Option<Type>>,
    pub offset: u32,
    pub full_range: tsn_core::SourceRange,
    pub origin_module: Option<String>,
    pub original_name: Option<String>,
    /// For generic type aliases: the unresolved body TypeNode, used for lazy substitution.
    pub alias_node: Option<Box<TypeNode>>,
}

impl Symbol {
    pub fn new(kind: SymbolKind, name: impl Into<String>, line: u32) -> Self {
        Self {
            kind,
            name: name.into(),
            ty: None,
            line,
            col: 0,
            has_explicit_type: false,
            is_async: false,
            is_generator: false,
            doc: None,
            type_params: Vec::new(),
            type_param_constraints: Vec::new(),
            offset: 0,
            full_range: SourceRange::default(),
            origin_module: None,
            original_name: None,
            alias_node: None,
        }
    }

    pub fn with_type(mut self, ty: Type) -> Self {
        self.ty = Some(ty);
        self
    }
}

#[derive(Clone, Default)]
pub struct SymbolArena {
    symbols: Vec<Symbol>,
}

impl SymbolArena {
    pub fn push(&mut self, symbol: Symbol) -> SymbolId {
        let id = self.symbols.len();
        self.symbols.push(symbol);
        id
    }

    pub fn get(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id]
    }

    pub fn get_mut(&mut self, id: SymbolId) -> &mut Symbol {
        &mut self.symbols[id]
    }

    pub fn all(&self) -> &[Symbol] {
        &self.symbols
    }

    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    pub fn find_id_by_name_and_line(&self, name: &str, line: u32) -> Option<SymbolId> {
        self.symbols
            .iter()
            .position(|s| s.name == name && s.line == line)
    }
}
