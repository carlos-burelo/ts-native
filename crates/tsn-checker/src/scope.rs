use crate::symbol::SymbolId;
use std::collections::HashMap;
pub type ScopeId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeKind {
    Global,
    Module,
    Function,
    Block,
    Class,
    Namespace,
}

#[derive(Clone, Debug)]
pub struct Scope {
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,
    pub bindings: HashMap<String, SymbolId>,
    pub children: Vec<ScopeId>,
    pub ordered: Vec<SymbolId>,
}

impl Scope {
    pub fn new(kind: ScopeKind, parent: Option<ScopeId>) -> Self {
        Self {
            kind,
            parent,
            bindings: HashMap::new(),
            children: Vec::new(),
            ordered: Vec::new(),
        }
    }

    pub fn define(&mut self, name: impl Into<String>, id: SymbolId) {
        let name = name.into();
        self.bindings.insert(name, id);
        self.ordered.push(id);
    }

    pub fn lookup(&self, name: &str) -> Option<SymbolId> {
        self.bindings.get(name).copied()
    }

    pub fn resolve<'s>(&'s self, name: &str, arena: &'s ScopeArena) -> Option<SymbolId> {
        let mut current = self;
        loop {
            if let Some(&id) = current.bindings.get(name) {
                return Some(id);
            }
            match current.parent {
                Some(parent_id) => current = arena.get(parent_id),
                None => return None,
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct ScopeArena {
    scopes: Vec<Scope>,
}

impl ScopeArena {
    pub fn push(&mut self, scope: Scope) -> ScopeId {
        let id = self.scopes.len();
        self.scopes.push(scope);
        id
    }

    pub fn get(&self, id: ScopeId) -> &Scope {
        &self.scopes[id]
    }

    pub fn get_mut(&mut self, id: ScopeId) -> &mut Scope {
        &mut self.scopes[id]
    }

    pub fn child(&mut self, kind: ScopeKind, parent: ScopeId) -> ScopeId {
        let child_id = self.push(Scope::new(kind, Some(parent)));
        self.scopes[parent].children.push(child_id);
        child_id
    }

    pub fn global(&self) -> ScopeId {
        0
    }
}
