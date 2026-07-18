---
change_id: c1255-update-agent-tool-intent-lifecycle
title: "agent：工具意图流式与执行生命周期拆分（对齐 pi agent-loop）"
status: purpose-draft
priority: 1255
depends_on:
  - c1250-update-ai-bridge-assistant-stream-events
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: agent
---

# c1255-update-agent-tool-intent-lifecycle

## Why

1. 今日 `react.rs` 在收到完整 `XyChunk::FunctionCall` 时立刻 `yield ToolExecutionStart`，把 **「模型仍在生成工具参数」** 与 **「开始执行工具」** 绑死。
2. pi `agent-loop`：`toolcall_*` 只驱动 `message_update`；`message_end` 之后才 `tool_execution_start|update|end`。
3. c1250 提供增量事件后，若 agent 仍「攒齐再一次性 Start」，TUI 无法在首 chunk 展示意图；且 `ToolExecutionUpdate` 仍接近死路径（与 `agent-session` 流式预期不一致）。

## Purpose

ReAct 消费 c1250 流式块事件，固定行为：

1. 流式阶段：维护 partial assistant message；对外发射可观察的 **意图/消息更新**（含 tool 块渐进 args）；
2. 流结束（`done`/`toolUse`）后：才进入工具执行；发射 `ToolExecutionStart` →（可）`ToolExecutionUpdate` → `ToolExecutionEnd`；
3. 同一 assistant 回合内多个 toolCall 的批执行语义保持 ar3；
4. session 落盘仍为结构化 `AgentPart::{Text,Thinking,ToolCall}`，**不**把工具参数写入 Text 部分。

## What Changes

- `src/agent/runtime/react.rs` 流消费循环
- `XyEvent`：明确意图更新 vs 执行事件（可扩展变体或强化现有 `MessageUpdate` + 保留 ToolExecution*）
- wire `protocol::Event` 映射（注意今日 `ToolStart` 丢 args 的缺口，按需修）
- 激活工具执行期的 `ToolExecutionUpdate`（至少 bash / 长输出工具）
- BDD：意图事件先于执行；tool-stream 场景与实现一致

## Capabilities

- `agent-runtime`（modify）
- `agent-session`（modify，若 tool-stream 场景归属此处）
- `protocol-*`（modify，若事件形状变）

## Out of scope

- TUI 组件拆分与人类可读摘要 → **c1260**
- fastrace 包级 span → **c1265**
- XML 伪工具抽取
- 改变 Trust / allow-all 工具策略

## Ethics

- risk_level: medium
- prohibited_actions: 在流未结束时执行工具副作用；把 tool 参数双写进 Text part；为兼容保留错误的「FunctionCall→立刻执行」旁路
- required_evidence: 单测/BDD 证明 toolcall_delta 到达时已有可观察意图更新且尚未 ToolExecutionStart；执行期有 Update 或文档化的例外列表
- escalation_policy: 若 print/CLI 面依赖「Start=立即执行」语义，升级确认迁移顺序

## Depends

- `c1250-update-ai-bridge-assistant-stream-events`（须先归档或本分支已落地其事件）
