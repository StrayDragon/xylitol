//! Construct-time tool container for the agent runtime.
//!
//! [`ToolSet`] is a build-time-final collection of tools. It provides only
//! unit operations (`empty`, `from_iter`, `plus`, `remove`, `merge`, `retain`);
//! once handed to a turn, the loop consumes the final set directly. There is no
//! runtime allow/exclude filter API.

use std::sync::Arc;

use crate::runtime_protocol::XyTool;

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

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn XyTool>> {
        self.tools.iter().find(|t| t.name() == name).cloned()
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
    pub fn merge(mut self, other: ToolSet) -> Self {
        self.tools.extend(other.tools);
        self
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
