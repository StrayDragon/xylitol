//! Resource loading — unified resource discovery and caching.
//!
//! Aligns with pi's resource-loader.ts. Central resource layer that discovers:
//! - Project context files (AGENTS.md, CLAUDE.md)
//! - Prompt templates
//! - Skills
//! - Themes
//! - System prompt files (SYSTEM.md, APPEND_SYSTEM.md)
//!
//! All resources are loaded eagerly and can be refreshed with `reload()`.

pub mod loader;

pub use loader::DefaultResourceLoader;
