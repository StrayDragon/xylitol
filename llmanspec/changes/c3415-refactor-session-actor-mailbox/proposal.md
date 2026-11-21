---
depends_on: ["c3410-update-actor-execution-contract", "c3412-refactor-driver-trait-surface"]
needs_specs_change: false
title: 会话槽升格 SessionActor：有界信箱与三通道路由落地
---

## Why

落实 c3410 合约的执行权威侧改造：把手写的占用 / 宽限 / 驱逐状态机与排队拒绝特判，替换为单一 SessionActor 与有界信箱的物理性质。

## What Changes

- 新增 SessionActor（每会话一个，select 主循环）与有界命令信箱；命令枚举覆盖协议 Command 词表。
- 三通道路由：Queued 对话通道（排队加逐项撤销）、Exclusive bang 通道（忙碌拒绝）、Inline 控制通道（立即处理）。
- XyInProcessDriver 重写为 SessionActor 的 handle 门面；occupancy host 手写状态机删除。
- ReAct 内环、七阶段中间件、XyEvent 形状按 c3410 合约保持或重述。

## Capabilities

- agent-runtime（实现落点）
- server-core（占用语义随槽上收的验证）

## Impact

- serve 会话槽 journal 与流式 tape 机制保留；执行章节场景已在 c3410 重写，此处对齐实现并保持回归地板全绿。

## Open Questions

- driver 方法面增删清单（对 thoughtdag 嵌入端的方法面锁定）。
- SessionActor 与 serve 槽 journal 的所有权边界。
