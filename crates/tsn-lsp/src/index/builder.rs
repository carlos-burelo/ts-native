use tsn_checker::SymbolKind;

use crate::document::DocumentState;

use super::{ExportEntry, ProjectIndex};

pub fn index_file(index: &mut ProjectIndex, uri: &str, state: &DocumentState) {
    let exports: Vec<ExportEntry> = state
        .symbols
        .iter()
        .filter(|s| is_indexable(s.kind, s.line))
        .map(|s| ExportEntry {
            name: s.name.clone(),
            kind: s.kind,
            uri: uri.to_owned(),
            line: s.line,
            col: s.col,
            type_str: s.type_str.clone(),
            doc: s.doc.clone(),
        })
        .collect();

    for export in &exports {
        index
            .name_index
            .entry(export.name.clone())
            .or_default()
            .push((uri.to_owned(), export.clone()));
    }

    index.module_exports.insert(uri.to_owned(), exports);
}

fn is_indexable(kind: SymbolKind, line: u32) -> bool {
    if line == u32::MAX {
        return false;
    }
    !matches!(kind, SymbolKind::Parameter | SymbolKind::TypeParameter)
}
