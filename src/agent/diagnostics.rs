//! Diagnostics — non-fatal issue collection during session startup.
//!
//! Aligns with pi's diagnostics.ts. Collects info/warning/error level
//! diagnostics for API key checks, model validation, CWD issues, etc.

#![allow(dead_code)]
#[allow(dead_code)]
/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DiagLevel {
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for DiagLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagLevel::Info => write!(f, "info"),
            DiagLevel::Warning => write!(f, "warning"),
            DiagLevel::Error => write!(f, "error"),
        }
    }
}

/// A single diagnostic message.
#[derive(Debug, Clone)]
pub(crate) struct Diagnostic {
    pub(crate) level: DiagLevel,
    pub(crate) message: String,
    /// Optional source hint (e.g., "provider:openai", "session:load").
    pub(crate) source: Option<String>,
}

impl Diagnostic {
    pub(crate) fn info(message: impl Into<String>) -> Self {
        Self {
            level: DiagLevel::Info,
            message: message.into(),
            source: None,
        }
    }

    pub(crate) fn warning(message: impl Into<String>) -> Self {
        Self {
            level: DiagLevel::Warning,
            message: message.into(),
            source: None,
        }
    }

    pub(crate) fn error(message: impl Into<String>) -> Self {
        Self {
            level: DiagLevel::Error,
            message: message.into(),
            source: None,
        }
    }

    pub(crate) fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Format for display.
    pub(crate) fn display(&self) -> String {
        let source_str = self
            .source
            .as_ref()
            .map(|s| format!(" [{s}]"))
            .unwrap_or_default();
        format!(
            "{level}{source}: {msg}",
            level = self.level,
            source = source_str,
            msg = self.message
        )
    }
}

/// Collection of diagnostics gathered during session creation.
#[derive(Debug, Clone, Default)]
pub(crate) struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub(crate) fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Add a diagnostic.
    pub(crate) fn add(&mut self, diag: Diagnostic) {
        self.items.push(diag);
    }

    /// Convenience: add info.
    pub(crate) fn add_info(&mut self, msg: impl Into<String>) {
        self.add(Diagnostic::info(msg));
    }

    /// Convenience: add warning.
    pub(crate) fn add_warning(&mut self, msg: impl Into<String>) {
        self.add(Diagnostic::warning(msg));
    }

    /// Convenience: add error.
    pub(crate) fn add_error(&mut self, msg: impl Into<String>) {
        self.add(Diagnostic::error(msg));
    }

    /// Get all diagnostics.
    pub(crate) fn all(&self) -> &[Diagnostic] {
        &self.items
    }

    /// Get diagnostics of a specific level.
    pub(crate) fn of_level(&self, level: DiagLevel) -> Vec<&Diagnostic> {
        self.items.iter().filter(|d| d.level == level).collect()
    }

    /// Check if there are any errors.
    pub(crate) fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.level == DiagLevel::Error)
    }

    /// Check if there are any warnings.
    pub(crate) fn has_warnings(&self) -> bool {
        self.items.iter().any(|d| d.level == DiagLevel::Warning)
    }

    /// Number of diagnostics.
    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the collection is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Format all diagnostics for display.
    pub(crate) fn display_all(&self) -> String {
        if self.items.is_empty() {
            return String::from("no issues found");
        }
        self.items
            .iter()
            .map(|d| d.display())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostics_empty() {
        let d = Diagnostics::new();
        assert!(d.is_empty());
        assert_eq!(d.len(), 0);
    }

    #[test]
    fn test_diagnostics_add_and_filter() {
        let mut d = Diagnostics::new();
        d.add_warning("no API key");
        d.add_error("CWD missing");
        d.add_info("session loaded");

        assert_eq!(d.len(), 3);
        assert!(d.has_warnings());
        assert!(d.has_errors());
        assert_eq!(d.of_level(DiagLevel::Warning).len(), 1);
        assert_eq!(d.of_level(DiagLevel::Error).len(), 1);
        assert_eq!(d.of_level(DiagLevel::Info).len(), 1);
    }

    #[test]
    fn test_diagnostic_display() {
        let diag = Diagnostic::warning("no API key configured").with_source("provider:openai");
        let text = diag.display();
        assert!(text.contains("warning"));
        assert!(text.contains("no API key"));
        assert!(text.contains("provider:openai"));
    }

    #[test]
    fn test_diagnostics_display_all() {
        let mut d = Diagnostics::new();
        d.add_warning("warning 1");
        d.add_info("info 1");
        let text = d.display_all();
        assert!(text.contains("warning"));
        assert!(text.contains("info"));
    }

    #[test]
    fn test_diagnostics_display_all_empty() {
        let d = Diagnostics::new();
        assert_eq!(d.display_all(), "no issues found");
    }
}
