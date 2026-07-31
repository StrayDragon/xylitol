pub mod toolset;

pub use toolset::ToolSet;

use crate::protocol::ports::XyTool;

// ── Argument validation ─────────────────────────────────────────

/// Error returned from argument validation.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ToolValidationError {
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Type error at field '{field}': expected {expected}")]
    TypeError { field: String, expected: String },
    #[error("Unexpected field: {0}")]
    UnexpectedField(String),
    #[error("Validation failed: {0}")]
    Other(String),
}

/// Validate tool call arguments against the tool's JSON parameter schema.
pub fn validate_tool_arguments(
    tool: &dyn XyTool,
    args: &serde_json::Value,
) -> Result<(), ToolValidationError> {
    let schema = tool.parameters_schema();

    if !args.is_object() && !schema.is_null() {
        return Err(ToolValidationError::TypeError {
            field: "(root)".to_string(),
            expected: "object".to_string(),
        });
    }

    // Check required fields
    if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
        for field in required {
            let field_name = field.as_str().unwrap_or("");
            if args.get(field_name).filter(|v| !v.is_null()).is_none() {
                return Err(ToolValidationError::MissingField(field_name.to_string()));
            }
        }
    }

    // Check types and unexpected fields
    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object())
        && let Some(obj) = args.as_object()
    {
        for key in obj.keys() {
            if !properties.contains_key(key) {
                return Err(ToolValidationError::UnexpectedField(key.clone()));
            }
            if let Some(prop_schema) = properties.get(key)
                && let Some(expected_type) = prop_schema.get("type").and_then(|t| t.as_str())
            {
                let val = &obj[key];
                let type_ok = match expected_type {
                    "string" => val.is_string(),
                    "integer" | "number" => val.is_number(),
                    "boolean" => val.is_boolean(),
                    "array" => val.is_array(),
                    "object" => val.is_object(),
                    _ => true,
                };
                if !type_ok {
                    return Err(ToolValidationError::TypeError {
                        field: key.clone(),
                        expected: expected_type.to_string(),
                    });
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

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
        impl crate::protocol::ports::XyTool for DummyTool {
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
                _: &crate::protocol::ports::XyToolCtx,
                _: serde_json::Value,
            ) -> Result<String, crate::protocol::error::XyToolError> {
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
    fn test_toolset_default_tools_contains_all() {
        let set = ToolSet::from_iter(crate::infra::tools::default_tools());
        let names: Vec<&str> = set.iter().map(|t| t.name()).collect();
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
    fn test_toolset_remove_and_retain() {
        let set = ToolSet::from_iter(crate::infra::tools::default_tools())
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
        impl crate::protocol::ports::XyTool for Named {
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
                _: &crate::protocol::ports::XyToolCtx,
                _: serde_json::Value,
            ) -> Result<String, crate::protocol::error::XyToolError> {
                Ok("ok".into())
            }
        }
        let builtins =
            || -> Vec<Arc<dyn XyTool>> { vec![Arc::new(Named("read")), Arc::new(Named("bash"))] };
        let mcp = || -> Vec<Arc<dyn XyTool>> {
            vec![
                Arc::new(Named("mcp:fs:read")),
                Arc::new(Named("mcp:git:status")),
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
        assert!(names.contains(&"mcp:fs:read"));
        assert!(names.contains(&"read"));
    }
}
