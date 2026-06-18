//! Skill system — SKILL.md discovery aligning with pi's skills.ts.
//!
//! Skills are discovered by scanning directories for SKILL.md files.
//! Each SKILL.md has YAML frontmatter with name, description,
//! and optional disable-model-invocation flag.
//!
//! NOTE(c45): The old YAML-config-based SkillManager is kept for now but marked
//! for removal by c75. New code lives in `src/infra/skills/loader.rs`.
//!
//! MCP (Model Context Protocol) enables dynamic tool loading from external
//! servers via stdio or SSE transport.

#![allow(dead_code)]

// New SKILL.md loader (c45-align-skills-system)
mod loader;
// Old YAML-config loader (to be removed by c75)
mod manager;
pub(crate) use manager::*;

// MCP tools
mod mcp;
#[allow(unused_imports)]
pub(crate) use mcp::{McpClientManager, McpToolAdapter};
