---
depends_on: []
---

# c25-add-agent-loop

## Why

Agent 执行循环是核心——接收用户 prompt、调用 LLM、解析工具调用请求、执行工具、将结果返回 LLM、重复直到任务完成。这是整个系统的"心脏"。

## What Changes

1. 在 `src/agent/loop.rs` 实现 agent 执行循环（基于 adk-core/adk-agent/adk-runner）
2. 集成 adk-model 的 LLM Provider（仅 OpenAI-compatible Response API + Anthropic-compatible 两种模式）
3. 工具调用分派（调用 `ToolRegistry`）
4. 事件系统（`AgentEvent` 枚举 + 事件发射）
5. Session runtime（基于 adk-session SQLite 后端）

### Agent 循环流程

```
用户 prompt → 构建上下文 → 调用 LLM（流式）→ 解析响应
  → 文本输出 → 发射事件
  → 工具调用 → 执行工具 → 将结果加入上下文 → 重新调用 LLM
  → 重复直到 LLM 返回最终响应（无工具调用）
```

### 事件类型

- `TextDelta` — 流式文本输出
- `ToolCallStart` — 工具调用开始
- `ToolCallEnd` — 工具调用完成（含结果）
- `StepComplete` — 单步完成
- `Error` — 错误事件

## Capabilities

- `agent-loop`: agent 执行循环 + 工具分派 + 事件系统 + session runtime

## Impact

- 新增 `adk-core`, `adk-agent`, `adk-runner`, `adk-model`, `adk-session` 依赖
- `src/agent/loop.rs` 和 `src/agent/model.rs` 从占位变为实际实现
- 这是整个项目最核心的 change
