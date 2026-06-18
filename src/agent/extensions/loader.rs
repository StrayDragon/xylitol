//! Extension loader — loads extensions from configured paths.
//!
//! Supports two loading strategies:
//! - File-based: load a `.rs` module via dynamic library (future: wasm, dylib)
//! - Built-in: extension provided as a factory function
//!
//! Aligns with pi's extensions/loader.ts.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use super::types::{
    ExtensionContext, ExtensionEvent, RegisteredTool, ToolDefinition, ToolResultHookResult,
    ToolSourceInfo,
};

// ── Extension trait ─────────────────────────────────────────────────

/// Extension trait — implement this to create a custom extension.
///
/// Aligns with pi's `ExtensionFactory` pattern.
pub trait Extension: Send + Sync {
    /// Called when the extension is activated.
    fn init(&self) {}

    /// Return the extension name.
    fn name(&self) -> &str;

    /// Return tools registered by this extension.
    fn tools(&self) -> Vec<Arc<dyn ToolDefinition>> {
        vec![]
    }

    /// Handle a lifecycle event.
    ///
    /// Return Some(ToolCallHookResult) for before_tool_call events,
    /// Some(ToolResultHookResult) for after_tool_call events,
    /// None for observe-only events.
    fn on_event(
        &self,
        _event: &ExtensionEvent,
        _ctx: &ExtensionContext,
    ) -> Option<ExtensionEventResult> {
        None
    }
}

// ── Event Result ────────────────────────────────────────────────────

/// Result of an extension event handler.
#[derive(Debug, Clone)]
pub enum ExtensionEventResult {
    /// Block a tool call.
    BlockToolCall { reason: String },
    /// Modify a tool result.
    ModifyToolResult {
        content: Option<serde_json::Value>,
        is_error: Option<bool>,
    },
}

// ── ExtensionLoader ─────────────────────────────────────────────────

/// Loads extensions from configured paths.
pub struct ExtensionLoader {
    /// Loaded extensions.
    extensions: Vec<Arc<dyn Extension>>,
    /// Tool registry (extension_name -> tools).
    tools: HashMap<String, Vec<RegisteredTool>>,
}

impl Default for ExtensionLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtensionLoader {
    /// Create an empty extension loader.
    pub fn new() -> Self {
        Self {
            extensions: Vec::new(),
            tools: HashMap::new(),
        }
    }

    /// Load extensions from a directory.
    ///
    /// Currently loads Rust-native extensions registered via `add_extension()`.
    /// Future: supports loading `.wasm` or `.so`/`.dylib` files.
    pub fn load_from_dir(&mut self, _dir: &Path) -> Result<usize, String> {
        // Placeholder — future implementation
        // For now, extensions are registered via add_extension()
        Ok(0)
    }

    /// Add a pre-built extension (factory style).
    pub fn add_extension(&mut self, extension: Arc<dyn Extension>) {
        let name = extension.name().to_string();
        let source_path: Option<String> = None;

        // Register tools
        let ext_tools: Vec<RegisteredTool> = extension
            .tools()
            .into_iter()
            .map(|def| RegisteredTool {
                definition: def,
                source_info: ToolSourceInfo {
                    extension_name: name.clone(),
                    source_path: source_path.clone(),
                },
            })
            .collect();

        if !ext_tools.is_empty() {
            self.tools.insert(name.clone(), ext_tools);
        }

        self.extensions.push(extension);
    }

    /// Get all loaded extensions.
    pub fn extensions(&self) -> &[Arc<dyn Extension>] {
        &self.extensions
    }

    /// Get all registered tools from all extensions.
    pub fn get_all_tools(&self) -> Vec<&RegisteredTool> {
        self.tools.values().flat_map(|v| v.iter()).collect()
    }

    /// Dispatch an event to all extensions.
    pub fn dispatch_event(
        &self,
        event: &ExtensionEvent,
        ctx: &ExtensionContext,
    ) -> Vec<ExtensionEventResult> {
        let mut results = Vec::new();
        for ext in &self.extensions {
            if let Some(result) = ext.on_event(event, ctx) {
                results.push(result);
            }
        }
        results
    }

    /// Check if any extension wants to block a tool call.
    /// Returns the first blocking reason, or None.
    pub fn check_tool_call_blocked(
        &self,
        tool_call_id: &str,
        tool_name: &str,
        args: &serde_json::Value,
        ctx: &ExtensionContext,
    ) -> Option<String> {
        let event = ExtensionEvent::BeforeToolCall {
            tool_call_id: tool_call_id.to_string(),
            tool_name: tool_name.to_string(),
            args: args.clone(),
        };
        for result in self.dispatch_event(&event, ctx) {
            if let ExtensionEventResult::BlockToolCall { reason } = result {
                return Some(reason);
            }
        }
        None
    }

    /// After a tool call, let extensions modify the result.
    pub fn modify_tool_result(
        &self,
        tool_call_id: &str,
        tool_name: &str,
        result: &serde_json::Value,
        is_error: bool,
        ctx: &ExtensionContext,
    ) -> Option<ToolResultHookResult> {
        let event = ExtensionEvent::AfterToolCall {
            tool_call_id: tool_call_id.to_string(),
            tool_name: tool_name.to_string(),
            result: result.clone(),
            is_error,
        };
        let mut modified = false;
        let mut new_content = None;
        let mut new_is_error = None;

        for ext_result in self.dispatch_event(&event, ctx) {
            if let ExtensionEventResult::ModifyToolResult {
                content,
                is_error: err,
            } = ext_result
            {
                modified = true;
                if content.is_some() {
                    new_content = content;
                }
                if err.is_some() {
                    new_is_error = err;
                }
            }
        }

        if modified {
            Some(ToolResultHookResult {
                content: new_content,
                is_error: new_is_error,
            })
        } else {
            None
        }
    }
}

// ── Extension Runner ────────────────────────────────────────────────

/// Manages the lifecycle of loaded extensions.
#[derive(Default)]
pub struct ExtensionRunner {
    loader: ExtensionLoader,
}

impl ExtensionRunner {
    /// Create a new ExtensionRunner with an empty loader.
    pub fn new() -> Self {
        Self {
            loader: ExtensionLoader::new(),
        }
    }

    /// Get the underlying loader.
    pub fn loader(&self) -> &ExtensionLoader {
        &self.loader
    }

    /// Get mutable access to the loader.
    pub fn loader_mut(&mut self) -> &mut ExtensionLoader {
        &mut self.loader
    }

    /// Activate all extensions.
    pub fn activate(&self) {
        for ext in self.loader.extensions() {
            ext.init();
        }
    }

    /// Dispatch an event to all extensions.
    pub fn dispatch(
        &self,
        event: &ExtensionEvent,
        ctx: &ExtensionContext,
    ) -> Vec<ExtensionEventResult> {
        self.loader.dispatch_event(event, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::extensions::types::{
        ExtensionContext, ExtensionEvent, ToolCallHookResult, ToolResultHookResult,
    };
    use std::sync::Arc;

    // ── Test tool ──

    struct GreetTool;

    impl ToolDefinition for GreetTool {
        fn name(&self) -> &str {
            "greet"
        }
        fn description(&self) -> &str {
            "Greet someone by name"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Name to greet" }
                },
                "required": ["name"]
            })
        }
        fn execute(
            &self,
            _tool_call_id: &str,
            params: serde_json::Value,
            _signal: Option<Arc<tokio_util::sync::CancellationToken>>,
            _ctx: &ExtensionContext,
        ) -> Result<serde_json::Value, String> {
            let name = params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("world");
            Ok(serde_json::json!({"greeting": format!("Hello, {}!", name)}))
        }
    }

    // ── Test extension ──

    struct TestExtension {
        name: String,
        tool: Option<Arc<dyn ToolDefinition>>,
    }

    impl TestExtension {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                tool: None,
            }
        }

        fn with_tool(name: &str) -> Self {
            Self {
                name: name.to_string(),
                tool: Some(Arc::new(GreetTool)),
            }
        }
    }

    impl Extension for TestExtension {
        fn name(&self) -> &str {
            &self.name
        }

        fn tools(&self) -> Vec<Arc<dyn ToolDefinition>> {
            self.tool.iter().cloned().collect()
        }
    }

    // ── Hook extension (block / modify) ──

    struct BlockBashExtension;

    impl Extension for BlockBashExtension {
        fn name(&self) -> &str {
            "block-bash"
        }

        fn on_event(
            &self,
            event: &ExtensionEvent,
            _ctx: &ExtensionContext,
        ) -> Option<ExtensionEventResult> {
            if let ExtensionEvent::BeforeToolCall { tool_name, .. } = event {
                if tool_name == "bash" {
                    return Some(ExtensionEventResult::BlockToolCall {
                        reason: "bash is blocked by policy".into(),
                    });
                }
            }
            None
        }
    }

    struct ModifyOutputExtension;

    impl Extension for ModifyOutputExtension {
        fn name(&self) -> &str {
            "modify-output"
        }

        fn on_event(
            &self,
            event: &ExtensionEvent,
            _ctx: &ExtensionContext,
        ) -> Option<ExtensionEventResult> {
            if let ExtensionEvent::AfterToolCall { tool_name, .. } = event {
                if tool_name == "read" {
                    return Some(ExtensionEventResult::ModifyToolResult {
                        content: Some(serde_json::json!({"content": "[redacted]"})),
                        is_error: None,
                    });
                }
            }
            None
        }
    }

    // ── Tests ──

    #[test]
    fn test_register_tool() {
        let mut loader = ExtensionLoader::new();
        loader.add_extension(Arc::new(TestExtension::with_tool("test-ext")));

        let tools = loader.get_all_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].definition.name(), "greet");
    }

    #[test]
    fn test_tool_execute() {
        let tool = GreetTool;
        let ctx = ExtensionContext::new(
            "/tmp".into(),
            Arc::new(crate::infra::session::SessionManager::default()),
            Arc::new(crate::agent::registry::ModelRegistry::default()),
            None,
            None,
        );
        let result = tool
            .execute("call-1", serde_json::json!({"name": "Alice"}), None, &ctx)
            .unwrap();
        assert_eq!(result["greeting"], "Hello, Alice!");
    }

    #[test]
    fn test_block_tool_call() {
        let mut loader = ExtensionLoader::new();
        loader.add_extension(Arc::new(BlockBashExtension));

        let ctx = ExtensionContext::new(
            "/tmp".into(),
            Arc::new(crate::infra::session::SessionManager::default()),
            Arc::new(crate::agent::registry::ModelRegistry::default()),
            None,
            None,
        );

        let reason = loader.check_tool_call_blocked(
            "call-1",
            "bash",
            &serde_json::json!({"command": "rm -rf /"}),
            &ctx,
        );
        assert_eq!(reason, Some("bash is blocked by policy".into()));

        // read should not be blocked
        let reason = loader.check_tool_call_blocked(
            "call-2",
            "read",
            &serde_json::json!({"path": "file.txt"}),
            &ctx,
        );
        assert_eq!(reason, None);
    }

    #[test]
    fn test_modify_tool_result() {
        let mut loader = ExtensionLoader::new();
        loader.add_extension(Arc::new(ModifyOutputExtension));

        let ctx = ExtensionContext::new(
            "/tmp".into(),
            Arc::new(crate::infra::session::SessionManager::default()),
            Arc::new(crate::agent::registry::ModelRegistry::default()),
            None,
            None,
        );

        let modified = loader.modify_tool_result(
            "call-1",
            "read",
            &serde_json::json!({"content": "secret data"}),
            false,
            &ctx,
        );
        assert!(modified.is_some());
        let m = modified.unwrap();
        assert_eq!(
            m.content,
            Some(serde_json::json!({"content": "[redacted]"}))
        );
    }

    #[test]
    fn test_extension_context_abort() {
        let token = tokio_util::sync::CancellationToken::new();
        let ctx = ExtensionContext::new(
            "/tmp".into(),
            Arc::new(crate::infra::session::SessionManager::default()),
            Arc::new(crate::agent::registry::ModelRegistry::default()),
            None,
            Some(Arc::new(token.clone())),
        );

        assert!(!ctx.is_aborted());
        ctx.abort();
        assert!(ctx.is_aborted());
    }

    #[test]
    fn test_extension_event_types() {
        let event = ExtensionEvent::ToolExecutionStart {
            tool_call_id: "call-1".into(),
            tool_name: "bash".into(),
            args: serde_json::json!({"command": "ls"}),
        };
        assert_eq!(event.event_type(), "tool_execution_start");

        let event = ExtensionEvent::AfterToolCall {
            tool_call_id: "call-1".into(),
            tool_name: "bash".into(),
            result: serde_json::json!({"ok": true}),
            is_error: false,
        };
        assert_eq!(event.event_type(), "after_tool_call");
    }

    #[test]
    fn test_extension_runner_activate() {
        let mut runner = ExtensionRunner::new();
        runner
            .loader_mut()
            .add_extension(Arc::new(TestExtension::new("ext-1")));
        runner
            .loader_mut()
            .add_extension(Arc::new(TestExtension::new("ext-2")));
        // init should not panic
        runner.activate();
        assert_eq!(runner.loader().extensions().len(), 2);
    }
}
