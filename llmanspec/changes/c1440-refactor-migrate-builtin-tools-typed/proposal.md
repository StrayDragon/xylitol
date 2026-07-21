---
change_id: c1440-refactor-migrate-builtin-tools-typed
title: 将其余内置工具迁到 TypedTool（grep/write/edit/bash/read）
status: purpose-draft
priority: 1440
depends_on: []
author: agent
deferred: true
---

# c1440-refactor-migrate-builtin-tools-typed

> **purpose-draft / 延后（暂不 apply）**
>
> C3 样板已落地：`TypedTool` + blanket → `dyn XyTool`（`infra/tools/typed.rs`）；`ls` / `find` 已迁。
> 本 change 只记「把剩余可迁内置工具收齐」的意向；**用户明确升 full / 开 apply 前禁止实现**。
> 纯内部重构、不改工具参数名/默认值/错误文案 → 升 full 时仍可走 **不改 live specs** 的 quick/refactor 路径；若发现行为测钉死文案再补 contracts。

## Why

1. 内置工具已有 `*Args`（C1）与 `parse_tool_args`，但多数仍手写 `impl XyTool` + 入口再 parse，与 `ls`/`find` 双轨。
2. 统一到 `TypedTool` 后：parse 只在 blanket 一层；`execute_typed` / `execute_as_parts_typed` 吃 struct，减少重复与漏解析。
3. MCP / 动态工具继续留在 `Value` + `XyTool`——本 change **不**动跨进程口。

## Purpose（已钉）

将 `default_tools()` 中尚未 `impl TypedTool` 的内置工具全部迁完；行为与 schema **字面保持**。

| 工具 | 现状 | 迁移要点 |
|---|---|---|
| `ls` / `find` | 已 `TypedTool` | **不做** |
| `grep` | `XyTool` + `parse_tool_args` | 直迁 `execute_typed` |
| `write` | 同上 + `prompt_guidelines` | 迁时覆写 guidelines |
| `edit` | 同上 + `execution_mode` Sequential + guidelines | 迁时覆写 mode/guidelines |
| `bash` | 同上 + guidelines | 迁时覆写 guidelines |
| `read` | 覆写 `execute_as_parts`（图文） | 用 `execute_as_parts_typed`；`execute` 可委托 parts |

## What Changes（apply 时）

- `grep` / `write` / `edit` / `bash` / `read`：`impl XyTool` → `impl TypedTool`；`*Args` 按需 `pub`（关联类型可见性）
- 单测仍经 `XyTool::execute` / `execute_as_parts` 调用（blanket）
- 可选：更新 `src/AGENTS.md`「工具」段样板列表；本 draft 归档时去掉延后标记

## Out of scope

- 改 `XyTool` / MCP / 脚本 hook 的 JSON 合约
- 改工具参数名、默认值、截断策略、错误文案（除非单测强制对齐）
- `mutation` / `patch` / `accumulator` 等非 `XyTool` 装配叶
- 再抽 `xylitol-domain` 或动 protocol 分层

## Capabilities（升 full 时再钉）

- 倾向 **无** live spec 增改（内部重构）
- 若 BDD 钉死某工具错误字符串/输出形状被无意改动 → 再挂 `agent-tools` 等 capability

## Ethics

- risk_level: low
- prohibited_actions: 在本 change 仍为 purpose-draft 时写实现；改 MCP/hook JSON 口；改工具对外参数语义
- required_evidence（apply 前）: `default_tools` 内建均为 `TypedTool`（或有书面例外）；相关 `infra/tools` 测 + `just qa` 绿
- escalation_policy: 若某工具迁后需改对外文案才能绿 → STOP，升 SDD 或回退该工具

## 触发条件（何时升 full / apply）

任一条即可开闸：

1. 新增第二套内置工具装配路径，重复 parse 痛点再现
2. 批量改内置 Args/schema，希望只改 `execute_typed` 一侧
3. 人工明确要求「收齐 TypedTool」

在此之前：**只留草案，不实现。**
