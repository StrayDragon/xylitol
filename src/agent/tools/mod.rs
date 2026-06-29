pub mod definition;

use std::collections::HashSet;
use std::sync::Arc;

use crate::runtime_protocol::XyTool;

// ── ToolRegistry ───────────────────────────────────────────────────

/// Registry for managing available tools.
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Vec<Arc<dyn XyTool>>,
    /// If set, only tools whose names are in this set are returned by
    /// [`list()`]. Takes precedence over `excluded`.
    allowed: Option<HashSet<String>>,
    /// Tools whose names are in this set are excluded from [`list()`].
    /// Only applied when `allowed` is `None`.
    excluded: Option<HashSet<String>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// Create an empty registry. Use `infra::tools::default_tools() (then wrapped via ToolRegistry::from_tools)` at the
    /// composition root to get one pre-loaded with built-in tools.
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            allowed: None,
            excluded: None,
        }
    }

    /// Build a registry from an iterable of tools (e.g. `infra::tools::default_tools()`).
    pub fn from_tools<I: IntoIterator<Item = Arc<dyn XyTool>>>(tools: I) -> Self {
        let mut reg = Self::new();
        for t in tools {
            reg.register(t);
        }
        reg
    }

    pub fn register(&mut self, tool: Arc<dyn XyTool>) {
        self.tools.push(tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn XyTool>> {
        self.tools.iter().find(|t| t.name() == name).cloned()
    }

    /// Set the allowed tool names. When set, only tools in this set are
    /// returned by [`list()`]. Pass `None` to clear.
    pub fn set_allowed_tool_names(&mut self, names: Option<HashSet<String>>) {
        self.allowed = names;
    }

    /// Set the excluded tool names. Tools in this set are filtered out from
    /// [`list()`]. Only applied when `allowed` is `None`. Pass `None` to clear.
    pub fn set_excluded_tool_names(&mut self, names: Option<HashSet<String>>) {
        self.excluded = names;
    }

    /// Return the list of tools, respecting `allowed` and `excluded` filters.
    pub fn list(&self) -> &[Arc<dyn XyTool>] {
        // Note: filtering is applied in the `list_filtered()` method.
        // This returns all tools; consumers should use `list_filtered()`
        // to respect access controls.
        &self.tools
    }

    /// Return tools respecting `allowed` and `excluded` filters.
    ///
    /// Precedence: `allowed` wins (only allowed tools are returned),
    /// then `excluded` is applied on top.
    pub fn list_filtered(&self) -> Vec<&Arc<dyn XyTool>> {
        let mut iter: Box<dyn Iterator<Item = &Arc<dyn XyTool>>> = Box::new(self.tools.iter());

        if let Some(ref allowed) = self.allowed {
            iter = Box::new(iter.filter(move |t| allowed.contains(t.name())));
        } else if let Some(ref excluded) = self.excluded {
            iter = Box::new(iter.filter(move |t| !excluded.contains(t.name())));
        }

        iter.collect()
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

    /// Create an empty registry (alias of [`new`](Self::new)).
    ///
    /// Historically this registered all built-in tools; that construction now
    /// lives in `infra::tools::default_tools() (then wrapped via ToolRegistry::from_tools)` so the agent layer does not
    /// name concrete tool types. Existing call sites should switch to that.
    pub fn builtins() -> Self {
        Self::new()
    }
}

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
    use super::*;

    #[test]
    fn test_registry_empty_on_new() {
        let reg = ToolRegistry::new();
        assert!(reg.list().is_empty());
        assert!(reg.get("read").is_none());
    }

    #[test]
    fn test_registry_register_and_get() {
        struct DummyTool;
        #[async_trait::async_trait]
        impl crate::runtime_protocol::XyTool for DummyTool {
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
                _: &crate::runtime_protocol::XyToolCtx,
                _: serde_json::Value,
            ) -> Result<String, crate::domain::error::XyToolError> {
                Ok("ok".into())
            }
        }
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool));
        let tool = reg.get("dummy");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name(), "dummy");
    }

    #[test]
    fn test_registry_builtins_contains_all() {
        let reg = ToolRegistry::from_tools(crate::infra::tools::default_tools());
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
        let reg = ToolRegistry::from_tools(crate::infra::tools::default_tools());
        let allowed: Vec<String> = vec!["read".into(), "bash".into()];
        let filtered = reg.filtered(Some(&allowed));
        assert_eq!(filtered.len(), 2);
        let names: Vec<&str> = filtered.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"read"));
        assert!(names.contains(&"bash"));
    }

    #[test]
    fn test_registry_filtered_all_when_none() {
        let reg = ToolRegistry::from_tools(crate::infra::tools::default_tools());
        let filtered = reg.filtered(None);
        assert_eq!(filtered.len(), 7);
    }
}
