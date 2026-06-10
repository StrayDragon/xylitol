pub mod accumulator;
pub mod bash;
pub mod edit;
pub mod find;
pub mod grep;
pub mod ls;
pub mod mutation;
pub mod operations;
pub mod patch;
pub mod path_utils;
pub mod process;
pub mod read;
pub mod truncate;
pub mod write;

use std::sync::Arc;

use crate::agent::traits::XyTool;

// ── ToolRegistry ───────────────────────────────────────────────────

/// Registry for managing available tools.
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Vec<Arc<dyn XyTool>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn register(&mut self, tool: Arc<dyn XyTool>) {
        self.tools.push(tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn XyTool>> {
        self.tools.iter().find(|t| t.name() == name).cloned()
    }

    pub fn list(&self) -> &[Arc<dyn XyTool>] {
        &self.tools
    }

    /// Return tools matching the given names. Returns all if `allowed` is `None` or empty.
    pub fn filtered(&self, allowed: Option<&[String]>) -> Vec<Arc<dyn XyTool>> {
        match allowed {
            Some(names) if !names.is_empty() => self
                .tools
                .iter()
                .filter(|t| names.contains(&t.name().to_string()))
                .cloned()
                .collect(),
            _ => self.tools.clone(),
        }
    }

    /// Wrap every registered tool with a hook-checking wrapper.
    /// TODO: implement in hook-system phase — for now it's a no-op passthrough.
    pub fn wrap_with_hooks<F>(&mut self, _hook_cb: F)
    where
        F: Fn(&str, &serde_json::Value) -> bool + Send + Sync + 'static,
    {
        // Placeholder: will implement proper hook tool wrapper in Phase 5.
        // For now, tools are passed through as-is.
    }

    /// Create a registry with all built-in tools registered.
    pub fn builtins() -> Self {
        let mq = Arc::new(mutation::FileMutationQueue::new());
        let mut reg = Self::new();
        reg.register(Arc::new(read::ReadTool));
        reg.register(Arc::new(write::WriteTool::new(mq.clone())));
        reg.register(Arc::new(edit::EditTool::new(mq.clone())));
        reg.register(Arc::new(bash::BashTool));
        reg.register(Arc::new(grep::GrepTool));
        reg.register(Arc::new(find::FindTool));
        reg.register(Arc::new(ls::LsTool));
        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_empty_on_new() {
        let reg = ToolRegistry::new();
        assert!(reg.list().is_empty());
        assert!(reg.get("read").is_none());
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(read::ReadTool));
        let tool = reg.get("read");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name(), "read");
    }

    #[test]
    fn test_registry_builtins_contains_all() {
        let reg = ToolRegistry::builtins();
        let names: Vec<&str> = reg.list().iter().map(|t| t.name()).collect();
        assert!(names.contains(&"read"));
        assert!(names.contains(&"write"));
        assert!(names.contains(&"edit"));
        assert!(names.contains(&"bash"));
        assert!(names.contains(&"grep"));
        assert!(names.contains(&"find"));
        assert!(names.contains(&"ls"));
        assert_eq!(names.len(), 7);
    }

    #[test]
    fn test_registry_filtered_subset() {
        let reg = ToolRegistry::builtins();
        let allowed: Vec<String> = vec!["read".into(), "bash".into()];
        let filtered = reg.filtered(Some(&allowed));
        assert_eq!(filtered.len(), 2);
        let names: Vec<&str> = filtered.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"read"));
        assert!(names.contains(&"bash"));
    }

    #[test]
    fn test_registry_filtered_all_when_none() {
        let reg = ToolRegistry::builtins();
        let filtered = reg.filtered(None);
        assert_eq!(filtered.len(), 7);
    }
}
