---
change_id: c535-refactor-server-onto-driver
title: "Server 统一到 Driver 主线"
status: proposed
priority: 535
depends_on: ["c530-add-lib-embed-api"]
author: agent
track: A2
---

# c535-refactor-server-onto-driver

## Why

Print 走 `InProcessDriver`，Server 仍 `Mutex<ReActAgent>` 旁路——双后端。远程 `RemoteDriver` 无法与 Server 命令面对称，违背「多 client 同一条主线」。

依据：`docs/architecture/library-and-clients.md`。

## What Changes

1. Server `AppState`（或等价）持有 `InProcessDriver`（或 `Box<dyn Driver>`），REST/WS 经 Driver / `dispatch` 执行命令。
2. 去掉「临时 Driver 只为 set_tools 再 into_agent」过渡路径。
3. 保持现有 REST/WS 对外 URL 行为兼容（除非文档标明破坏性变更）。

## Capabilities

- `server-runtime`（modify / add）
- `layer-architecture`（add la-server-driver）

## Impact

- `src/app/server/**`
- 可能触及 `dispatch` 接线

## Out of scope

- 完整 RemoteDriver 实现（c540）
- 产品 TUI
