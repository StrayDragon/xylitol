---
change_id: c1475-add-otel-export
title: 可选 OTLP/HTTP 出口（feature otel + 默认 none）
status: in-progress
priority: 1475
depends_on: []
author: agent
branch: feature/c1475-add-otel-export
base_sha: c8fa127776ea9cd8bb8ddcc23dfd7c9661721249
checkpointed: false
---

# c1475-add-otel-export

## Why

1. 产品已决定以 **OpenTelemetry → Langfuse** 作为首要可观测出口，取代自研 Inspect 检视台主路径。
2. 现有 fastrace + `provider-trace.jsonl` 只能窄读；需要标准 OTLP 才能进 Langfuse / Collector / Jaeger。
3. 用户本地 Langfuse 尚未就绪：装配必须 **默认不收集**，坏配置 **自动降级**，不挡对话。
4. 架构硬约束：时间线栈 **仅 fastrace**（禁止 tracing 双栈）；Codex 的 `tracing-opentelemetry` 路径不可照搬。

## What Changes

- Cargo feature **`otel`**：闸住 `fastrace-opentelemetry` / `opentelemetry*` / `opentelemetry-otlp`（HTTP）依赖
- `AppConfig` 增加可选 **`[otel]`**（`exporter = none | otlp-http`、endpoint、headers/凭证引用、protocol、environment、service_name）
- 组合根装配 **fan-out Reporter**：本地 FileReporter（仍由既有 env/build 闸）± 可选 `OpenTelemetryReporter`
- **Fallback**：缺配 / `none` / 凭证不全 / exporter 构建失败 → 不装 OTLP，warn 进文件 log，进程继续
- OTLP **MUST NOT** 写 stdout/stderr（TUI 安全）
- 文档：删除 Inspect roadmap；新增 [OTEL与Langfuse观测.md](../../../docs/roadmaps/OTEL与Langfuse观测.md)；更新 architecture 进程内观测

## Out of scope

- GenAI / `langfuse.*` 属性映射（→ `c1480-add-otel-genai-langfuse`）
- 自研 Inspect UI / TUI 只起检视页
- OTLP gRPC（Langfuse 不支持；本刀只 HTTP）
- Metrics / Logs 柱（本刀仅 traces）
- 敏感全文默认上传

## Capabilities

- `infra-otel`（新）
- `runtime-config`（`rc24`）
- `infra-provider-trace`（澄清 opt-in 出口与 `no-default-otel` 并存）

## Impact

- 用户：配置开启后可把既有 fastrace span 送到 OTLP；默认零出口流量
- 二进制：未开 `otel` feature 时不拉 OTEL 依赖（或等价零装配）
- 风险：medium（远程出口与凭证）；默认 off + 降级缓解

## Ethics

- risk_level: medium
- prohibited_actions: 默认开启 OTLP；把 Authorization/API key 写入 span 属性；引入 tracing 双栈；因 OTEL 失败阻断主路径
- required_evidence: 默认 none 单测/场景；坏配置降级；feature off 路径；文档已替换 Inspect 轨
- escalation_policy: 若需默认采样或全文 raw 上传 → STOP，另开 change
