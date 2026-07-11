---
change_id: c525-add-async-queue-runtime
title: "异步可并发队列运行时"
status: proposed
priority: 525
depends_on: []
author: agent
track: A
---

# c525-add-async-queue-runtime

## Why

steer/follow-up 队列语义已由 c461 锁定，但实现偏同步 Mutex，且 QueueUpdate 双通道阻碍 TUI bridge。需要统一的**异步、可并发、可取消**队列运行时，供消息通道与未来同类编排复用（非插件平台）。

设计 SSOT（产品）：`docs/architecture/queue-and-interrupt.md`。
实现 SSOT：本目录 `design.md`。

## What Changes

1. 抽出异步队列运行时（多生产者 enqueue、单消费者 drain、按通道隔离）。
2. 统一 QueueUpdate 到活跃 EventStream（消灭 EventBus 旁路）。
3. 保持 abort / QueueMode 产品语义不变。
4. `QueueStats` 结构化（可与 c500 命名债一并）。

## Capabilities

- `agent-runtime`（add ar-q10, ar-q11）

## Impact

- `src/agent/session/queue.rs`（或 runtime/queue）
- `src/agent/runtime/react.rs`、`src/app/core/driver.rs`
- EventBus 生命周期旁路清理

## Out of scope

- 用户可见后台 job 系统
- 替换工具并行调度器
- 产品 TUI 开闸（仍冻结；本 change 为开闸前置能力）
