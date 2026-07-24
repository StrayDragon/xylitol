---
change_id: c1590-fix-otel-generation-request-input
title: 补齐 llm.request generation 的请求体 observation input
status: purpose-draft
priority: 1590
depends_on:
  - c1485-add-otel-usage-io
  - c1555-add-otel-turn-input-preview
author: agent
---

# c1590-fix-otel-generation-request-input

> **purpose-draft**：仅候补意向；**不** attach / **不** apply，除非用户明确升格。

## Why

c1550 / c1555 后 Langfuse Session 已有：

- `agent.turn` 根：用户提示预览（随 `observation_io`）
- `tool.execute`：参数/结果（随 `tool_observation_io`）
- `llm.request`：多数有 **output** 与 usage

但实测（如 session `2c113306-…`）大量 `llm.request` 的 **input 仍空**。排障「这一轮模型实际吃进什么请求」仍要回本地 `provider-trace.jsonl`。

根因（c1485 既有策略）：generation input 依赖 raw 事件缓冲到 `response.json` / `chat.completion.json` / `message.json` 才写入 `langfuse.observation.input`；Responses 等路径若未以该事件名吐出请求体，则 **output-only**。

## What Changes（意向）

1. 在 `[otel].observation_io` ≠ none 时，保证 `llm.request` **尽量**带上请求侧 input（与现档位截断硬顶一致）
2. 优先复用已有请求装配事实（adapter 发出的 body / messages 摘要），**不**依赖偶然的 raw 事件名
3. 缺请求体时仍 MUST NOT 伪造空 usage；input 可缺席但应可诊断（文档或属性说明 gap）
4. **不**改变默认 `observation_io=none`；**不**与 `tool_observation_io` 合并

## Out of scope

- tool I/O（已 c1550）
- turn 根用户预览（已 c1555）
- `token.estimate` 双 root
- 默认开启全文上云

## Capabilities（升格时）

- `infra-otel`（修订或新 req：generation request input 完备性）

## Ethics

- risk_level: medium（请求体可能含路径/密钥片段）
- prohibited_actions: 默认 full；绕过 `observation_io` 闸
- required_evidence: none 单测；truncated 硬顶；至少一条 Responses 路径带 input 的验收
- escalation_policy: 升格前确认 input 形状（全文 JSON vs messages 摘要）
