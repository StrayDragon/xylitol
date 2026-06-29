# c295 — Design Notes

## 1. 核心原则

> 先合并，再重命名；重命名只针对公共对外接口。

- **合并**：消除同一概念多个类型。
- **命名空间前缀 `Xy`**：用于标识“xylitol 对外暴露的接口/定义/工具”，帮助调用者在混合依赖环境中快速识别项目自有 API。
- **不改动 wire 格式**：`protocol::Command` / `protocol::Event` 的 JSON 字段名保持不变。

## 2. 实体合并细节

### 2.1 Usage

当前 `Usage` 与 `XyUsage` 字段不完全对应。合并后统一使用 `XyUsage`（原 `Usage` 改名）：

```rust
pub struct XyUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub cache_write_1h: u64,
    pub total_tokens: u64,
    pub cost: Option<XyUsageCost>,
}
```

`token_estimator::calculate_context_tokens` 改为接收 `&XyUsage`，按 `total_tokens > 0 ? total_tokens : input+output+cache_read+cache_write` 计算。

### 2.2 StopReason

`XyFinishReason` 的 `Stop` / `MaxTokens` 是 `StopReason` 的子集。统一后 `XyChunk::Done` 使用完整 `XyStopReason`。

### 2.3 Compaction 配置

```text
YAML/JSON file load
      │
      ▼
XyCompactionSettingsConfig  (all optional, serde)
      │
      ▼ From impl
XyCompactionSettings        (runtime, defaults applied)
```

删除只有 `enabled` 的旧 `CompactionConfig`。

### 2.4 ToolDefinition / XyToolSchema

```rust
pub struct XyToolDefinition {
    pub schema: XyToolSchema,
    pub prompt_snippet: Option<String>,
    pub prompt_guidelines: Vec<String>,
    pub execution_mode: XyToolExecutionMode,
    pub source_info: Option<XySourceInfo>,
}
```

`XyToolSchema` 继续作为“可序列化工具契约”存在于 `domain::types`。

### 2.5 事件体系

统一为单一领域事件枚举 `XyEvent`（原 `AgentLifecycleEvent` 改名）：

```rust
pub enum XyEvent {
    AgentStart { session_id: String, model: String },
    AgentEnd { session_id: String, reason: String },
    TurnStart { turn_index: u32 },
    TurnEnd { turn_index: u32 },
    MessageStart { role: String, message: Option<XyAgentMessage> },
    MessageUpdate { text: String, thinking: Option<String>, message: Option<XyAgentMessage> },
    MessageEnd { role: String, message: Option<XyAgentMessage> },
    ToolExecutionStart { id: String, name: String, args: Value },
    ToolExecutionUpdate { id: String, output: String },
    ToolExecutionEnd { id: String, name: String, result: String, is_error: bool },
    CompactionStart { reason: String },
    CompactionEnd { result: Option<String>, aborted: bool },
    ModelSelect { provider: String, model_id: String },
    ThinkingLevelChanged { level: String },
    QueueUpdate { steer_count: usize, follow_up_count: usize },
    AutoRetryStart { attempt: u32, max_retries: u32, delay_ms: u64 },
    AutoRetryEnd { success: bool, attempt: u32 },
    SessionInfoChanged { key: String, value: Value },
}
```

`agent::runtime` 输出流为 `XyEventStream`。`XyEventSink` 接收 `&XyEvent`。

`protocol::Event` 保持独立的 wire 表示，通过 `From`/`TryFrom` 转换：

```rust
impl From<&XyEvent> for protocol::Event { ... }
impl TryFrom<&protocol::Event> for XyEvent { ... }
```

## 3. `Xy*` 前缀决策矩阵

| 类型类别 | 是否加 `Xy` | 示例 |
|---|---|---|
| `runtime_protocol` port trait | ✅ | `XyModel`, `XySessionStore` |
| `runtime_protocol` 伴生类型 | ✅ | `XyStream`, `XyBashResult` |
| `domain` 核心对外抽象 | ✅ | `XyEvent`, `XyModelConfig`, `XyUsage` |
| `domain` 消息/持久化专用类型 | ❌ | `AgentMessage`, `SessionEntry` |
| `protocol` wire 类型 | ❌ | `Command`, `Event` |
| `infra` 具体实现 | ❌ | `EventBus`, `InfraBashExecutor` |
| `agent` 内部编排类型 | ❌ | `AgentLoop`, `ToolRegistry` |

## 4. 迁移路径

1. 用 `rg` 列出每个旧类型的所有引用点。
2. 用 `sd` 做精确替换（按完整词边界，避免子串误伤）。
3. 每替换一个类型后 `cargo check`。
4. 全部替换后 `cargo fmt`。
5. 跑完整测试套件，更新快照。

## 5. 不做的事

- 不改 `protocol::Command` / `protocol::Event` 的 JSON 字段名。
- 不改动 `infra/` 中具体实现的命名。
- 不引入 workspace / 不拆分 crate。
- 不新增功能或行为。
