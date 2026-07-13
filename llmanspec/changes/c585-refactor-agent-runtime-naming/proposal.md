---
change_id: c585-refactor-agent-runtime-naming
title: "Rename ReActAgent/Agent → AgentRuntime/AgentCapabilities"
status: purpose-draft
priority: 585
depends_on: []
author: agent
---

# c585-refactor-agent-runtime-naming

> **status: purpose-draft** — 命名决议已锁定；升格 specs/tasks 前勿改代码。

## Why

三层都叫 agent（`Driver` → `ReActAgent` → `Agent`）+ Strategy 注释与代码倒置，理解摩擦大。应用面已走 `Driver`，rename 主要服务 crate 内可读性。

## 命名决议（锁定）

| 旧 | 新 | 理由 |
|---|---|---|
| `ReActAgent` | **`AgentRuntime`** | 跑循环的驱动器/运行时 |
| `session::Agent` | **`AgentCapabilities`** | 能力聚合体；**不用** `AgentContext`（已占用：`domain::message::AgentContext` = LLM 请求快照） |
| `domain::AgentContext` | **保持不变** | 避免双改 |

## What（升格后）

- 机械 rename + `agent/mod` re-export、`builder`、`app/core`、`embed`、bdd、API snapshot
- Delta：`agent-runtime` / `agent-session` / `layer-architecture` / `server-runtime` 中 MUST 点名
- **MUST NOT** 与 `c494` 同 PR

## 非目标（本 draft）

- 不拆 `react.rs` 循环；不改 ReAct 行为；不改 `Driver` 对外语义。

## Next

升格为 full（delta specs + tasks）后走 `llman-sdd-apply`。确认后可 `llman-sdd-propose` 补全工件。
