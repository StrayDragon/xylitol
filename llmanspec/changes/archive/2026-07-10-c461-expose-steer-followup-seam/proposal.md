---
change_id: c461-expose-steer-followup-seam
title: "暴露 steer / follow-up 队列到 Agent + Driver seam"
status: proposed
priority: 461
depends_on: ["c450-revise-app-tui-contract"]
author: agent
track: B
---

# c461-expose-steer-followup-seam

## Why

产品 TUI 已锁定：流中 Enter = **steer**（下一 ReAct 迭代注入），Alt+Enter = **follow-up**（当前 agent loop 全部完成后再跑）。pi 在 `packages/agent` 用 `PendingMessageQueue` + `runLoop` drain 实现；xylitol 仅有未接线的 `SteeringHooks`、无生产者的 `XyEvent::QueueUpdate`、以及 settings 里未传入 runtime 的 `SteeringMode`。

若不先扩 agent/Driver/protocol seam，c480 键位与 c465 bridge 无法真实接线。

探查依据：`_tmp_prompts/05-result.md`、pi `agent.ts` / `agent-loop.ts` / `agent-session.ts`。

## What Changes

1. **新建 `PendingMessageQueue`**（`src/agent/runtime/queue.rs` 或等价）：`enqueue` / `drain` / `clear` / `has_items`；mode = `all` | `one-at-a-time`（对齐 `SteeringMode`）。
2. **队列挂在 `Agent`（session 级）**，经 `Arc<Mutex<…>>`（或等价）供 ReAct async 闭包与外部 `Driver` 并发入队。
3. **ReAct 循环接线**：每轮模型调用前 drain steering 并注入 history；将停（无 tool_calls 且将结束）前 drain follow-up；入队/出队发射 `XyEvent::QueueUpdate`。
4. **`Driver` 扩展**：`steer` / `follow_up` / `clear_queue`（可分队列）/ `queue_stats`；`InProcessDriver` 实现；`RemoteDriver` 预留或经 Command 转发。
5. **`protocol::Command`**：增加 `Steer` / `FollowUp` / `ClearQueue`（或等价命名）；`dispatch` 路由（非 WS 专用）。
6. **settings wiring**：构造 Agent 时传入 `steering_mode` / `follow_up_mode`。
7. **abort 语义（已锁定）**：`abort` **只清空 steer 队列**；**保留 follow_up 队列**，供 UI（c480）restore 到编辑器。提供显式 `clear_queue` 供需要时清 follow_up。
8. **`SteeringHooks`**：本变更以直持 queue 为主路径；hooks 回调可保留为扩展点或标记废弃路径（不得再作为唯一接线方式）。

## Capabilities

- `agent-runtime`（modify）
- `protocol-app`（modify）

## Impact

- `src/agent/runtime/{queue,react,hooks,mod}.rs`
- `src/agent/session/`（Agent 持有 queue）
- `src/app/core/driver.rs`、`dispatch.rs`
- `src/protocol/command.rs`
- 可能触及 builder / composition 传 mode
- **不改** `src/app/tui` 产品 UI（键位在 c480）
- **不改** `packages/xylitol-tui`

## 不在范围

- TUI Esc/Enter/Alt+Enter 接线（c480）
- QueueUpdate 的 UI 呈现（c465/c475）
- bash `!` / session 树

## 验证

```bash
llman sdd validate c461-expose-steer-followup-seam --strict --no-interactive
cargo test -p xylitol --lib agent::
# 实现后：队列单测 + ReAct 注入单测 + Driver 方法单测
```
