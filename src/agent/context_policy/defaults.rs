//! Unexposed ContextPolicy defaults — **only** edit this file to change defaults.
//!
//! Product YAML / env MUST NOT shadow these values (c1890; same code-first rule as c1880).

use super::{StatusBarMode, ToolsMode};

pub const TOOLS_MODE_DEFAULT: ToolsMode = ToolsMode::Full;

pub const STATUS_BAR_MODE_DEFAULT: StatusBarMode = StatusBarMode::Off;

/// Full mode may rewrite tools between turns; Search must not.
pub const ALLOW_MIDTURN_TOOLS_REWRITE_DEFAULT: bool = true;
