# c272-complete-core-event-mapping — Design

## 1. protocol::Event 增补

```rust
pub enum Event {
    // existing variants unchanged...
    TurnStart { turn_index: u32 },
    TurnEnd { turn_index: u32 },
    MessageStart { role: String },
    MessageEnd { role: String },
    MessageUpdate { text: String, thinking: Option<String> },
    ToolExecutionUpdate { id: String, output: String },
    CompactionEnd,
}
```

新 variant 使用 `#[serde(tag = "type", rename_all = "snake_case")]`，与现有风格一致。老的 client 收到未知 tag 可通过 `#[serde(other)]` 忽略（需在接收端确认）。

## 2. AgentSession port 改造

`AgentSession::new` 签名变更：

```rust
// Before:
pub fn new(
    model_registry: ModelRegistry,
    tool_registry: ToolRegistry,
    session_manager: SessionManager,       // concrete
    system_prompt: Option<String>,
    ...
) -> Self

// After:
pub fn new(
    model_registry: ModelRegistry,
    tool_registry: ToolRegistry,
    store: Arc<dyn SessionStore>,           // port
    sink: Arc<dyn EventSink>,              // port
    system_prompt: Option<String>,
    ...
) -> Self
```

`SessionIO` 内部通过 `Arc<dyn SessionStore>` 调用 `load_context`/`append_entry`/`exists`，不再需要具体 `SessionManager` 类型。

`EventSink` 在 `emit_lifecycle` 调用点替换 `EventBus::emit`。注意：event_bus 的订阅/退订功能（`subscribe`/`unsubscribe`）仍需要具体 `EventBus`——sink port 只覆盖生命周期事件。保留 `EventBus` 作为 sink 的具体实现，loop 内通过 sink port 发射事件。

## 3. RPC Agent 缓存

`RpcState` 增加 `cached_agent: Option<Agent>`。`ensure_agent()` 检查缓存是否有效（session_id / model / thinking 无变化），有效则直接返回 `&mut Agent`，无效则替换。

重建触发条件：session_id 变化、current_model_id 变化、thinking_level 变化。ToolRegistry 在所有命令中固定（`default_tools()`），无需重建。

## 4. print.rs 事件补全

补全的事件按以下方式渲染：
- `TurnStart` — eprintln! `[Turn {turn_index}]`
- `TurnEnd` — eprintln! `[Turn {turn_index} end]`
- `MessageStart` — 静默（不中断输出流）
- `MessageEnd` — 静默
- `MessageUpdate` — 覆盖当前行显示 text/thinking（与 TextDelta 类似）
- `ToolExecutionUpdate` — eprintln! 流式输出
- `CompactionEnd` — eprintln! `[Compaction complete]`
