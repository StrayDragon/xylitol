//! Resource loading — unified resource discovery and caching.
//!
//! Central resource layer that discovers:
//! - Project context files (AGENTS.md, CLAUDE.md)
//! - Prompt templates
//! - Skills
//! - Themes
//! - System prompt files (SYSTEM.md, APPEND_SYSTEM.md)
//!
//! All resources are loaded eagerly and can be refreshed with `reload()`.

pub mod loader;

#[allow(unused_imports)]
pub use loader::{
    AgentsFile, DefaultResourceLoader, PromptTemplate, ResourceDiagnostic, SkillInfo, ThemeInfo,
    load_project_context_files,
};
