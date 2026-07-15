---
change_id: c1000-add-infra-provider-trace
title: "观测栈迁 fastrace + 进程内 provider raw/mapped trace"
status: full
priority: 1000
depends_on: []
author: agent
track: B
wave: provider-observability
domain: infra
---

# c1000-add-infra-provider-trace

## Why

1. 需要 **agent 可读** 的 raw SSE ↔ mapped `XyChunk` 对照，区分上游通道混写 vs 适配器映射 bug。
2. 观测应是**基础设施**：高性能、关闭近零开销；**禁止** `tracing` + `fastrace` 双栈。
3. 外挂 MITM（已暂停 c999）不满足 agent 分析。

## Purpose

1. **迁移**：移除 `tracing` / `tracing-subscriber`；采用 **fastrace**（timeline）+ **`log`**（级别日志，挂到当前 span）。组合根只装 **file-only** Reporter/logger（永不 stdout/stderr）。
2. **Provider trace**：每次 provider 流用 fastrace root/`request_id`；raw 协议事件与 mapped `XyChunk` 记为 **Event**（同 trace）；落盘 `provider-trace.jsonl`（或等价），供 agent 对照。
3. **闸门**：`cfg(debug_assertions)` 默认安装 Reporter；release 默认不装，经 `XYLITOL_PROVIDER_TRACE` / `XYLITOL_DEBUG`（级别日志）显式打开。关闭时不设 Reporter / 用 noop，发射点不昂贵序列化。
4. **安全**：Authorization / API-key 头 MUST NOT 入 dump；不经 script hook。

## What Changes

- `Cargo.toml`：加 `fastrace`（应用 `enable`）、`log` + 文件 logger；去掉 tracing 栈
- `app/cli/logging.rs` → 组合根装 fastrace Reporter + log 文件后端
- 全仓 `tracing::` ≈47 处 → `log::`（或等价）
- `infra/provider/trace` + 三适配器接线
- delta：`infra-provider-trace`（新）+ **modify** `infra-logging`（合约从 tracing 改为 fastrace+log）

## Capabilities

- `infra-provider-trace`
- `infra-logging`（modify）

## Out of scope

- 外挂 mitm/claude-tap；OTel/Jaeger 默认导出（fastrace 生态可后置）
- 保留任何 `tracing` / `tracing-subscriber` 依赖
- SSE 进 hook / session JSONL

## Ethics

- risk_level: medium
- prohibited_actions: release 默认全开；密钥入 dump；ConsoleReporter/stderr；双栈残留 tracing
- required_evidence: `rg tracing` 于 src/Cargo 为零（测试允许临时除外）；debug 有对照 JSONL；release 默认无
- escalation_policy: Reporter 落盘格式或截断策略有歧义时先收紧

## Depends

- []
