---
change_id: c610-add-driver-session-tree-kind
title: "Driver：SessionTreeKind 可扩展会话树 API"
status: full
priority: 610
depends_on: []
author: agent
track: P
---

# c610-add-driver-session-tree-kind

## Why

产品活树 / travel 需要经 Driver 缝读树与改 leaf，但 infra 的 `get_tree` / `navigate_tree` 名过泛，且后者只是 `branch()`，不等于 pi travel（含 `editor_text`）。后续还可能有非消息历史的 session 树，不宜再堆无修饰的 `get_tree`。

## Purpose

在 Driver（及 Remote/REST）上引入 **kind 参数化** 的会话树 API：首版只实现 `MessageHistory`；未实现 kind 明确报错。为 c615 产品接线提供稳定缝，而不把泛名 `navigate_tree` 直接抬到应用面。

## What Changes

1. **类型**（`app/core` 或 `domain`，不加 `Xy`）：`SessionTreeKind`（至少 `MessageHistory`）、`SessionTreeTravel { kind, selected_id, leaf_id, editor_text }`。
2. **Driver**：`session_tree(kind)` → `Vec<SessionTreeNode>`（或文档化的等价 DTO）；`travel_session_tree(kind, entry_id)` → `SessionTreeTravel`（MessageHistory：user→父 leaf + editor_text；非 user→leaf=id、无 prefill）。
3. **InProcess**：经 store/`SessionManager` 读树 + branch；travel 语义在 Driver（或紧邻 helper）实现，不把 pi 规则推给裸 `navigate_tree` 别名。
4. **Remote + REST**：MessageHistory 读树 / travel 端点；RemoteDriver 对接。
5. **Harness ScriptedDriver**：可记录调用或返回可控桩，供后续 c615。

## Capabilities

- `cli-entry`（Driver 面）
- `server-runtime`（REST / Remote 对齐）

## Out of scope

- 产品 TUI 换活树（→ **c615**）
- 第二棵 `SessionTreeKind` 的真实实现（仅预留枚举扩展点）
- 产品 Shift+F / `fork_session` 语义对齐
- 改包 `TreeSelector` / demo ast4（已归档）

## Impact

- `src/app/core/driver.rs`、`dispatch`（若加 Command）、`harness` ScriptedDriver
- `src/app/server/rest.rs`
- `src/domain/session_types.rs`（若 kind/travel 放 domain）
- 可选：infra `get_tree` 旁路文档化为 MessageHistory 实现细节，不强制本 change 全量改名
