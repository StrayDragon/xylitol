//! Skill system — SKILL.md discovery.
//!
//! Skills are discovered by scanning directories for SKILL.md files.
//! Each SKILL.md has YAML frontmatter with name, description,
//! and optional disable-model-invocation flag.
//!
//! Note: MCP (Model Context Protocol) client integration has been moved to
//! [`crate::infra::mcp`] as it is an independent protocol.

// SKILL.md loader
pub(crate) mod loader;
