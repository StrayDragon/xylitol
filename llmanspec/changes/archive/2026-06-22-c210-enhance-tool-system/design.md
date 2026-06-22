# Design: c210-enhance-tool-system

## Tool Filtering

```rust
impl ToolRegistry {
    pub fn set_allowed(&mut self, names: Option<HashSet<String>>);
    pub fn set_excluded(&mut self, names: Option<HashSet<String>>);
    pub fn list(&self) -> Vec<&dyn XyTool>;  // respects both filters
}
```

Filter precedence: if `allowed` is set, only tools in allowed set are returned. Then `excluded` is applied on top.

## Per-Tool Execution Mode

```rust
trait XyTool {
    fn execution_mode(&self) -> ToolExecutionMode { ToolExecutionMode::Parallel }
}
```

When batch contains ANY sequential tool → entire batch runs sequentially.

## Argument Validation

```rust
fn validate_tool_arguments(tool: &dyn XyTool, args: &Value) -> Result<Value, ToolValidationError>
```

Validates against `tool.parameters_schema()` JSON Schema using `jsonschema` crate.
