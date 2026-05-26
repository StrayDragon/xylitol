# Design: c104-refactor-core-traits

## XyModel Trait

```rust
pub trait XyModel: Send + Sync {
    fn generate_stream(
        &self,
        messages: Vec<XyContent>,
        tools: &[XyToolSchema],
        stream: bool,
    ) -> Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>;
}
```

与 `adk_core::Llm` 的差异：
- 参数用自有类型而非 `LlmRequest`
- 直接接受 tool schema 列表（不需要 `Tool` trait 对象）
- 返回 `XyChunk`（自有 streaming chunk 类型）

## XyTool Trait

```rust
#[async_trait]
pub trait XyTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, args: serde_json::Value) -> Result<String, XyToolError>;
}
```

与 `adk_core::Tool` 的差异：
- 返回 `String` 而非 `Vec<Part>`（tool output 在 xylitol 中总是文本）
- 不需要 `ToolContext`（安全检查在 wrapper 层处理）
- 错误类型是自有的 `XyToolError`

## XyContent / XyPart 类型

```rust
pub struct XyContent {
    pub role: Role,
    pub parts: Vec<XyPart>,
}

pub enum Role { System, User, Assistant, Tool }

pub enum XyPart {
    Text(String),
    Thinking(String),
    FunctionCall { name: String, args: String, id: String },
    FunctionResponse { name: String, result: String, id: String },
}

pub enum XyChunk {
    TextDelta(String),
    ThinkingDelta(String),
    FunctionCallDelta { name: Option<String>, args_delta: String, id: Option<String> },
    Done,
}
```

## XyError 体系

```rust
#[derive(Debug, thiserror::Error)]
pub enum XyError {
    #[error("provider error: {0}")]
    Provider(#[source] anyhow::Error),
    #[error("tool error: {0}")]
    Tool(#[from] XyToolError),
    #[error("session error: {0}")]
    Session(#[source] anyhow::Error),
    #[error("max iterations reached ({0})")]
    MaxIterations(usize),
}

#[derive(Debug, thiserror::Error)]
pub enum XyToolError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("execution failed: {0}")]
    ExecutionFailed(#[source] anyhow::Error),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("timeout after {0:?}")]
    Timeout(std::time::Duration),
}
```

## Adapter Pattern（兼容层）

在阶段三（c105）之前，adk-runner 仍需要 `adk_core::Llm` 和 `adk_core::Tool`。
通过 thin adapter 桥接：

```rust
pub struct XyModelToLlm(Arc<dyn XyModel>);
impl adk_core::Llm for XyModelToLlm {
    // 将 LlmRequest → Vec<XyContent>，调用 XyModel，将 XyChunk → LlmResponse
}

pub struct XyToolToTool(Arc<dyn XyTool>);
impl adk_core::Tool for XyToolToTool {
    // 将 ToolContext args → serde_json::Value，调用 XyTool，将 String → Vec<Part>
}
```

adapter 集中在 `src/agent/compat.rs`，是 adk 类型的唯一出现位置（除 `loop.rs`）。

## 迁移策略

按依赖深度从底向上迁移：
1. 先定义类型 + trait（`types.rs`、`error.rs`、`traits.rs`）
2. 迁移 tool 实现（7 个文件，机械替换）
3. 迁移 wrapper（security、mcp、approval）
4. 迁移 provider（openai、anthropic、fake）
5. 创建 compat adapter
6. 更新 `loop.rs` 和 `planner.rs` 内部使用自有类型

每一步都保持 `cargo build` 通过。
