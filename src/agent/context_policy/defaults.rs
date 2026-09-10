//! Unexposed ContextPolicy defaults — **only** edit this file to change defaults.
//!
//! Product YAML / env MUST NOT shadow these values (c1890; same code-first rule as c1880).

use super::ToolsMode;

pub const TOOLS_MODE_DEFAULT: ToolsMode = ToolsMode::Full;

/// Full mode may rewrite tools between turns.
pub const ALLOW_MIDTURN_TOOLS_REWRITE_DEFAULT: bool = true;
