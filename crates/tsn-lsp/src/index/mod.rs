pub mod builder;
pub mod query;
use crate::document::DocumentState;
use std::collections::{HashMap, HashSet};
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
    /// file URI → set of file URIs that import it (reverse dependency map)
    pub reverse_deps: HashMap<String, HashSet<String>>,
    /// module specifier → resolved file URI (e.g. "std:math" → "file:///...")
    pub module_cache: HashMap<String, String>,
}

impl ProjectIndex {
    pub fn new() -> Self {
        Self {
            module_exports: HashMap::new(),
            name_index: HashMap::new(),
            reverse_deps: HashMap::new(),
            module_cache: HashMap::new(),
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

        // Remove this file from all reverse_deps sets
        for dependents in self.reverse_deps.values_mut() {
            dependents.remove(uri);
        }
        // Remove its own reverse_deps entry
        self.reverse_deps.remove(uri);

        // Invalidate module_cache entries that resolved to this uri
        self.module_cache.retain(|_, v| v != uri);
    }

    pub fn exports_for(&self, uri: &str) -> &[ExportEntry] {
        self.module_exports.get(uri).map_or(&[], Vec::as_slice)
    }

    pub fn definitions_of(&self, name: &str) -> &[(String, ExportEntry)] {
        self.name_index.get(name).map_or(&[], Vec::as_slice)
    }

    pub fn dependents_of(&self, uri: &str) -> impl Iterator<Item = &str> {
        self.reverse_deps
            .get(uri)
            .into_iter()
            .flat_map(|s| s.iter().map(String::as_str))
    }
}

impl Default for ProjectIndex {
    fn default() -> Self {
        Self::new()
    }
}
