---
change_id: c716-update-app-tui-agents-layout
title: "src/app/tui/AGENTS.md：kimi 式布局地图与下沉边界"
status: full
priority: 716
depends_on: []
author: agent
track: QA
wave: stage-qa-docs
---

# c716-update-app-tui-agents-layout

## Why

`../kimi-code/apps/kimi-code/AGENTS.md` 用短地图 + 协调者/下沉边界 + 硬约束，比进度叙述更抗腐。xylitol `src/app/tui/AGENTS.md` 已有硬约束与验证表，但缺「文件布局一句话」与「禁止继续堆 host/root」的明确操作边界，下一波拆分易再次堆 God 文件。

## Purpose

仅文档：按 kimi 风格刷新 `src/app/tui/AGENTS.md`（布局表、模块职责、Esc/abort 分岔指针、size budget 提示），不改运行时行为。

## What Changes

1. 重写/增补 `src/app/tui/AGENTS.md` 本地专属节（保留现有 HOW / 验证 / PI_DELTAS 指针）。
2. 可选：`write-tui` skill 加一行「先读 AGENTS 布局表」。

## Capabilities

- `app-tui-host`（ath10）

## Out of scope

- 代码拆分（c730）；行为修复（c720/c725）；改根 `AGENTS.md` 长文

## Ethics

- risk_level: low
- prohibited_actions: 把进度板/行数/commit 列表写入 AGENTS；静默改 PI_DELTAS 决议
- required_evidence: 文档 diff 满足 ath10 场景；`validate --strict`

## Depends

- 无（可与 c715 并行）
