---
change_id: c540-update-wire-remote-parity
title: "线协议与 RemoteDriver 对齐进程内语义"
status: proposed
priority: 540
depends_on: ["c535-refactor-server-onto-driver"]
author: agent
track: A2
---

# c540-update-wire-remote-parity

## Why

进程内 EventStream 已有 `QueueUpdate` 等生命周期事件；`XyEvent::to_wire_event` 丢弃多项；`RemoteDriver` 多数命令 stub。远程 client 无法获得与 Print 对等的体验。

依据：`docs/architecture/库与多客户端.md`、`用户可见事件.md`。

## What Changes

1. 定义 wire `Event` 对生命周期闭集的映射策略（至少：`QueueUpdate`、关键 turn/message；明确仍可降级的变体）。
2. 补齐 `RemoteDriver` 对 `Driver` 的命令面（模型/会话/队列/导出等），经 REST/WS。
3. Server 侧确保发出对应 wire 事件（依赖 c535 后的 Driver 流）。

## Capabilities

- `protocol-app`（modify / add）
- `server-runtime`（add remote parity）

## Impact

- `src/protocol/event.rs`、`src/app/core/driver.rs`（RemoteDriver）
- `src/app/server/**`

## Out of scope

- 产品 TUI UI
- 厂商事件宽表（仍禁止）
