---
change_id: c1255-update-agent-tool-intent-lifecycle
title: agent：工具意图流式与执行生命周期拆分（对齐 pi agent-loop）
status: full
priority: 1255
depends_on:
- c1250-update-ai-bridge-assistant-stream-events
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: agent
branch: feat/tui-dev
base_sha: 680e83b0baf487deed2bfe8f17a19c96c0c4f975
checkpointed: true
checkpoint_sha: fadf2a07dbd988513305aa3a0298a65a3fda3b3b
---

# c1255-update-agent-tool-intent-lifecycle

## Why

1. 今日 `react.rs` 在收到完整 `XyChunk::ToolCallEnd` 时立刻 `yield ToolExecutionStart`，把 **「模型仍在生成工具参数」** 与 **「开始执行工具」** 绑死。
2. pi `agent-loop`：`toolcall_*` 只驱动 `message_update`；`message_end` 之后才 `tool_execution_start|update|end`。
3. c1250 提供增量事件后，若 agent 仍「攒齐再一次性 Start」，TUI 无法在首 chunk 展示意图；且 `ToolExecutionUpdate` 仍接近死路径（与 `agent-session` 流式预期不一致）。

## Purpose

ReAct 消费 c1250 流式块事件，固定行为：

1. 流式阶段：维护 partial assistant message；对外发射可观察的 **意图/消息更新**（含 tool 块渐进 args）；
2. 流结束（`MessageEnd`）后：才进入工具执行；发射 `ToolExecutionStart` →（可）`ToolExecutionUpdate` → `ToolExecutionEnd`；
3. 同一 assistant 回合内多个 toolCall 的批执行语义保持 ar3；
4. session 落盘仍为结构化 `AgentPart::{Text,Thinking,ToolCall}`，**不**把工具参数写入 Text 部分。

## What Changes

- `src/agent/runtime/react.rs` 流消费循环
- `XyEvent`：意图 ⊂ `MessageUpdate.message`；`ToolExecution*` 仅执行路径
- 激活工具执行期的 `ToolExecutionUpdate`（至少一次；长输出后续可多段）
- live specs：`agent-runtime` ar21 + `intent-before-execution`；`agent-session` a3 收紧
- BDD / 单测：意图事件先于执行

## Capabilities

- `agent-runtime`（modify）
- `agent-session`（modify）

## Out of scope

- TUI 组件拆分与人类可读摘要 → **c1260**
- fastrace 包级 span → **c1265**
- XML 伪工具抽取
- 改变 Trust / allow-all 工具策略
- wire `Event::ToolStart` 增加 args（远程面按需另开）

## Ethics

- risk_level: medium
- prohibited_actions: 在流未结束时执行工具副作用；把 tool 参数双写进 Text part；为兼容保留错误的「End→立刻 Start」旁路
- required_evidence: 单测/BDD 证明 toolcall_delta 到达时已有可观察意图更新且尚未 ToolExecutionStart；执行期有 Update
- escalation_policy: 若 print/CLI 面依赖「Start=立即执行」语义，升级确认迁移顺序

## Depends

- `c1250-update-ai-bridge-assistant-stream-events`（已归档；本分支已落地其事件）
