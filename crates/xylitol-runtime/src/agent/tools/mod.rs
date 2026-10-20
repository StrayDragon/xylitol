pub mod freeze;
pub mod provider_safe_names;
pub mod toolset;

pub use freeze::{MCP_FIRST_TURN_GATE_TIMEOUT, ToolFreezePhase, ToolTableFingerprint};
pub use toolset::ToolSet;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use xylitol_plugin_sdk::XyTool;

    #[test]
    fn test_toolset_empty() {
        let set = ToolSet::empty();
        assert_eq!(set.iter().count(), 0);
        assert!(set.get("read").is_none());
    }

    #[test]
    fn test_toolset_from_iter_and_get() {
        struct DummyTool;
        #[async_trait::async_trait]
        impl xylitol_plugin_sdk::XyTool for DummyTool {
            fn name(&self) -> &str {
                "dummy"
            }
            fn description(&self) -> &str {
                "dummy"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({})
            }
            async fn execute(
                &self,
                _: &xylitol_plugin_sdk::XyToolCtx,
                _: serde_json::Value,
            ) -> Result<String, xylitol_plugin_sdk::error::XyToolError> {
                Ok("ok".into())
            }
        }
        let tool: Arc<dyn XyTool> = Arc::new(DummyTool);
        let set = ToolSet::from_iter(vec![tool]);
        let tool = set.get("dummy");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name(), "dummy");
    }

    #[test]
    fn test_toolset_remove_and_retain() {
        let set = crate::test_support::named_tools(&[
            "read", "write", "edit", "bash", "grep", "find", "ls",
        ])
        .remove("bash")
        .retain(|t| matches!(t.name(), "read" | "write" | "edit" | "grep" | "find" | "ls"));
        let names: Vec<&str> = set.iter().map(|t| t.name()).collect();
        assert!(!names.contains(&"bash"));
        assert!(names.contains(&"read"));
        assert_eq!(names.len(), 6);
    }

    #[test]
    fn overlay_by_name_double_rebuild_keeps_unique_names() {
        struct Named(&'static str);
        #[async_trait::async_trait]
        impl xylitol_plugin_sdk::XyTool for Named {
            fn name(&self) -> &str {
                self.0
            }
            fn description(&self) -> &str {
                "n"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({})
            }
            async fn execute(
                &self,
                _: &xylitol_plugin_sdk::XyToolCtx,
                _: serde_json::Value,
            ) -> Result<String, xylitol_plugin_sdk::error::XyToolError> {
                Ok("ok".into())
            }
        }
        let builtins =
            || -> Vec<Arc<dyn XyTool>> { vec![Arc::new(Named("read")), Arc::new(Named("bash"))] };
        let mcp = || -> Vec<Arc<dyn XyTool>> {
            vec![
                Arc::new(Named("mcp__fs__read")),
                Arc::new(Named("mcp__git__status")),
            ]
        };
        let once = ToolSet::rebuild_agent_tools(builtins(), mcp());
        let twice = once
            .clone()
            .overlay_by_name(ToolSet::rebuild_agent_tools(builtins(), mcp()));
        let names: Vec<&str> = twice.iter().map(|t| t.name()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(names.len(), sorted.len(), "duplicate names: {names:?}");
        assert_eq!(names.len(), 4);
        assert!(names.contains(&"mcp__fs__read"));
        assert!(names.contains(&"read"));
    }
}
