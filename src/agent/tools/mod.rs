/// Tool execution result.
pub(crate) struct ToolResult {
    pub(crate) output: String,
    pub(crate) success: bool,
}

/// Tool trait — expanded in c20.
pub(crate) trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&self, args: &str) -> ToolResult;
}

/// Tool registry — expanded in c20.
pub(crate) struct ToolRegistry;

impl ToolRegistry {
    pub(crate) fn new() -> Self {
        Self
    }
}
