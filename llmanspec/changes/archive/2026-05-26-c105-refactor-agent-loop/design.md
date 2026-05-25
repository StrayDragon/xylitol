# Design: c105-refactor-agent-loop

## XyRunner 核心设计

```rust
pub struct XyRunner {
    model: Arc<dyn XyModel>,
    tools: ToolRegistry,  // holds Arc<dyn XyTool>
    session: Arc<dyn XySession>,
    system_prompt: String,
    max_iterations: usize,
}

impl XyRunner {
    pub fn run(&self, user_input: &str) -> impl Stream<Item = AgentEvent> {
        // 1. 从 session 加载历史
        // 2. 构建 messages: system + history + user_input
        // 3. 调用 model.generate_stream(messages, tools)
        // 4. 解析 streaming chunks:
        //    - Text → emit TextDelta
        //    - Thinking → emit ThinkingDelta
        //    - FunctionCall → emit ToolCallStart, execute tool, emit ToolCallEnd
        // 5. 如果有 tool call → append tool result to messages → goto 3
        // 6. 如果无 tool call → emit StepComplete → 保存 session → done
        // 7. 如果 iteration >= max_iterations → emit Error
    }
}
```

## XySession 设计

```rust
pub trait XySession: Send + Sync {
    fn load(&self, session_id: &str) -> Result<Vec<XyContent>>;
    fn save(&self, session_id: &str, messages: &[XyContent]) -> Result<()>;
}
```

`InMemorySession` 用 `DashMap<String, Vec<XyContent>>`。
与 `infra/session/` 的 snapshot/compaction 系统保持独立——
snapshot 是 xylitol 的持久化层，XySession 是运行时历史。

## Streaming 事件流

保持现有 `AgentEvent` 枚举不变——它已经是 UI 层的稳定契约：

```rust
pub enum AgentEvent {
    TextDelta(String),
    ThinkingDelta(String),
    ToolCallStart { name: String, args: String, id: String },
    ToolCallEnd { name: String, result: String, id: String },
    StepComplete,
    Error(XyError),
}
```

`loop.rs` 中的 `map_adk_event` 函数将被移除——
XyRunner 直接产生 `AgentEvent`，无需映射层。

## Tool Dispatch

```
XyRunner 接收 FunctionCall chunk
  → 在 ToolRegistry 中查找 tool by name
  → 调用 SecurityToolWrapper（如果启用）
  → 执行 XyTool::execute(args)
  → 构建 FunctionResponse
  → append to messages
  → continue loop
```

## 与 Planner 的关系

`planner.rs` 的 `call_llm()` 直接调用 `XyModel::generate_stream`（非 streaming 模式），
不经过 XyRunner。这保持不变——planner 是独立的单轮调用。

## 测试策略

- `FakeProvider` 已实现 `XyModel`（c104），可直接用于 ReAct loop 测试
- 创建 `XyRunner` 的 builder pattern，支持注入 mock model + mock tools
- 现有 `tests/support/harness.rs` 重写为使用 `XyRunner` 而非 `adk Runner`

## 迁移后的依赖变化

```
Before: adk-core, adk-agent, adk-runner, adk-model, adk-session (5 crates)
After:  async-openai, reqwest (2 crates, both widely used)
```
