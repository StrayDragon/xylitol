---
change_id: c1265-add-obs-fastrace-package-spans
title: "观测：Phase A — inspect 滞后脚本 + agent 低频 span"
status: purpose-draft
priority: 1265
depends_on:
  - c1250-update-ai-bridge-assistant-stream-events
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: infra
branch: feat/tui-dev
---

# c1265-add-obs-fastrace-package-spans

## Why

1. t0718 已证明：用现有 `provider-trace.jsonl`（raw `function_call_arguments.delta` vs mapped `ToolCall*`）可在分钟级判定映射滞后；缺的是 **可复用的 inspect 配方**，不是再堆每条 SSE 的 span。
2. agent / app-tui 仍无与 `provider.request` 可关联的低频边界，跨层 timing 只能猜。
3. 原草稿把 `adapter.map_event`、TUI 60Hz、`engine.frame`、live CI 捆在一起——热路径风险高、与 t0718 价值不对齐。

## Purpose（Phase A — 收窄）

1. **文档/skill**：在 `xylitol-inspect-runtime-logs` 固化 t0718 滞后计算（按 `request_id`：首个 args delta → 首个 mapped ToolCall*）。
2. **低频 agent span**（关闸零成本，复用 `provider_trace_active` 或等价闸）：
   - `react.stream`（每次 provider HTTP 流 1 个）
   - `react.turn`（每次 ReAct 迭代 1 个）
   - 可选 `tool.execute {name,id}`（仅工具执行）
3. 命名稳定；尽量挂 `request_id` / 父子关系（若跨 crate 根 span 限制，至少属性关联）。
4. **MUST** 证明闸关闭时无昂贵序列化；file-only（ipt3 / dl1）。

## Explicitly deferred（Phase B+，本 change 不做）

- `adapter.map_event`（与现有 raw/mapped Event 重复；热路径）
- `tui.host.event` / `tui.bridge.apply` / `xylitol-tui` `engine.frame`
- `XYLITOL_LIVE_MODEL` 真机剧本 / CI 断言
- 引入 `tracing` crate

## What Changes

- `.agents/skills/xylitol-inspect-runtime-logs/SKILL.md`（+ `.claude` 镜像若存在）：滞后脚本片段
- `src/agent/runtime`（及必要）：低频 span + 关闸
- 可选：`FileTraceReporter` 对「无 event 的 span」是否写一行起止——仅当 Phase A agent span 否则不可见时再动
- live specs：`infra-provider-trace`（ipt1/ipt2 增补跨层/零成本）和/或 `infra-logging` 指针；**不**新建大 capability 除非必要

## Capabilities

- `infra-provider-trace`（modify）
- 可能触及 `infra-logging`（文档级）

## Out of scope

- 见「Explicitly deferred」
- XML 抽取器；默认 CI 付费模型

## Ethics

- risk_level: low
- prohibited_actions: span/log 写 Authorization；stdout/stderr 毁 TUI；release 默认昂贵序列化；热路径无采样 span
- required_evidence: 关闸零/近零开销；开闸一次请求可见 skill 滞后计算 + agent 低频 span（或属性关联）
- escalation_policy: 若 agent span 不可见于 JSONL → 最小扩展 reporter 或加边界 Event；若延迟可测 → 砍 span 只留 skill

## Depends

- `c1250-update-ai-bridge-assistant-stream-events`（已归档；事件名与 mapped 形状）
