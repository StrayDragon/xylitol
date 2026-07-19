---
change_id: c1420-align-compact-estimate-gate-local-tokenizer
title: Compact 与 TUI 同源计量；LocalTokenizer 仅 on/off 且默认 off
status: full
priority: 1420
depends_on: []
author: agent
track: B
wave: token-estimate-align
domain: agent
apply_band: P0
branch: feat/c1420-align-compact-estimate-gate-local-tokenizer
base_sha: 2e6efd061cdb8087514f6ca0533c02d8f78dbcc5
checkpointed: false
---

# c1420-align-compact-estimate-gate-local-tokenizer

> attach 默认相对 `origin/main`（落后本地 main）；`base_sha` 已改为与本地 `main` 的 merge-base，避免把 perf footer 提交算进本 change。

> 明确不做：跨进程 IPC / Single-flight；every-N / idle。usage 落盘 **留在本 change**。

## Why

1. **计量分叉**：TUI footer 走 paa1；`maybe_auto_compact` 仍用 `len/4`。
2. **LocalTokenizer 默认太贵**：有映射就 encode；应仅 `on|off`，默认 `off`。
3. **Api 锚点断链**：端点有 usage，但 ReAct 落盘硬编码 `usage: None`。

## Purpose

1. ReAct 持久化 `Done.usage` / `stop_reason`。
2. Compact 阈值与 footer 同源估计（paa1 + local 闸）。
3. `token_estimate.local_tokenizer: on|off`，默认 `off`。

## What Changes

- `react.rs`：捕获 Done → AssistantMessage
- `EstimateOpts.allow_local_tokenizer` + AppConfig 键
- `maybe_auto_compact` 改用共享估计
- Live specs：`agent-runtime` ar23、`domain-compaction` c2/c16、`package-ai-bridge-accounting` paa10、`runtime-config` rc19

## Capabilities

- `agent-runtime`（modify）
- `domain-compaction`（modify）
- `package-ai-bridge-accounting`（modify）
- `runtime-config`（modify）

## Impact

- 有 usage 后 footer/compact 可走 Api；默认少 HF encode。
- Compact 触发时机相对今日 len/4 可能变化。

## Out of scope

- Single-flight / IPC；every-N / idle；TUI 下载确认；RemoteCount 默认策略
