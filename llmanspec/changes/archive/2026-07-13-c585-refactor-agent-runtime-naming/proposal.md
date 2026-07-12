---
change_id: c585-refactor-agent-runtime-naming
title: "Rename ReActAgent/Agent → AgentRuntime/AgentCapabilities"
status: draft
priority: 585
depends_on: []
author: agent
---

# c585-refactor-agent-runtime-naming

## Why

三层都叫 agent（`Driver` → `ReActAgent` → `Agent`）+ Strategy 注释与代码倒置，理解摩擦大。应用面已走 `Driver`，rename 主要服务 crate 内可读性。

## 命名决议（锁定）

| 旧 | 新 | 理由 |
|---|---|---|
| `ReActAgent` | **`AgentRuntime`** | 跑循环的驱动器/运行时 |
| `session::Agent` | **`AgentCapabilities`** | 能力聚合体；**不用** `AgentContext`（已占用：`domain::message::AgentContext`） |
| `domain::AgentContext` | **保持不变** | 避免双改 |

## What Changes

1. 机械 rename 类型与 `agent/mod` re-export、`builder`、`app/core`、`embed`、bdd、API snapshot、skills 文档。
2. 更新 MUST 点名：`agent-runtime` / `agent-session` / `layer-architecture` / `server-runtime`。
3. 修正 `agent/mod.rs` Strategy 注释为 Runtime + Capabilities。

## 非目标

- 不拆 `react.rs` 循环；不改 ReAct 行为；不改 `Driver` 对外语义。
- 不与 TUI layout change 纠缠。

## Capabilities

- `agent-runtime`、`agent-session`、`layer-architecture`、`server-runtime`
