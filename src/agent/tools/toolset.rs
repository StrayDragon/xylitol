//! Construct-time tool container for the agent runtime.
//!
//! [`ToolSet`] is a build-time-final collection of tools. It provides only
//! unit operations (`empty`, `from_iter`, `plus`, `remove`, `merge`,
//! `overlay_by_name`, `retain`); once handed to a turn, the loop consumes the
//! final set directly. There is no runtime allow/exclude filter API.

use std::sync::Arc;

use crate::protocol::ports::XyTool;

/// A build-time-final set of tools.
#[derive(Clone, Default)]
pub struct ToolSet {
    tools: Vec<Arc<dyn XyTool>>,
}

impl ToolSet {
    /// Create an empty tool set.
    pub fn empty() -> Self {
        Self { tools: Vec::new() }
    }

    /// Build a tool set from an iterable of tools.
    #[allow(clippy::should_implement_trait)]
    pub fn from_iter<I: IntoIterator<Item = Arc<dyn XyTool>>>(tools: I) -> Self {
        let mut set = Self::empty();
        for t in tools {
            set.tools.push(t);
        }
        set
    }

    /// Return the list of tools.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn XyTool>> {
        self.tools.iter()
    }

    /// Get a tool by registry name, or by provider wire name (`:` → `_`).
    pub fn get(&self, name: &str) -> Option<Arc<dyn XyTool>> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .cloned()
            .or_else(|| {
                self.tools
                    .iter()
                    .find(|t| {
                        xylitol_ai_bridge::provider::tool_wire::to_wire_tool_name(t.name()) == name
                    })
                    .cloned()
            })
    }

    /// Add a single tool, returning the updated set.
    pub fn plus(mut self, tool: Arc<dyn XyTool>) -> Self {
        self.tools.push(tool);
        self
    }

    /// Remove the tool with the given name, returning the updated set.
    pub fn remove(mut self, name: &str) -> Self {
        self.tools.retain(|t| t.name() != name);
        self
    }

    /// Merge another tool set into this one, returning the updated set.
    ///
    /// Prefer [`Self::overlay_by_name`] / [`Self::rebuild_agent_tools`] for MCP
    /// settle/reload so repeated rebuilds do not duplicate names.
    pub fn merge(mut self, other: ToolSet) -> Self {
        self.tools.extend(other.tools);
        self
    }

    /// Overlay `other` by tool name: later entries replace earlier; result names
    /// are unique.
    pub fn overlay_by_name(mut self, other: ToolSet) -> Self {
        for tool in other.tools {
            let name = tool.name();
            if let Some(pos) = self.tools.iter().position(|t| t.name() == name) {
                self.tools[pos] = tool;
            } else {
                self.tools.push(tool);
            }
        }
        self
    }

    /// Rebuild the agent tool set: builtins first, then MCP/custom overlay by name.
    pub fn rebuild_agent_tools(
        builtins: impl IntoIterator<Item = Arc<dyn XyTool>>,
        mcp: impl IntoIterator<Item = Arc<dyn XyTool>>,
    ) -> Self {
        Self::from_iter(builtins).overlay_by_name(Self::from_iter(mcp))
    }

    /// Retain only tools matching the predicate, returning the updated set.
    pub fn retain<F>(mut self, mut pred: F) -> Self
    where
        F: FnMut(&Arc<dyn XyTool>) -> bool,
    {
        self.tools.retain(|t| pred(t));
        self
    }
}
