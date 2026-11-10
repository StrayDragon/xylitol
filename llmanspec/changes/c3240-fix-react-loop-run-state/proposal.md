---
depends_on: []
title: "Fix ReAct loop run-state consistency"
---

# ReAct 内环运行状态一致性修复

## Why

ReAct 内环的运行状态由散落的 bool / 原子计数拼出，无单一转移入口（扫描证据见 research/implicit-state-machines.md，A1/A3/A4/A6/A14/A15）：

- **RunCoordinator**（state.rs:96）：phase + cancel + active_turn + pending + RunLease.finished 五份状态，显式 enum 但转移分散；finish_run 早退路径可残留 cancel 与 Cancelling 相位。
- **RetryState**（retry.rs:74，已复核 react/mod.rs:1235-1290 调用点）：attempt 每次 run 一份但不随轮次重置；第 1 轮重试过后，第 2 轮即使未重试也满足 attempt()>0，发出**虚假 AutoRetryEnd**（该事件直达 TUI 通知条）。
- **turn_aborted**（react/mod.rs:1045）：16 处写点把取消、中间件故障、模型错误、模态不支持混为同一 bool；turn_end 只剩 Aborted|Ok 两值，归因丢失。
- **嵌套计数**（dispatch.rs:35）：nested 手工 fetch_sub 五处，新增早退路径漏减即永久占额。
- **settles 计数**（hooks.rs:24）：hook 只装一次，跨 run 不复位，第二个 run 立即触顶。
- **bang cancel 槽**（bang_exec.rs:13）：并发执行时后者覆盖前者，前者失去取消句柄。

## What Changes

- RunCoordinator 引入单一 transition（事件 × 当前相位 → 新相位），finish/cancel 路径收敛；Cancelling 且 cancel=None 等非法组合变为不可构造。
- RetryState 语义对齐：每轮（或每次用户 turn）复位 attempt；AutoRetryEnd 只在真实重试发生后发出。
- turn_aborted 改为归因枚举（UserCancel / MiddlewareStop /ModelError / ModalityUnsupported / ...），turn_end 事件透出归因。
- nested 与 run_inflight 类计数改 RAII guard（Drop 递减）；settles 计数随 run 边界复位。
- bang cancel 槽：并发执行显式报 busy 或排队，不覆盖。

## Capabilities

- agent-runtime（内环 / 队列 / abort / XyEvent）；关联 protocol-app（若 TurnEnd 归因进事件流）。

## Impact

- 范围：crates/xylitol-runtime（agent/runtime/{state,retry,dispatch,hooks,react}、driver/bang_exec）。
- 行为变更：AutoRetryEnd 事件不再虚假出现（TUI 通知条行为随之修正）；TurnEnd 归因字段若进事件流需 wire 映射同步（按 src/AGENTS.md，RunId 仍不得进事件）。
- 验证：两轮重试场景单测（先触发重试再正常轮，断言无 AutoRetryEnd）；取消归因 BDD。

## Open Questions

1. TurnEnd 归因是否进 XyEvent（跨进程可见，需 wire/protocol 同步）还是仅进程内 hook？
2. settle/stall 计数复位边界：run 开始 vs 用户 turn 开始？
