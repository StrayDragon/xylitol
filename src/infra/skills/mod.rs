//! Skill system — SKILL.md discovery.
//!
//! Skills are discovered by scanning directories for SKILL.md files.
//! Each SKILL.md has YAML frontmatter with name, description,
//! and optional disable-model-invocation flag.
//!
//! MCP (Model Context Protocol) enables dynamic tool loading from external
//! servers via stdio or SSE transport.

#![allow(dead_code)]

// SKILL.md loader
pub(crate) mod loader;

// MCP tools
mod mcp;
