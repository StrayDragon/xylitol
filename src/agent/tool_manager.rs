//! ToolManager — tool registry, selection, and filtering.
//!
//! Extracted from [`AgentSession`](super::session::AgentSession) to isolate
//! tool-related responsibilities into a focused component.

use crate::agent::tools::ToolRegistry;

/// Manages tool registry and active tool selection.
///
/// Owned by [`AgentSession`](super::session::AgentSession) as a composed field.
pub struct ToolManager {
    /// Registry of all available tools.
    registry: ToolRegistry,
    /// Names of currently active tools (empty = all allowed).
    active_tools: Vec<String>,
}

impl std::fmt::Debug for ToolManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolManager")
            .field("active_tools", &self.active_tools)
            .field("registry", &"<ToolRegistry>")
            .finish()
    }
}

impl ToolManager {
    /// Create a new ToolManager with the given registry.
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry,
            active_tools: Vec::new(),
        }
    }

    /// Get the underlying ToolRegistry (read-only).
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    /// Get the names of currently active tools.
    pub fn active_tools(&self) -> &[String] {
        &self.active_tools
    }

    /// Set which tools are active by name.
    pub fn set_active_tools(&mut self, tool_names: Vec<String>) {
        self.active_tools = tool_names;
    }
}
