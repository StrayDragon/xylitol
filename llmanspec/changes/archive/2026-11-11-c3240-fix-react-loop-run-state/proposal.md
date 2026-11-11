---
depends_on: []
title: "Fix ReAct loop run-state consistency"
branch: sdd/c3240-fix-react-loop-run-state
base_branch: main
base_sha: 5f1c20b6302f590a2a82ef474a0175d1edd8432e
---

# ReAct 内环运行状态一致性修复

## Why

ReAct 内环的运行状态由散落的 bool / 原子计数拼出，无单一转移入口（扫描证据见 research/implicit-state-machines.md，A1/A3/A4/A6/A14/A15）：

- **RunCoordinator**（state.rs:96）：phase + cancel + active_turn + pending + RunLease.finished 五份状态，显式 enum 但转移分散；finish_run 早退路径可残留 cancel 与 Cancelling 相位。
- **RetryState**（retry.rs:74，已复核 react/mod.rs:1235-1290 调用点）：attempt 每次 run 一份但不随轮次重置；第 1 轮重试过后，第 2 轮即使未重试也满足 attempt()>0，发出**虚假 AutoRetryEnd**（该事件直达 TUI 通知条）。
- **turn_aborted**（react/mod.rs:1045）：16 处写点把取消、中间件故障、模型错误、模态不支持混为同一 bool；turn_end 只剩 Aborted|Ok 两值，归因丢失。
- **嵌套计数**（dispatch.rs:35）：nested 手工 fetch_sub 五处，新增早退路径漏减即永久占额。
- **settles 计数**（hooks.rs:24）：hook 只装一次，跨提交不复位，第二问立即触顶。
- **bang cancel 槽**（bang_exec.rs:13）：并发执行时后者覆盖前者，前者失去取消句柄。

## What Changes

> 2026-11-11 深挖定稿，范围扩充为「缺陷修复 + 词汇定稿 + 协议一次性改名」。词汇三层：session ⊃ turn（问答轮：一条输入到最终答复）⊃ step（一次模型调用周期 + 工具批）；run 降为宿主实现术语（一次提交 / 一个事件流），不进共享概念词汇。原 turn_index / TurnStart / TurnEnd 实为 step 边界，全部改名对齐，不留别名。

- 词汇与协议改名（一步到位）：XyEvent::TurnStart/TurnEnd → StepStart/StepEnd、turn_index → step_index；wire / protocol / obs（TurnEndReason → StepEndReason）/ 脚本 hook 事件名（turn_start / turn_end → step_start / step_end）/ 全部消费端与 BDD 同步；术语与 AGENTS.md 更新三层词汇定义与 run 实现术语注记。
- max_turns 语义重定：移除 session.max_turns、should_stop_after_turn 与 settles 计数（settle 计数防不了不 settle 的 runaway——永不收尾的循环根本不触发它）；新增 session.max_steps_per_turn（未配置不启用），经新单槽 should_stop_after_step 钩子在每次 step 收尾（含工具续跑）生效，达到上限 → AgentEnd(reason=StepLimit)。按提交计的问答轮上限记为有意放弃的能力。
- RetryState 修复：每 step（模型调用）边界重建；AutoRetryEnd 仅真实重试后发射；补 AutoRetry 事件语义 spec 条款。
- AbortReason 全栈透出：内部归因枚举（UserCancel / ModelError / MiddlewareStop / MiddlewareFault / ModalityUnsupported / StepLimit），turn_aborted 十六处写点收敛；AgentEnd 增可选字段 reason（None = 自然完成）；wire / TUI bridge（固定区词表）/ print / BDD 同步。
- RunCoordinator 转移收敛单一 transition 入口；mark_streaming 不得覆写 Cancelling（已核实竞态）。深挖修订：扫描指控「Cancelling 且 cancel=None 可达」复核后未找到构造路径，降级未确认。
- bang 取消槽：并发忙拒（单飞语义）；nested 计数 RAII guard（Drop 递减）。

## Capabilities

- agent-runtime（内环 / 队列 / abort / XyEvent）；关联 protocol-app（若 TurnEnd 归因进事件流）。

## Impact

- 范围：crates/xylitol-runtime（agent/runtime/{state,retry,dispatch,hooks,react}、driver/bang_exec）。
- 行为变更：AutoRetryEnd 事件不再虚假出现（TUI 通知条行为随之修正）；AgentEnd.reason 进事件流，wire 映射同步（RunId 仍不得进事件）。
- 验证：两轮重试场景单测（先触发重试再正常轮，断言无 AutoRetryEnd）；取消归因 BDD。

## 决策记录（2026-11-11，全部已决）

1. 归因进 XyEvent：AgentEnd 增可选字段 reason，wire / TUI / print 全栈透出。
2. settle 计数机制整体移除，由 session.max_steps_per_turn 经 should_stop_after_step 取代；无「复位边界」问题。
