---
depends_on:
  - c170-refactor-agent-message-types
  - c175-refactor-event-system
  - c180-rebuild-agent-session
  - c185-upgrade-agent-loop
  - c190-extend-session-manager
  - c195-wire-compaction-lifecycle
  - c200-integrate-skills
  - c205-expand-provider-layer
  - c210-enhance-tool-system
---

# c215-align-agent-types: 对齐 pi coding agent 类型系统

## Why

xylitol 核心消息类型与 pi coding agent 的存在结构化差异：

1. **AssistantMessage** 缺少 `api`/`provider`/`model`/`responseId`/`errorMessage`/`timestamp` — 这些字段在 pi 中用于 provider 溯源、错误传播和会话重放
2. **ToolResultMessage** 缺少 `toolName`/`details` — 用于分辨哪个工具产生的哪个结果，以及结构化详情
3. **UserMessage** 缺少 `timestamp` — 用于消息排序和超时检测
4. **Usage** 缺少 `cost` 子对象（计费字段）和 `cacheWrite1h` — 用于 token 计费和缓存分析
5. **ThinkingContent** 缺少 `redacted`/`thinkingSignature` — 用于安全过滤后的 thinking 恢复
6. **事件系统** 缺少 `message_start`/`message_update` 事件 — pi 在消息流式传输过程中发射这些事件
7. **ModelMeta** 缺少 `api`/`provider`/`cost`/`maxTokens`/`thinkingLevelMap` — 不能反映 provider 完整元数据
8. **AgentState**/**AgentContext** 没有作为正式结构体存在 — 分散在 session/loop 的局部变量

同时，旧 `XyContent`/`XyPart`/`XyRole` 类型仍然存在于代码库中（243 处引用），作为内部工作类型。迁移到 `AgentMessage` 就可以彻底清理这些类型。

## What Changes

### 1. AgentMessage 变体字段补齐

| 变体 | 新增字段 |
|------|---------|
| `AssistantMessage` | `api`, `provider`, `model`, `response_id`, `error_message`, `timestamp`, `diagnostics` |
| `ToolResultMessage` | `tool_name`, `details`, `timestamp` |
| `UserMessage` | `timestamp` |

### 2. AgentPart::Thinking 补充

- `redacted: bool` — 是否被安全过滤
- `signature: Option<String>` — 用于 thinking 恢复的签名

### 3. Usage 补充

- `cache_write_1h: u64` — Anthropic 1h 缓存写入
- `cost: UsageCost` （含 `input`/`output`/`cache_read`/`cache_write`/`total` f64 美元计费）

### 4. 事件系统补齐

- `AgentLifecycleEvent::MessageStart { message: AgentMessage }` — 每条消息开始时
- `AgentLifecycleEvent::MessageUpdate { message: AgentMessage, delta: Value }` — 流式更新期间

### 5. 运行时结构体

- `AgentState` — 公开 agent 快照（systemPrompt, model, tools, messages, isStreaming, streamingMessage, pendingToolCalls, errorMessage）
- `AgentContext` — 每次 LLM 调用前的上下文快照（systemPrompt, messages, tools）

### 6. ModelMeta 补齐

- `api: String`
- `provider: String`
- `cost_input: f64` / `cost_output: f64` / `cost_cache_read: f64` / `cost_cache_write: f64`
- `max_tokens: u64`
- `thinking_levels: Vec<String>`

### 7. 删除旧类型

- 删除 `XyContent`、`XyPart`、`XyRole` 定义
- 删除 `from_xy_content`/`into_xy_content`/`from_xy_part`/`into_xy_part` 转换函数
- 内部所有 `Vec<XyContent>` 历史 → `Vec<AgentMessage>`
- 内部所有 `XyPart` 构建 → `AgentPart` 构建

## Capabilities

- `agent-types` — 消息/part/usage/stop_reason 定义
- `event-system` — 生命周期事件扩展

## Impact

- `src/agent/message.rs` — 主类型定义修改
- `src/agent/types.rs` — 删除旧类型，重新导出精简集
- `src/agent/loop.rs` — 历史类型迁移，事件发射
- `src/agent/session.rs` — EventBus 事件适配
- `src/agent/provider/*.rs` — provider 传递 metadata
- `src/agent/model_manifest.rs` — ModelMeta 补齐
- `src/infra/event/lifecycle.rs` — 新增事件变体
- `src/infra/session/*.rs` — 序列化适配
- `tests/` — 测试适配
