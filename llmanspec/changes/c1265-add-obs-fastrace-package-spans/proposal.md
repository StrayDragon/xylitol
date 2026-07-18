---
change_id: c1265-add-obs-fastrace-package-spans
title: 观测：Phase A — inspect 滞后脚本 + agent 低频 span + fastrace-futures
status: full
priority: 1265
depends_on:
- c1250-update-ai-bridge-assistant-stream-events
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: infra
branch: feat/tui-dev
base_sha: 680e83b0baf487deed2bfe8f17a19c96c0c4f975
checkpointed: true
checkpoint_sha: 739398e5bd2523dd4ae3972f8ffa6696b95b4362
---

# c1265-add-obs-fastrace-package-spans

## Why

1. t0718 已证明：用现有 `provider-trace.jsonl`（raw `function_call_arguments.delta` vs mapped `ToolCall*`）可在分钟级判定映射滞后；缺的是 **可复用的 inspect 配方**，不是再堆每条 SSE 的 span。
2. agent 仍无与 `provider.request` 可关联的低频边界；跨 await 的 Stream 若手写 `set_local_parent` 易丢上下文。
3. fastrace 生态里 **`fastrace-futures`**（`StreamExt::in_span`）正好覆盖 provider Stream，减少 DIY；OTel/Jaeger/axum 等面向跨进程导出或 HTTP 框架，与当前「本地 JSONL 排障」不对口，留给后续。
4. 产品 roadmap「出口流量检视」需要进程内可关联时间线作事实源底座——Phase A 先把底座与排障配方钉死，不为检视台提前堆 UI。

## Purpose（Phase A）

1. **skill**：`xylitol-inspect-runtime-logs` 固化 t0718 滞后计算（按 `request_id`）。
2. **依赖**：主 crate / bridge 按需引入 **`fastrace-futures`**；用 `in_span` 包裹 provider 输出 Stream，使 `react.stream` 在每次 poll 保持 local parent。
3. **低频 span**（关闸零成本，复用 `provider_trace_active`）：
   - `react.stream`（每 HTTP 流 1 个；优先经 fastrace-futures）
   - `react.turn`（每 ReAct 迭代 1 个）
   - 可选 `tool.execute {name,id}`
4. 命名稳定；`request_id` 属性关联优先于强行跨 crate 真父子。
5. **MUST** 关闸无昂贵序列化；file-only（ipt3 / dl1）；**禁止** `fastrace-tracing` / 引入 `tracing`。

## Phase B+（本 change 不做，但 design 预留钩子）

| 项 | 说明 | 产品挂钩 |
|---|---|---|
| **opt-in OTel reporter**（`fastrace-opentelemetry` 等） | 与现有 `FileTraceReporter` **并存或可切换**；默认仍 JSONL；禁止默认打 stdout | 为「出口流量检视」/ Web Inspect 导出标准时间线做准备 |
| `fastrace-reqwest` | 仅当需要 W3C `traceparent` 出站传播时 | 跨服务协同（非模型厂商回传） |
| TUI / engine 采样 span | 另开 change | 视觉排障，非检视事实源主路径 |
| `adapter.map_event` | 与 raw/mapped Event 重复 | 不做 |
| live model CI | 另开 | — |

## What Changes

- Cargo：`fastrace-futures`（版本与 `fastrace` 对齐）
- skill：滞后脚本
- agent + bridge stream 接线：`in_span` + 低频 span
- live specs：`infra-provider-trace` 增补跨层/零成本；可选一句「未来可并列 OTel reporter」文档意图（非本 change MUST）
- design 记清：自定义 JSONL schema **保留**（OTel 不能替代 t0718 对照）

## Capabilities

- `infra-provider-trace`（modify）
- 可能触及 `infra-logging`（文档级）

## Out of scope

- Phase B+ 表中各项的实现
- XML 抽取器；默认 CI 付费模型；ConsoleReporter

## Ethics

- risk_level: low
- prohibited_actions: span/log 写 Authorization；stdout/stderr 毁 TUI；release 默认昂贵序列化；默认启用外部 OTel 出口；引入 `tracing`
- required_evidence: 关闸零成本；开闸 skill 可算 lag；`react.stream` 经 futures 集成可见关联
- escalation_policy: span 不可见 JSONL → lifecycle Event；延迟可测 → 砍 span 留 skill；OTel 与 JSONL 冲突 → JSONL 优先

## Depends

- `c1250`（已归档）
- 产品前瞻：`docs/roadmaps/出口流量检视.md`（进程内时间线 → Inspect 事实源；OTel 为可选导出）
