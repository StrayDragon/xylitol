---
change_id: c1440-refactor-migrate-builtin-tools-typed
title: 将其余内置工具迁到 TypedTool（grep/write/edit/bash/read）
status: in-progress
priority: 1440
depends_on: []
author: agent
branch: c1440-refactor-migrate-builtin-tools-typed
base_sha: 4bd9fe39fd8edb229e8b964d73696d5d862e5a0b
checkpointed: true
checkpoint_sha: 4bd9fe39fd8edb229e8b964d73696d5d862e5a0b
---

# c1440-refactor-migrate-builtin-tools-typed

> 用户已确认升 full / apply。纯内部重构、不改 live specs。

## Why

1. 内置工具已有 `*Args` 与 `parse_tool_args`，但多数仍手写 `impl XyTool` + 入口再 parse，与 `ls`/`find` 双轨。
2. 统一到 `TypedTool` 后：parse 只在 blanket 一层；`execute_typed` / `execute_as_parts_typed` 吃 struct。
3. MCP / 动态工具继续留在 `Value` + `XyTool`——本 change **不**动跨进程口。

## Purpose

将 `default_tools()` 中尚未 `impl TypedTool` 的内置工具全部迁完；行为与 schema **字面保持**。

| 工具 | 现状 | 迁移要点 |
|---|---|---|
| `ls` / `find` | 已 `TypedTool` | **不做** |
| `grep` | `XyTool` + `parse_tool_args` | 直迁 `execute_typed` |
| `write` | 同上 + `prompt_guidelines` | 迁时覆写 guidelines |
| `edit` | 同上 + `execution_mode` Sequential + guidelines | 迁时覆写 mode/guidelines |
| `bash` | 同上 + guidelines | 迁时覆写 guidelines |
| `read` | 覆写 `execute_as_parts`（图文） | 用 `execute_as_parts_typed`；`execute_typed` 委托 parts |

## What Changes

- `grep` / `write` / `edit` / `bash` / `read`：`impl XyTool` → `impl TypedTool`；`*Args` 按需 `pub`
- 单测仍经 `XyTool::execute` / `execute_as_parts` 调用（blanket）
- 可选：更新 `src/AGENTS.md`「工具」段

## Out of scope

- 改 `XyTool` / MCP / 脚本 hook 的 JSON 合约
- 改工具参数名、默认值、截断策略、错误文案（除非单测强制对齐）
- `mutation` / `patch` / `accumulator` 等非 `XyTool` 装配叶
- 再抽 `xylitol-domain` 或动 protocol 分层

## Capabilities

- **无** live spec 增改（内部重构）

## Ethics

- risk_level: low
- prohibited_actions: 改 MCP/hook JSON 口；改工具对外参数语义
- required_evidence: `default_tools` 内建均为 `TypedTool`；相关 `infra/tools` 测 + `just qa` 绿
- escalation_policy: 若某工具迁后需改对外文案才能绿 → STOP，升 SDD 或回退该工具
