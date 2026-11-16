---
depends_on: []
blocks: []
title: 核心显式上下文运行
---

# 核心显式上下文运行

## Why

ThoughtDAG surface 需要把画布编译出的完整 `messages` 作为本轮上下文送入内核，而不是只取最后一条 user 消息。当前 `XyDriver` 只有 `run(prompt)` 与 `run_with_context(McpResourceContext)`；`AgentRuntime::start_root_stream` 固定从 session 加载历史。内核没有“请求上下文”入口，surface 无法在不污染 session 的前提下传入完整对话。

## What Changes

- 在 `XyDriver` 应用协议上增加一个**第一类请求对象** `TurnRequest`，而不是再加一个 `run_with_messages` 方法。
- `TurnRequest` 表达：本轮 user parts、请求上下文（前序 messages）、system prompt 覆盖。
- `XyDriver::run_turn(request)` 默认回退到 `run(prompt_text)`；只有 in-process driver 覆盖。
- `AgentRuntime` 增加 `submit_turn`：有请求上下文时作为种子历史；无上下文时保持从 session 加载。
- 请求上下文只进入本轮历史，不写 session；本轮 user/assistant 仍写 session。
- system 消息经 `XyGenerateOptions.system_prompt`，不得作为 user 落盘。

## Design Choice: TurnRequest, Not Another Method

### 备选方案与取舍

| 方案 | 问题 | 结论 |
|---|---|---|
| 加 `run_with_messages(prompt, messages)` | 方法继续增殖；system/parts 无处放；B/C 还要再加 `run_with_options` | 拒绝 |
| 把上下文写成 session 条目或临时 session | 污染 session；fork/replay 语义变复杂；违背“一画布不强制一 session” | 拒绝 |
| 复用 `McpResourceContext` | MCP 资源语义，不是对话消息；无法表达 assistant/tool result | 拒绝 |
| 用 context-inject loop 中间件 | 需要按请求动态注册；线程/生命周期复杂；surface 无法配置 | 拒绝 |
| **`TurnRequest` 第一类请求对象** | 需要新增公共类型；但一次到位，后续 model/effort/abort 可扩展 | **采用** |

### 为什么不是补丁

- `run(prompt)` 保留为便捷入口；`run_turn` 是唯一的“完整请求”入口。
- `TurnRequest` 用 `#[non_exhaustive]` + 构造器，后续 B/C 可加 `model` / `effort` / `request_id`，不破坏调用方。
- 不引入第二套消息词表：上下文用 `XyLlmMessage`，runtime 内转 `AgentMessage`。
- 不改变 session 真源：session 仍只记录真实 turn；请求上下文是 ephemeral overlay。

## Implementation Points

### 1. TurnRequest 类型

放在 `crates/xylitol-runtime/src/driver/types.rs`（或 `proto.rs` 同层），并由 `xylitol::embed` 再导出。

```rust
#[non_exhaustive]
pub struct TurnRequest {
    pub parts: Vec<xylitol_plugin_sdk::message::AgentPart>,
    pub context: Vec<xylitol_plugin_sdk::message::XyLlmMessage>,
    pub system_prompt: Option<String>,
}

impl TurnRequest {
    pub fn text(prompt: impl Into<String>) -> Self;
    pub fn new(parts: Vec<AgentPart>) -> Self;
    pub fn with_context(self, context: Vec<XyLlmMessage>) -> Self;
    pub fn with_system_prompt(self, system_prompt: Option<String>) -> Self;
    pub fn prompt_text(&self) -> String;
}
```

- `parts`：本轮 user 内容，含图片。
- `context`：本轮之前的历史消息；可为空。
- `system_prompt`：本轮覆盖；`None` 用 agent 配置。
- `prompt_text`：仅用于默认回退，拼接文本 part。

### 2. Driver 契约

`crates/xylitol-runtime/src/driver/proto.rs`：

```rust
async fn run_turn(&mut self, request: TurnRequest) -> EventStream {
    self.run(&request.prompt_text()).await
}
```

- 默认实现忽略 `context` / `system_prompt`，保证所有现有 driver 可用。
- `run` / `run_with_context` 保持不变；不递归。
- `XyInProcessDriver` 覆盖 `run_turn`。

### 3. InProcessDriver

`crates/xylitol-runtime/src/driver/in_process/mod.rs`：

- `run_turn`：
  - 保留 `apply_pending_capability_overlay`、`ensure_tool_table_frozen`、session 绑定。
  - `context` → `Vec<AgentMessage::Llm>`。
  - 调 `agent.submit_turn(request)`。
- 不做 MCP context 的隐式转换；MCP 仍走 `run_with_context`。
- 若 `context` 为空，行为与 `run` 等价。

### 4. AgentRuntime

`crates/xylitol-runtime/src/agent/runtime/react/mod.rs`：

- 新增 `submit_turn(request: TurnRequest)` 与内部 `submit_turn_parts`。
- 与 `submit_parts` 共用准入、turn 创建、`freeze_root_config`。
- `start_root_stream` 增加参数：
  - `seeded_history: Option<Vec<AgentMessage>>`
  - `system_prompt_override: Option<String>`
- 有 seed 时跳过 `load_conversation_history`；无 seed 保持现状。
- `FrozenRootConfig` 增加 `system_prompt_override`；`freeze_root_config` 用覆盖值替代 `inner.system_prompt()`。
- 历史构建：
  - `let mut history = seeded_history.unwrap_or_else(load...)`
  - push 本轮 user parts。
  - 后续 `persist_agent_message` 只写本轮 user/assistant；seed 不写。
- 保持 `r1039`：system 经 `XyGenerateOptions.system_prompt`，不得作为 user 落盘。
- 保持 `r1042`：普通 turn 仍从 session 重读 model/thinking；seeded turn 不改变该机制。

### 5. 转换规则

- `XyLlmMessage::UserMessage` / `AssistantMessage` / `ToolResultMessage` → `AgentMessage::Llm`，字段原样保留。
- 图片由 surface 放进本轮 `parts` 的 `AgentPart::Image`；driver 不猜。
- surface 负责把完整 `messages` 拆成：
  - system → `system_prompt`
  - 最后一条 user → `parts`
  - 其余 → `context`
- 空 `context` / 无 user 由 surface 校验；driver 仍按 `parts` 运行。

### 6. 错误与边界

- seed 含未知 role：surface 转换阶段拒绝。
- seed 含 `ToolResultMessage` 但无对应 tool call：保留，模型按上下文处理；不额外校验。
- seed 长度上限：surface 限制；核心不截断。
- seed 不写 session：session 历史与模型上下文可能不一致；这是显式语义，文档标注。

## Capabilities

- 新增 `agent-run-context` capability：
  - 请求上下文作为本轮种子历史。
  - seed 不落盘。
  - system 覆盖经 generate options。
  - 本轮 user/assistant 落盘。
- 与 `agent-llm-projection` 的 `AgentMessage` 词表一致；不新增平行消息类型。

## Impact

- 兼容：`run_turn` 默认回退，现有 5+ 个 `XyDriver` 实现不受影响。
- 影响面：`proto.rs`、`driver/types.rs`、`in_process/mod.rs`、`react/mod.rs`、`src/embed.rs`；`xtd-core` / `xtd-web` 使用新入口。
- 跨仓：thoughtdag `add-explicit-context-run` 依赖本 change；跨仓依赖不能写进 `depends_on`，需在两边 proposal 互相标注。
- 风险：seed 不落盘导致 session 重放看不到完整上下文；需要在 UI/文档说明。
- 风险：system 覆盖可能丢失 boot 的上下文注入；需明确“覆盖”而非“追加”，并提供追加选项。

## Verification

- 单测：`run_turn` 默认回退；in-process 把 context 传给 `submit_turn`。
- 单测：seed 不写 session；本轮 user/assistant 写 session。
- 单测：system 覆盖进入 `XyGenerateOptions.system_prompt`，不写 user。
- 单测：`TurnRequest::text` 与 `run(prompt)` 等价。
- 集成：mock model 收到 seed + 本轮 prompt；事件流正常。
- 回归：`run(prompt)`、`run_with_context`、`submit_parts` 仍按旧行为。

## Open Questions

- system 覆盖还是追加？建议覆盖，追加作为可选。
- seed 是否允许包含 system 消息？建议 surface 提取为 `system_prompt`，seed 只放 user/assistant/tool。
- `TurnRequest` 是否现在就放 `model` / `effort`？建议不放，留给 `add-per-request-model-effort` 扩展；`#[non_exhaustive]` 保证可加。
- 是否允许 `context` 为空但 `system_prompt` 有值？允许。

## Dependencies

- 无前置。
- 被 thoughtdag `add-explicit-context-run` 依赖。
