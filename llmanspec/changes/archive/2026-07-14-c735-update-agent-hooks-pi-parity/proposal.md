---
change_id: c735-update-agent-hooks-pi-parity
title: "Hook 对齐 pi：provider 三缝 + 生命周期脚本桥 + 修假合约"
status: full
priority: 735
depends_on: []
author: agent
track: A
wave: hooks-pi-parity
---

# c735-update-agent-hooks-pi-parity

## Why

`agent-hooks` 合约与 BDD 宣称 `after_provider_*`，但 `HookEvent` 无变体、热路径不调用 `HookDispatcher`；命名与 pi `before_provider_request` 冲突。真实扩展点仅 `AgentHooks`（tool/context）。要对齐 pi extension 事件面并接通脚本即插即用。

## Purpose

1. **Wave 0**：在 Responses/Anthropic adapter 落地 pi 三缝：`before_provider_headers`、`before_provider_request`、`after_provider_response`（仅 status+headers，流消费前）；修 spec 命名；空配置零开销。
2. **Wave 1**：ReAct 在 `AgentHooks` 之外桥接脚本 `tool_call` / `tool_result` / `context`（及可选 `tool_execution_*` 观察）。
3. **Wave 2**：在现有 `XyEvent` 发射点旁挂脚本：`agent_*`、`turn_*`、`message_*`、`model_select` / `thinking_level_select`（有则接）。
4. **Wave 3**：session 操作点挂 `session_start` / `session_shutdown` / compact / fork / tree / switch（能接到的 MUST 接；payload 用本仓 id）。
5. **Completions adapter**：本 change MUST 接受 hooks 句柄但 HTTP 三缝可为 no-op（文档化）；默认 OpenAI 走 Responses。
6. **W4/W5**（trust 桥、`input`、`resources_discover`、`ctx.ui`）→ `future.md`，不挡归档。
7. BDD provider 场景 MUST 非空 stub；`just` 相关测试绿。

## What Changes

1. `HookEvent` 扩展 + `event_matches` 支持 pi 全名。
2. `composition`/`bootstrap` 建 `Arc<HookDispatcher>`，注入 model factory 与 agent。
3. adapter HTTP helper：headers → body → send → after_response。
4. `react` / session / Driver 关键点 dispatch。
5. 修订 `agent-hooks` main spec（h1/h7 命名）。
6. 填 BDD；单测覆盖 modify body / block / empty noop。

## Capabilities

- `agent-hooks`（modify）

## Out of scope

- Track B 抓包（`c999`）
- 成功 SSE body 进 hook
- Completions 完整 HTTP 三缝
- W4/W5（见 future.md）
- pi `ctx.ui` / custom tool render

## Ethics

- risk_level: medium
- prohibited_actions: 默认把 Authorization 明文写入 hook 日志文件；默认开启会改写所有请求的 hook
- required_evidence: 单测 + hooks.feature BDD 非空实现通过
- escalation_policy: 与 pi 语义冲突时以本仓可测行为为准并记 design

## Depends

- []
