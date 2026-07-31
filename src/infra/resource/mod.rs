//! Resource loading — unified resource discovery and caching.
//!
//! Central resource layer that discovers:
//! - Project context files (AGENTS.md, CLAUDE.md)
//! - Skills
//! - Themes
//! - System prompt files (SYSTEM.md, APPEND_SYSTEM.md)
//!
//! All resources are loaded eagerly and can be refreshed with `reload()`.

pub mod loader;

#[allow(unused_imports)]
pub use loader::{AgentsFile, DefaultResourceLoader, ResourceDiagnostic, SkillInfo, ThemeInfo};
