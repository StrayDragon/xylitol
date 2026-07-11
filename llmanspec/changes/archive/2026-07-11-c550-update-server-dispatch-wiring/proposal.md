---
change_id: c550-update-server-dispatch-wiring
title: "Server 命令经 dispatch 统一执行"
status: proposed
priority: 550
depends_on: ["c540-update-wire-remote-parity"]
author: agent
track: A3
---

# c550-update-server-dispatch-wiring

## Why

c535/c540 后 Server 已持 `InProcessDriver`，但 REST 仍手写 `driver.select_model` / steer 等。`dispatch(Command)` 是跨面命令 SSOT，TUI 开闸后也要复用；Server 先接线可验证并减少双路径。

## What Changes

1. 将可映射的 REST 命令改为构造 `protocol::Command` → `dispatch`。
2. 保留 run/abort/WS 等不宜进 dispatch 的路径（与现有 dispatch 非目标一致）。
3. 文档：`docs/architecture/库与多客户端.md` 更新「dispatch 无消费方」缺口。

## Capabilities

- `server-runtime`
- `layer-architecture`

## Impact

- `src/app/server/rest.rs`、可能 `dispatch` 小扩

## Out of scope

- 产品 TUI slash UI（属 Track B）
- 改变对外 REST URL
