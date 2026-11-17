---
depends_on: []
blocks: []
title: 工具门轮询改纯被动视观察
needs_specs_change: false
---

# 工具门轮询改纯被动视观察

## Why

c3345 引入的 `poll_tool_gate` 用 `hub_wait(100ms)` 实现分段等待。`McpPrepareHub::wait` 的语义是 `timeout(window, self.ensure())`——`ensure()` 是**整条 MCP prepare 管线**(连接 / list tools / 构建 adapter)。后果:宿主后台 50ms 轮询把 ensure 重跑上百次,连接被打断重建,tray 在 connecting↔connected 间抖动;且每次窗口超时都会写入 `SESSION_PREPARE_TIMEOUT_NOTICE`,污染端侧 cue 与 Assembling 呈现。

## What Changes

- `poll_tool_gate` 改纯被动:每轮只读 `hub.view()`(`configured == 0 || complete` 即 settle),**不得**调用 `hub.wait` / `ensure`;门限耗尽才一次性回落 `armed_result` 并写一次门限通知。
- bootstrap 本身由 materialize 时的 `begin_session_prepare`(`hub.fire()`)自启,被动轮询只观察进度,不驱动。
- 无合约变更(`server-core` r1923 的「不持锁等待、有界收口、下行回推」契约不变);`needs_specs_change: false`。

## Impact

- crates:`xylitol-runtime`(`prepare.rs` 轮询实现 + 单测:计数 hub 断言零 ensure/wait 调用)。
- 复验:tmux 真机复现(准备期提交),tray 不抖、cue 常驻、Assembling 正常、冻结后自动上行。
