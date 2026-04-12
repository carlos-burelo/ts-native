use crate::constants::{SEVERITY_ERROR, SEVERITY_HINT, SEVERITY_WARNING};
use crate::document::DocumentState;
use crate::util::converters::range_on_line;
use tower_lsp::lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location,
    Position, Range, Url,
};

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

            let related_information = if d.related.is_empty() {
                None
            } else {
                let items: Vec<DiagnosticRelatedInformation> = d
                    .related
                    .iter()
                    .filter_map(|r| {
                        let url = Url::parse(&r.uri).ok()?;
                        let pos = Position {
                            line: r.line,
                            character: r.col,
                        };
                        Some(DiagnosticRelatedInformation {
                            location: Location::new(
                                url,
                                Range {
                                    start: pos,
                                    end: pos,
                                },
                            ),
                            message: r.message.clone(),
                        })
                    })
                    .collect();
                if items.is_empty() {
                    None
                } else {
                    Some(items)
                }
            };

            LspDiagnostic {
                range: range_on_line(d.line, d.col, d.end_col),
                severity: Some(severity),
                message: d.message.clone(),
                source: Some("tsn-lsp".into()),
                related_information,
                ..Default::default()
            }
        })
        .collect()
}
