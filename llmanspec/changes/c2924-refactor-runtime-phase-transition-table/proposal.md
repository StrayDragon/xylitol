---
depends_on: []
title: agent-runtime RunCoordinator phase 状态机迁移表化（xstate E1-E4 守卫）
---

## Why

`RunCoordinator` 的 `RuntimePhase`（`Idle/Starting/Streaming/Cancelling`）是 agent 编排内核的显式状态机：6 个转移点、2 个 run_id 匹配守卫（`mark_streaming`/`finish_run` 的 stale-lease 防护）、1 个 idle 门控（`try_begin_immediate` 仅 Idle 可入）。现行覆盖=2 个单元单测（`reject_path_try_begin_only_when_idle` / `stale_finish_does_not_clear_newer_run`）+ 间接行为场景，**转移图无完备性守卫**——「Cancelling 后 stale finish」「守卫失败相不变形」等只有点状断言。c2922 已在 server-core r1786 验证迁移表+`check_transitions` 通路，本 change 复制到最核心的编排状态机。该模块在 rutis 集成「不迁移清单」内，表化资产不受集成影响。

## What Changes

- `agent-runtime.feature` 新增规则 `runtime-phase-transition-table`：场景大纲 `phase-transition-matrix`，列 `from/event/guard/to`，覆盖：
  - `Idle→Starting`（try_begin_immediate，guard=phase_idle）
  - `Starting→Streaming`（mark_streaming，guard=run_id 匹配）
  - `Starting→Cancelling` / `Streaming→Cancelling`（cancel_active）
  - `Starting→Idle` / `Streaming→Idle` / `Cancelling→Idle`（finish_run 匹配 run_id）
  - 守卫失败相：非 Idle try_begin 拒绝、异 run_id mark_streaming 不变形（以 guard 列区分，E2 确定性门禁验证同 (from,event,guard) 唯一 to）
- BDD 侧：`RunCoordinator` 直驱步骤（同 r1786「独立单元驱动转移真值」模式，不经 HTTP/agent 全链路），given 造初态+pending 预置、when 施加事件、then 观测 phase；binding sync fn 保 outline 全 case 展开。
- 既有 2 个单测保留（点状回归），迁移表接管图完备性。

## Out of Scope

- `ActiveTurnSlot` / pending 队列策略的表化（replace_pending 语义等非状态机维度）。
- app-tui-host 连接代际表化（等 rutis P4 边界，见 c2922 备忘）。
- agent-runtime 其余行为场景不动。

## Verification

- `llman-sdd validate --specs --strict` 含 check_transitions E1-E4 全过。
- `cargo test --test bdd`：新增 case 数 == 表行数，全绿（1025 基线+N）。
- `cargo test --lib runtime::state` 既有单测不回归。
