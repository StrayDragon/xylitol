---
depends_on: []
title: run 历史标识统一重命名为 turn 词表
branch: sdd/c3338-refactor-turn-identifiers
base_branch: main
base_sha: 262e5be3a14ceb8000f2498f2c0bfa9e0fcdb504
---

## Why

run 已在概念层退役，但标识符层仍存 RunCoordinator/RunId/RunLease/RunEvent/RunTransition/run_inflight/finish_run/active_run_id 等 run 前缀（约 100 处）。术语治理要求文档叙事零使用 run，标识符与文档词表不一致造成检索与理解负担。用户决策：彻底清理被替换概念。

## What Changes

- 标识符重命名（含 BDD 场景大纲 guard 词与 spec 引用）：RunCoordinator→TurnCoordinator、RunLease→TurnLease、RunId→TurnId、RunEvent→TurnEvent、RunTransition→TurnTransition、run_id→turn_id、run_inflight*→turn_inflight*、finish_run→finish_turn、active_run_id→active_turn_id、allocate_run_id→allocate_turn_id、has_active_run→is_executing
- spec 直改：agent-runtime.feature 状态机 requirement 与场景大纲的标识符同步（纯代码组织演进导致的 spec 过期，按规则直改 live .feature）
- 文档：根 AGENTS.md 与 src/AGENTS.md 删除「run 已退役为历史实现命名」段（移除即改写，不留墓碑）

## Impact

- 公共 API 面变化（TurnId 等），API surface snapshot 同步
- 行为零变化：纯重命名 + 既有测试全绿
