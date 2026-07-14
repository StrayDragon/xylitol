---
change_id: c725-refactor-app-tui-shared-bang-loop
title: "生产与 harness 共用 bang 扇入：落实 ath7"
status: full
priority: 725
depends_on: ["c720-fix-app-tui-abort-suppress-race"]
author: agent
track: QA
wave: stage-qa-s2
---

# c725-refactor-app-tui-shared-bang-loop

## Why

ath7 要求单一扇入；c715 已抽出 helper 并消测试三份逻辑，但生产仍可能以「嵌套 select 包一层 helper」形式残留重复事件臂。abort 时序在 c720 稳住后，适合收敛环拓扑。

## Purpose

1. `run_host_loop` 与 harness 对 bang/终端/agent/tick **共用同一事件臂实现**。
2. 删除 Esc 敏感路径上的裸 `run_pending_bash`。
3. 保持 bang/agent abort 文案分岔；用户可见行为相对 c720 **不变**（重构）。

## What Changes

1. 提升 c715 helper 为唯一 bang 扇入口；`mod.rs` 去掉重复 match 臂。
2. 可选：将 bang_fut 并入外层单一 `select!`（若更清晰）；否则保留 `if take_bash { shared_loop() }` 但臂代码零重复。
3. harness / BDD 全走共享入口；删 HRS 残留。

## Capabilities

- `app-tui-host`（modify ath7）

## Out of scope

- HostSession 文件拆分（c730）；改 abort 语义（属 c720）

## Ethics

- risk_level: medium
- prohibited_actions: 重构中改变 cancelled/Aborted 分岔；bang 期间饿死 agent poll
- required_evidence: BASE/HRS/PTY bang Esc；ath7 场景审计

## Depends

- **c720** archive 后 apply
