pub(crate) mod bash;
pub(crate) mod edit;
pub(crate) mod find;
pub(crate) mod grep;
pub(crate) mod ls;
pub(crate) mod patch;
pub(crate) mod read;
pub(crate) mod write;

use std::sync::Arc;

use crate::agent::traits::XyTool;

/// Registry for managing available tools.
#[derive(Clone)]
pub(crate) struct ToolRegistry {
    tools: Vec<Arc<dyn XyTool>>,
}

impl ToolRegistry {
    pub(crate) fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub(crate) fn register(&mut self, tool: Arc<dyn XyTool>) {
        self.tools.push(tool);
    }

    pub(crate) fn get(&self, name: &str) -> Option<Arc<dyn XyTool>> {
        self.tools.iter().find(|t| t.name() == name).cloned()
    }

    pub(crate) fn list(&self) -> &[Arc<dyn XyTool>] {
        &self.tools
    }

    /// Transform each registered tool in-place.
    pub(crate) fn map_tools<F>(&mut self, f: F)
    where
        F: FnMut(Arc<dyn XyTool>) -> Arc<dyn XyTool>,
    {
        self.tools = self.tools.iter().cloned().map(f).collect();
    }

    /// Return tools matching the given names. Returns all if `allowed` is `None` or empty.
    pub(crate) fn filtered(&self, allowed: Option<&[String]>) -> Vec<Arc<dyn XyTool>> {
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

    /// Wrap every registered tool with a security-checking wrapper.
    pub(crate) fn wrap_with_security(&mut self, engine: crate::infra::security::SecurityEngine) {
        let engine = std::sync::Arc::new(engine);
        self.tools = self
            .tools
            .iter()
            .map(|t| {
                let w =
                    crate::infra::security::SecurityToolWrapper::new(t.clone(), (*engine).clone());
                Arc::new(w) as Arc<dyn XyTool>
            })
            .collect();
    }

    /// Create a registry with all built-in tools registered.
    pub(crate) fn builtins() -> Self {
        let mut reg = Self::new();
        reg.register(Arc::new(read::ReadTool));
        reg.register(Arc::new(write::WriteTool));
        reg.register(Arc::new(edit::EditTool));
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
}
