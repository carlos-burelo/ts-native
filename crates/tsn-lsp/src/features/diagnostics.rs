use crate::constants::{SEVERITY_ERROR, SEVERITY_WARNING, SEVERITY_HINT};
use crate::document::DocumentState;
use crate::util::converters::range_on_line;
use tower_lsp::lsp_types::{Diagnostic as LspDiagnostic, DiagnosticSeverity};

pub fn convert_diagnostics(state: &DocumentState) -> Vec<LspDiagnostic> {
    state
        .diagnostics
        .iter()
        .map(|d| {
            let severity = match d.severity {
                s if s == SEVERITY_ERROR => DiagnosticSeverity::ERROR,
                s if s == SEVERITY_WARNING => DiagnosticSeverity::WARNING,
                s if s == SEVERITY_HINT => DiagnosticSeverity::HINT,
                _ => DiagnosticSeverity::INFORMATION,
            };
            LspDiagnostic {
                range: range_on_line(d.line, d.col, d.end_col),
                severity: Some(severity),
                message: d.message.clone(),
                source: Some("tsn-lsp".into()),
                ..Default::default()
            }
        })
        .collect()
}
