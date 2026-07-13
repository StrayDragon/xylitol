---
change_id: c660-fix-agent-tool-abort-process
title: "Abort：工具/bash 进程树可靠取消"
status: purpose-draft
priority: 660
depends_on: []
author: agent
track: B
---

# c660-fix-agent-tool-abort-process

## Why

开箱「动手」质量：用户 Esc abort 后，子进程/工具不应继续跑。`infra-process` 已有 `kill_process_tree`；需确认 **agent 取消令牌 → bash/工具** 全路径可靠，并用 harness 钉住。

## Purpose

Abort / cancel 时：进行中的 bash/工具进程（含子进程）被终止；下一轮 run 不粘滞 cancel（已有 sticky 测则保持）。

## What Changes（实现时）

1. 审计 `XyToolCtx` / bash_exec / react abort 路径；缺口补接线。
2. 合成测：spawn 可挂起子进程 → abort → 进程退出（或可观测 killed）。
3. 文档化 OS 差异与失败模式（见 Ethics）。

## Capabilities

- `infra-process` / `infra-bash` / `agent-runtime` / `agent-tools`（按审计结果 modify）

## Design / 产品语义

- [`docs/architecture/插话续跑与中止.md`](../../../docs/architecture/插话续跑与中止.md)
- [`docs/architecture/工具与权限.md`](../../../docs/architecture/工具与权限.md)
- UI 文案留给 **c665**

## Impact

- agent + infra 取消路径；**不**改 TUI 键位

## Out of scope

- **computer-use**（本会话明确延后）
- TUI status 文案（c665）
- 权限 popup 平台

## Ethics

- risk_level: medium
- prohibited_actions: 无测的 kill；误杀无关进程组
- required_evidence: 至少一条可重复的 abort→进程死 测；注明 Windows/Unix 差异
- escalation_policy: 若某平台无法可靠杀树，在 design/future 写清并保持最佳努力 + 用户可见警告（c665）

## Depends

- 无（可与轨 A 并行）
