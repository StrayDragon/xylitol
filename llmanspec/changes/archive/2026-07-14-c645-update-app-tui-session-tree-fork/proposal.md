---
change_id: c645-update-app-tui-session-tree-fork
title: "产品会话树：Shift+F → Driver 新 session fork"
status: full
priority: 645
depends_on: ["c640-update-app-tui-session-tree-fold"]
author: agent
track: A
---

# c645-update-app-tui-session-tree-fork

## Why

travel（c615）只回看/落叶；用户需要从历史节点 **fork 出新会话**（对齐 pi `/fork`）。`Driver::fork_session` / `Command::Fork` / store 已具备；产品树 Shift+F 尚未接线。demo `ast5` 是**同会话 leaf** 形态，**不是**产品语义（产品选 **A：新 child session**）。

## Purpose

树开 Shift+F（选中 **user**，对齐 pi `/fork`）：

1. 经 store **分支路径 fork** 创建 child session（`get_branch` + 重链；父文件只读）。
2. **切换**到 child，关闭树，刷新 transcript / leaf。
3. 预填该 user 正文（可剥 steer 前缀）；**不**把该 user entry 写入 child。
4. **MUST NOT** 走 demo 同会话 `ast5`；**MUST NOT** 用文件序切片写 child（防存储坏）。

## What Changes

1. **修 `SessionManager` fork 存储（对齐 pi `createBranchedSession`）**：内容 = `get_branch` 路径 + 重链 `parent_id`；**禁止**文件序 `entries[..=i]`；**禁止**改写父 JSONL；header `parent_session`。
2. 产品树 Shift+F（**仅 user**，pi `/fork` `position: before`）：leaf = user.parent；预填正文；**不**把该 user 写入 child。
3. `effects::drain_pending`：`fork` → `switch_session(child)` → 关树 / 刷新 / 预填。
4. harness + store 测：兄弟分支不泄漏进 child；父文件不变；user-before 语义。
5. 规格 `ast10` / `ati25`；同步 keybindings / session-tree；必要时修订「记录 N 处分叉」BDD 到路径语义。
6. **不**改 demo `ast5`。

## Capabilities

- `app-tui-session-tree`（add：产品 Driver fork）
- `app-tui-input`（add：树开 Shift+F）

## Design SSOT

- 本 change `design.md`
- [`session-tree.md`](../../../src/app/tui/design/session-tree.md)
- [`keybindings.md`](../../../src/app/tui/design/keybindings.md)
- 对照 [`session-tree-vs-pi.md`](../../../src/app/tui/design/session-tree-vs-pi.md)（pi 新文件 vs demo 同会话）

## Impact

- `src/app/tui/layout/root.rs`、`host.rs`、`effects.rs`、`harness.rs`
- 会话文件：多一个 child JSONL；当前 Driver 指向 child

## Out of scope

- demo `ast5` 同会话 fork（保持）
- Shift+L/T annotation
- 跨目录「另存为」UI；用现有 `store.fork` 模型

## Ethics

- risk_level: medium
- prohibited_actions: 应用面直接改 session 内部；无 harness 的 fork；用 demo 同会话冒充产品 fork
- required_evidence: harness fork→switch→关树；user 预填；persist 往返或 ScriptedDriver 计数
- escalation_policy: 若 switch 后 leaf/scrollback 与 travel 语义冲突，design 写清并以 travel 重建为参考

## Depends

- **c640**（已归档；fold 键不冲突）
