---
change_id: c680-fix-provider-stream-abort
title: "Abort：中断模型 HTTP 流（停计费）"
status: full
priority: 680
depends_on: []
author: agent
track: B
---

# c680-fix-provider-stream-abort

## Why

c670 让 Esc abort 后 UI 立刻 idle，但 **provider SSE/HTTP 仍会 drain 完**——ReAct 只在 turn 入口查 `CancellationToken`，chunk 环与 `generate_stream`/`send()` 无 cancel 竞态。开箱原则：**abort 必须切断计费路径**，不能只静音事件。工具/bash 已有 cancel（c660）；模型流缺口是编码，非 reqwest 不能取消（drop Response/`bytes_stream` 即 abort）。

## Purpose

1. 用户 / 任意 client 调用 `Driver::abort`（或 `AgentRuntime::abort`）时，进行中的 **模型流** MUST 停止 poll，并 **drop** provider stream，从而关闭 HTTP body。
2. 挂起的 `call_with_retry` / `send()` MUST 可被同一 token 取消。
3. TUI / CLI / server / 未来 client **无感继承**（统一走 Driver/Runtime abort，不各面各写一套）。
4. 证据测固定：底层 drop 断连 + ReAct 中途停 poll + BDD 经 Driver。

## What Changes

1. ReAct：`select!` 竞态 `cancel.cancelled()` vs `chunk_stream.next()` / `call_with_retry`；abort 时 `drop(chunk_stream)` 并 `Error("aborted")`。
2. 不强制改 `XyModel` trait（第一拍够用；可选后续下沉 cancel）。
3. 测试：
   - `tests/provider_http_stream_abort.rs`：reqwest drop + Anthropic adapter drop → 服务端写失败。
   - `abort_mid_stream_stops_polling_model_chunks` 单测。
   - BDD：`经 Driver … abort` → `aborted` 且 TextDelta 远少于脚本总量。

## Capabilities

- `agent-runtime`（add：mid-stream provider abort）

## Impact

- `src/agent/runtime/react.rs`（主修）
- `src/infra/provider/fake.rs` / `factory.rs`（BDD slow stream）
- `tests/provider_http_stream_abort.rs`、`tests/features/agent.feature`、`tests/bdd.rs`
- 所有经 `Driver::abort` 的面自动继承

## Out of scope

- TUI 文案 / c670 UI 闸（已有）
- 改 `XyModel` 签名把 cancel 下沉到每个 adapter（可选 follow-up）
- 保证厂商对「已生成未送达」token 不计费（只能尽早 RST）

## Ethics

- risk_level: medium
- prohibited_actions: 仅 UI abort 而无 HTTP 证据
- required_evidence: drop 断连 PoC + mid-stream 停 poll + Driver BDD
- escalation_policy: 若某 adapter（如 async-openai）drop 不断连，再对症改 adapter 或 trait
