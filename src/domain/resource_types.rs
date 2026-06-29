//! Resource metadata vocabulary — pure data types for loaded resources.
//!
//! `AgentsFile`, `SkillInfo`, `PromptTemplate`, `ThemeInfo`, and
//! `ResourceDiagnostic` are shared by `agent` (prompt assembly, session) and
//! `infra` (resource loader). The `DefaultResourceLoader` runtime (file I/O,
//! caching) stays in `infra::resource`; this module holds only the metadata
//! shapes.

use std::path::PathBuf;

use crate::domain::source_info::SourceInfo;

// ── AgentsFile ────────────────────────────────────────────────────────

/// A discovered project context file.
#[derive(Debug, Clone)]
pub struct AgentsFile {
    /// Absolute file path.
    pub path: PathBuf,
    /// File content.
    pub content: String,
}

/// Diagnostic collected during resource loading.
#[derive(Debug, Clone)]
pub struct ResourceDiagnostic {
    /// "error" or "warning"
    pub level: String,
    /// Human-readable message
    pub message: String,
    /// Optional path that caused the diagnostic
    pub path: Option<PathBuf>,
}

impl ResourceDiagnostic {
    pub fn error(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self {
            level: "error".into(),
            message: message.into(),
            path,
        }
    }

    pub fn warning(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self {
            level: "warning".into(),
            message: message.into(),
            path,
        }
    }
}

/// A loaded prompt template.
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// Template name (filename without .md extension).
    pub name: String,
    /// Template content (markdown body, after frontmatter).
    pub content: String,
    /// Optional description from frontmatter.
    pub description: Option<String>,
    /// Optional argument hint from frontmatter.
    pub argument_hint: Option<String>,
    /// Provenance info for display.
    pub source_info: SourceInfo,
}

/// Skill metadata returned by the resource loader.
#[derive(Debug, Clone)]
pub struct SkillInfo {
    pub name: String,
    pub description: Option<String>,
    pub source_info: SourceInfo,
}

/// Theme metadata returned by the resource loader.
#[derive(Debug, Clone)]
pub struct ThemeInfo {
    pub name: String,
    pub source_info: SourceInfo,
}
