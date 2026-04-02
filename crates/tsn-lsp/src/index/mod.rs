pub mod builder;
pub mod query;
use crate::document::DocumentState;
use std::collections::HashMap;
use tsn_checker::SymbolKind;

#[derive(Debug, Clone)]
pub struct ExportEntry {
    pub name: String,
    pub kind: SymbolKind,
    pub uri: String,
    pub line: u32,
    pub col: u32,
    pub type_str: String,
    pub doc: Option<String>,
}

pub struct ProjectIndex {
    pub module_exports: HashMap<String, Vec<ExportEntry>>,

    pub name_index: HashMap<String, Vec<(String, ExportEntry)>>,
}

impl ProjectIndex {
    pub fn new() -> Self {
        Self {
            module_exports: HashMap::new(),
            name_index: HashMap::new(),
        }
    }

    pub fn update_file(&mut self, uri: &str, state: &DocumentState) {
        self.remove_file(uri);
        builder::index_file(self, uri, state);
    }

    pub fn remove_file(&mut self, uri: &str) {
        self.module_exports.remove(uri);
        for entries in self.name_index.values_mut() {
            entries.retain(|(u, _)| u != uri);
        }

        self.name_index.retain(|_, v| !v.is_empty());
    }

    pub fn exports_for(&self, uri: &str) -> &[ExportEntry] {
        self.module_exports.get(uri).map_or(&[], Vec::as_slice)
    }

    pub fn definitions_of(&self, name: &str) -> &[(String, ExportEntry)] {
        self.name_index.get(name).map_or(&[], Vec::as_slice)
    }
}

impl Default for ProjectIndex {
    fn default() -> Self {
        Self::new()
    }
}
