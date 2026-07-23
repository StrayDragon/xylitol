---
change_id: c1540-add-otel-tool-observation-io
title: Langfuse tool.execute 可选 observation I/O 档
status: purpose-draft
priority: 1540
depends_on:
  - c1495-add-otel-session-span-tree
author: agent
---

# c1540-add-otel-tool-observation-io

> **⚠️ purpose-draft（`llmanspec/do-not-read-me/`）**
> 仅记意向；**不** promote / **不** apply，除非用户明确升格。
> 与 generation 的 `[otel].observation_io` 平行：默认 none，显式档才上工具参数/结果。

## Why

c1495 后 Langfuse 能看到 `tool.execute` 树节点（name / tool_name），但 **input/output 恒空**。排障「调了什么参数、返回了什么」仍要回 TUI 或本地 JSONL。

用户期望：可选地把工具 I/O 挂到 observation 上，且**默认不上云**（路径、密钥、大段文件内容风险）。

## What Changes（意向）

1. 配置：`[otel].tool_observation_io = none | truncated | full`（默认 **none**）
   - 与现有 `observation_io`（只管 `llm.request`）**拆开**——LLM 与 tool 敏感度不同。
2. `none`：MUST NOT 写 `langfuse.observation.input` / `output` 到 `tool.execute`。
3. `truncated` / `full`：在 tool span 结束前写入截断/尽量完整的参数 JSON 与结果摘要（仍有硬顶，防 OTLP 爆包）。
4. 装配：组合根 → bridge/agent 静态闸（对齐 `observation_io` / `provider_trace_active`）；agent ↛ infra。
5. **不**改变 OTLP 默认不出口；**不**把 SSE delta events 问题塞进本 change（见主线 OTLP 降体积）。

## Out of scope

- 默认开启 tool I/O
- 修 OTLP per-delta 体积 / network error（另切片）
- `token.estimate` 双 root（另议；先不处理）

## Capabilities（升格时）

- `infra-otel`（新 req：tool observation I/O 档）

## Ethics

- risk_level: medium（敏感载荷上云）
- prohibited_actions: 默认 full；无截断硬顶的 full
- required_evidence: 默认 none 单测；truncated 有上限；validate
- escalation_policy: 升格前确认与 `observation_io` 拆配置（非合并）仍是产品偏好
