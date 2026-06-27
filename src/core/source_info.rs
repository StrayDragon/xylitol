//! Resource source-info vocabulary + shared text utilities.
//!
//! Pure data types describing where a loaded resource (skill, prompt, theme,
//! command) came from, plus small string helpers shared across layers. Zero
//! crate-internal dependencies — both `agent` and `infra` reference these.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The scope of a resource: where it was loaded from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceScope {
    /// User-level resource (e.g., ~/.xylitol/skills/).
    User,
    /// Project-level resource (e.g., <cwd>/.xylitol/skills/).
    Project,
    /// Temporary or synthetic resource (not persisted).
    Temporary,
}

/// The origin of a resource: how it was installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceOrigin {
    /// Installed via a package manager (npm, git).
    Package,
    /// Directly placed by the user (top-level dir or file).
    TopLevel,
}

/// Provenance information for a loaded resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    /// Absolute path to the resource file.
    pub path: PathBuf,
    /// Human-readable source description (e.g., "local", "npm:foo", "git:...").
    pub source: String,
    /// Scope of the resource.
    pub scope: SourceScope,
    /// Origin of the resource.
    pub origin: SourceOrigin,
    /// Optional base directory for relative path resolution.
    pub base_dir: Option<PathBuf>,
}

/// Create a SourceInfo from a resolved path and metadata.
pub fn create_source_info(
    path: PathBuf,
    source: String,
    scope: SourceScope,
    origin: SourceOrigin,
    base_dir: Option<PathBuf>,
) -> SourceInfo {
    SourceInfo {
        path,
        source,
        scope,
        origin,
        base_dir,
    }
}

/// Create a synthetic SourceInfo (e.g., for built-in or generated resources).
pub fn create_synthetic_source_info(
    path: PathBuf,
    source: String,
    scope: Option<SourceScope>,
    origin: Option<SourceOrigin>,
    base_dir: Option<PathBuf>,
) -> SourceInfo {
    SourceInfo {
        path,
        source,
        scope: scope.unwrap_or(SourceScope::Temporary),
        origin: origin.unwrap_or(SourceOrigin::TopLevel),
        base_dir,
    }
}

/// Escape a string for safe inclusion in XML/HTML text content.
///
/// Pure function shared by prompt assembly (`agent`) and skill loading
/// (`infra`); hence lives in the shared vocabulary layer.
pub fn xml_escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            _ => result.push(c),
        }
    }
    result
}
