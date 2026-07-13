---
change_id: c660-fix-agent-tool-abort-process
title: "Abort：工具/bash 进程树可靠取消"
status: full
priority: 660
depends_on: []
author: agent
track: B
---

# c660-fix-agent-tool-abort-process

## Why

开箱「动手」质量：用户 Esc abort 后，子进程/工具不应继续跑。`infra-process` 已有 `kill_process_tree`；工具 bash 已监听 run `CancellationToken`；缺口是 **交互 `!`/`!!` bash** 的 cancel 只挂在 `abort_bash`，而 `Driver`/`AgentRuntime::abort` 未调用它（对齐 pi：`agent.abort()` + `session.abortBash()`）。

## Purpose

Abort / cancel 时：

1. 进行中的 **ReAct 工具**（含 bash 工具）经当前 run `CancellationToken` 停止并杀进程树（既有路径保持）。
2. 进行中的 **交互 bash**（`execute_bash` / `!`）经同一 `abort` 入口取消并杀进程树。
3. 下一轮 `run` 不粘滞 cancel（既有 sticky 测保持）。

## What Changes

1. `BashExecHandler::abort` 可从 `&self` 调用（与 steer 队列同为 interior mutability）。
2. `AgentRuntime::abort` MUST 在取消 run token + 清 steer 之外调用 `abort_bash`。
3. 合成测：挂起 `sleep` → `abort` → `cancelled` / 进程退出。
4. `design.md` 记录 Unix/Windows 差异与失败模式。

## Capabilities

- `agent-runtime`（modify：abort 联动交互 bash）
- `infra-bash`（modify：abort 可达性 / 杀树证据场景）

## Design / 产品语义

- [`docs/architecture/插话续跑与中止.md`](../../../docs/architecture/插话续跑与中止.md)
- 本 change `design.md`
- UI 文案留给 **c665**

## Impact

- `src/agent/session/bash.rs`、`session/mod.rs`、`runtime/react.rs`；`Driver::abort` 经 `AgentRuntime::abort` 自动继承
- **不**改 TUI 键位 / status 文案

## Out of scope

- **computer-use**
- TUI abort 反馈（c665）
- 权限 popup 平台
- 远程 Driver 另开协议字段（server 已 `DELETE` → `driver.abort()`）

## Ethics

- risk_level: medium
- prohibited_actions: 无测的 kill；误杀无关进程组
- required_evidence: 至少一条可重复的 abort→交互 bash cancelled/杀树测；注明 OS 差异
- escalation_policy: 若某平台无法可靠杀树，在 design/future 写清并保持最佳努力（c665 可见警告）
