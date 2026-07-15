---
change_id: c1000-add-infra-provider-trace
title: "进程内 Provider raw+mapped trace（debug 默认 / release 开关）"
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

会话 JSONL / `XyChunk` 是归一化结果，无法证明「网关推了 `reasoning_text` 还是 `output_text`」。外挂 MITM/claude-tap（已暂停的 c999）对 **agent 分析不友好**。需要进程内、可落盘、可对照的 **raw SSE ↔ mapped chunk** 基础设施，以便区分 provider 错分 vs 适配器映射 bug。

## Purpose

1. 在 provider 适配器热路径旁挂 **可移植、client-agnostic** 的 trace 发射点：同一 `request_id` 下记录 raw 事件与映射后 `XyChunk`。
2. 落盘为 **agent 可读 JSONL**（独立文件，不毁 TUI）；**不**把成功 SSE 默认塞进 script hook。
3. 闸门对齐现有 logging：`cfg(debug_assertions)` **默认开**；release **默认关**，经 `XYLITOL_PROVIDER_TRACE`（及/或 `RUST_LOG` target）显式打开。
4. 性能：关闭时近零开销（`tracing` callsite + `enabled!` 门闩；昂贵序列化仅在启用时）；高流量不污染 `xylitol.log`。
5. 安全：Authorization / `x-api-key` 等密钥头 MUST NOT 进入 raw dump。

## What Changes

- 新 capability `infra-provider-trace`（ports/helpers + 落盘 Layer）
- 适配器（Responses / Anthropic / Completions）在 SSE 解析与 `XyChunk` yield 处发射
- `app/cli/logging` 组合根装配专用 Layer / 文件（或扩展 init）
- 单测：gate off 无分配副作用可观测；gate on 同 request 有 raw+mapped 行

## Capabilities

- `infra-provider-trace`（新建）
- 可能触碰 `infra-logging` 装配（组合根）；合约以本 change delta 为准

## Out of scope

- 外挂 mitmproxy / claude-tap（见 `do-not-read-me/c999-…`）
- OpenTelemetry / Jaeger 导出
- 把 SSE body 写入 session JSONL 或 hook
- 引入第二套 timeline 库（fastrace）作为默认路径

## Ethics

- risk_level: medium（日志含 prompt；密钥禁入）
- prohibited_actions: 默认在 release 开启；明文 API key 入 dump；stdout/stderr 写 trace
- required_evidence: debug 默认有 JSONL；release 默认无；同 request_id 对照 raw vs ThinkingDelta/TextDelta
- escalation_policy: 体积/隐私策略有歧义时先收紧 redact+截断再扩

## Depends

- 无硬依赖（建立在已归档 logging / provider 之上）

## Research summary（设计依据）

见同目录 [`design.md`](./design.md)。
