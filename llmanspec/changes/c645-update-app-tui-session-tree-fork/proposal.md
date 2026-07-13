---
change_id: c645-update-app-tui-session-tree-fork
title: "产品会话树：Shift+F fork 接线"
status: purpose-draft
priority: 645
depends_on: ["c635-update-app-tui-session-tree-filter"]
author: agent
track: A
---

# c645-update-app-tui-session-tree-fork

## Why

travel（c615）只是回看/落叶；用户需要从历史节点 **fork** 新分支（对齐 pi / demo Shift+F）。

## Purpose

树开 Shift+F：经 `Driver` / `Command::Fork`（或等价 seam）分叉；user 预填策略对齐 demo；leaf=选中。

## What Changes（实现时）

1. 确认/补齐 Driver fork seam（已有 `Command::Fork` 则接线）。
2. 产品键 Shift+F；harness：fork 后 session/树增长、editor 预填符合 design。

## Capabilities

- `app-tui-session-tree`（modify）
- 可能触及 `agent-session` / Driver（仅 seam，不 reach infra）

## Design SSOT

- [`session-tree.md`](../../../src/app/tui/design/session-tree.md) travel/fork
- playground `tree-power` · fork 态
- 对照 [`session-tree-vs-pi.md`](../../../src/app/tui/design/session-tree-vs-pi.md)

## Impact

- 会话分支语义；须可测、可持久化

## Out of scope

- 跨文件「新会话文件」若与现有 store 模型冲突 → 在 promote 的 design.md 写清（对齐现有 fork，不盲抄 pi 文件布局）

## Ethics

- risk_level: medium
- prohibited_actions: 应用面直接改 session 内部；无 harness 的 fork
- required_evidence: harness + 持久化往返或等价合成测
- escalation_policy: 若 fork 语义与现有 JSONL 冲突，先停并问用户

## Depends

- **c635**（松：与 **c640** 可并行 apply）
