# Design: c170-refactor-agent-message-types

## Approach

1. 新建 `src/agent/message.rs` 模块，包含 `AgentMessage` 枚举和 `AgentPart` 枚举
2. 保持与现有 `XyContent`/`XyPart` 的兼容过渡期：新类型与旧类型共存，逐步替换
3. `AgentMessage` 使用 serde 序列化以兼容 JSONL session 格式
4. LLM 消息转换层（`convert_to_llm()`）作为 provider 边界的 trait 方法
5. Session 版本迁移：v3→v4 在 `SessionManager` 加载时自动执行

## Key Types

```rust
pub enum AgentMessage {
    UserMessage { content: Vec<AgentPart>, timestamp: i64 },
    AssistantMessage {
        content: Vec<AgentPart>,
        stop_reason: StopReason,
        usage: Option<Usage>,
        timestamp: i64,
    },
    ToolResultMessage { ... },
    BashExecutionMessage { command: String, output: String, exit_code: Option<i32>, ... },
    CustomMessage { custom_type: String, content: Value, display: Value, details: Value },
    CompactionSummaryMessage { summary: String, tokens_before: u64, ... },
    BranchSummaryMessage { summary: String, from_id: String, ... },
}

pub enum AgentPart {
    Text(String),
    Image { url: Option<String>, data: Option<String>, media_type: String },
    Thinking(String),
    ToolCall { id: String, name: String, arguments: Value },
    ToolResult { tool_use_id: String, content: Vec<AgentPart>, is_error: bool },
}
```

## Migration Path

- Phase 1: 定义新类型，内部转换 `AgentMessage` ↔ `XyContent`
- Phase 2: 逐个替换领域模块中的 `XyContent` 引用
- Phase 3: 删除 `XyContent`/`XyPart`
