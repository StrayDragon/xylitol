---
title: 驱动运行时命令面收口
depends_on: []
needs_specs_change: false
branch: sdd/c3435-refactor-driver-runtime-command-surface
base_branch: main
base_sha: 20a50ea72435dce9390f6190f86689aadfba7fb5
---

## Why

c3434 把会话四操作与 `bind_session` 命令化后，`XyInProcessDriver` 仍有 29 处 `self.state_mut()` 内联调用。这些调用在 actor 持有 runtime 时会 panic；c3434 用「写待处理槽、下条命令前由 actor 应用」的方式绕开了部分 setter（`set_tools`、`set_workspaces`），但这是过渡折中：

- 待处理槽只覆盖两个字段，其余 setter 仍直接 panic。
- 语义被推迟到「下一条命令」，而不是调用返回时就已生效，时序不可预测。
- 新增一个 setter 就得重复一遍同样的绕行逻辑，技术债会持续累积。

本 change 一次性收口：把剩余 `state_mut()` 按「只读查询 / 变更命令 / 组装期独占」三类处理，目标是 `mod.rs` 的 `state_mut()` 计数降到个位数，且不再有待处理槽这种延迟语义。

## What Changes

### 一、只读查询走快照

`model_registry()`、`builtins_for_reload()` 等只读查询改为读 `snapshot()` 或共享探针，不再借用 runtime。

### 二、变更操作走命令

工具表状态变更（`freeze_tools`、`reopen_tools_for_regate`、`begin_tool_gating`）、资源应用（`apply_prompt_resources`、`apply_skills`）、模型选择（`select_model`、`restore_model`、`cycle_model`）、thinking 级别与 compaction，全部改为 `DriverCommand` 变体。

### 三、删除待处理槽

`SharedRuntimeProbe` 的 `pending_workspaces` / `pending_tools` 删除；`set_tools` / `set_workspaces` 直接命令化，调用返回时语义已生效。

### 四、组装期独占保留

组装期（`install_base_tools`、`ensure_tool_table_frozen`、`run` 首次绑定会话）在 actor 尚未 spawn 时执行，保留内联路径；这些点用 `state_mut()` 是安全的，因为此时没有 actor 竞争。

## 实测证据基线（2026-11-22，main @ c3434 归档后）

- `mod.rs` `state_mut()` 调用点 29 处；`prepare.rs` 5 处；`session.rs` 1 处。
- `DriverCommand` 变体 11 个，构造点 10 处。
- 全量 workspace 测试 4798 通过。

## Impact

- 全量 4798 测试与 `just qa` / `just qa-e2e` 为回归地板。
- 预期 `state_mut()` 从 35 处降到个位数，待处理槽机制整体删除。
