use tsn_checker::SymbolKind;

use crate::document::{import::uri_to_path, DocumentState};

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

    // Build reverse dependency map and module cache from import paths
    let doc_path = uri_to_path(uri);
    let doc_dir = std::path::Path::new(&doc_path)
        .parent()
        .map(|p| p.to_path_buf());

    for specifier in &state.import_paths {
        let resolved_uri = resolve_specifier_to_uri(specifier, doc_dir.as_deref());
        if let Some(target_uri) = resolved_uri {
            index
                .reverse_deps
                .entry(target_uri.clone())
                .or_default()
                .insert(uri.to_owned());
            index
                .module_cache
                .entry(specifier.clone())
                .or_insert(target_uri);
        }
    }
}

fn resolve_specifier_to_uri(
    specifier: &str,
    doc_dir: Option<&std::path::Path>,
) -> Option<String> {
    if specifier.starts_with("std:") {
        let stdlib = tsn_checker::module_resolver::stdlib_dir()?;
        let module_name = specifier.strip_prefix("std:")?;
        let mod_path = stdlib.join("std").join(module_name).join("mod.tsn");
        if mod_path.is_file() {
            let canonical = std::fs::canonicalize(&mod_path).ok()?;
            return Some(path_to_uri(&canonical.to_string_lossy()));
        }
        return None;
    }

    if specifier.starts_with('.') {
        let dir = doc_dir?;
        let joined = dir.join(specifier);
        let with_ext = if joined.extension().is_none() {
            joined.with_extension("tsn")
        } else {
            joined
        };
        let canonical = std::fs::canonicalize(&with_ext).ok()?;
        return Some(path_to_uri(&canonical.to_string_lossy()));
    }

    None
}

fn path_to_uri(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    if normalized.starts_with('/') {
        format!("file://{}", normalized)
    } else {
        format!("file:///{}", normalized)
    }
}

fn is_indexable(kind: SymbolKind, line: u32) -> bool {
    if line == u32::MAX {
        return false;
    }
    !matches!(kind, SymbolKind::Parameter | SymbolKind::TypeParameter)
}
