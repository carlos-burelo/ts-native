use tower_lsp::lsp_types::DocumentSymbol;

use crate::document::{DocumentState, SymbolRecord};
use crate::util::converters::{range_on_line, to_lsp_symbol_kind};

#[allow(deprecated)]
pub fn build_document_symbols(state: &DocumentState) -> Vec<DocumentSymbol> {
    state
        .symbols
        .iter()
        .filter(|s| s.line != u32::MAX)
        .map(symbol_to_doc)
        .collect()
}

#[allow(deprecated)]
fn symbol_to_doc(sym: &SymbolRecord) -> DocumentSymbol {
    let name_end = sym.name.len() as u32;
    let full_range = range_on_line(sym.line, 0, name_end);
    let select_range = range_on_line(sym.line, 0, name_end);

    let detail = if sym.type_str.is_empty() {
        None
    } else {
        Some(sym.type_str.clone())
    };

    DocumentSymbol {
        name: sym.name.clone(),
        detail,
        kind: to_lsp_symbol_kind(sym.kind),
        tags: None,
        deprecated: None,
        range: full_range,
        selection_range: select_range,
        children: None,
    }
}
