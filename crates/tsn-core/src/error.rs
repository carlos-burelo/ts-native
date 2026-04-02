use crate::source::SourceRange;
use std::fmt;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticKind {
    Error,
    Warning,
    Hint,
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub message: String,
    pub range: SourceRange,
    pub file: Arc<str>,
    pub code: Option<u32>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, range: SourceRange) -> Self {
        Diagnostic {
            kind: DiagnosticKind::Error,
            message: message.into(),
            range,
            file: Arc::from(""),
            code: None,
            metadata: std::collections::HashMap::new(),
        }
    }
    pub fn warning(message: impl Into<String>, range: SourceRange) -> Self {
        Diagnostic {
            kind: DiagnosticKind::Warning,
            message: message.into(),
            range,
            file: Arc::from(""),
            code: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn hint(message: impl Into<String>, range: SourceRange) -> Self {
        Diagnostic {
            kind: DiagnosticKind::Hint,
            message: message.into(),
            range,
            file: Arc::from(""),
            code: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.kind == DiagnosticKind::Error
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.kind {
            DiagnosticKind::Error => "error",
            DiagnosticKind::Warning => "warning",
            DiagnosticKind::Hint => "hint",
        };
        write!(
            f,
            "{} [{}:{}] {}",
            kind, self.file, self.range.start.line, self.message
        )
    }
}

#[derive(Default, Debug)]
pub struct DiagnosticBag {
    items: Vec<Diagnostic>,
}

impl DiagnosticBag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn error(&mut self, message: impl Into<String>, range: SourceRange) {
        self.items.push(Diagnostic::error(message, range));
    }

    pub fn warning(&mut self, message: impl Into<String>, range: SourceRange) {
        self.items.push(Diagnostic::warning(message, range));
    }

    pub fn hint(&mut self, message: impl Into<String>, range: SourceRange) {
        self.items.push(Diagnostic::hint(message, range));
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.kind == DiagnosticKind::Error)
    }

    pub fn all(&self) -> &[Diagnostic] {
        &self.items
    }

    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.items
            .iter()
            .filter(|d| d.kind == DiagnosticKind::Error)
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}
