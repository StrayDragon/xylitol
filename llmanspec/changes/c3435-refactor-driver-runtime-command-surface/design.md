# Design：驱动运行时命令面收口

## 分类

29 处 `state_mut()` 分三类：

| 类别 | 点 | 处理 |
|---|---|---|
| 只读查询 | `model_registry`、`builtins_for_reload`、`current_model_id` 邻近 | 读快照或共享探针 |
| 变更操作 | `freeze_tools`、`reopen_tools_for_regate`、`begin_tool_gating`、`apply_prompt_resources`、`apply_skills`、`select_model`、`restore_model`、`cycle_model`、`set_thinking_level`、`force_compact`、`clear_tool_freeze`、`set_tools`、`set_workspaces`、`replace_model_registry` | 新增 `DriverCommand` 变体 |
| 组装期独占 | `install_base_tools`、`ensure_tool_table_frozen`、`run` 首次 bind | 保留内联（actor 未 spawn） |

## 设计点

1. **快照只读**：`model_registry()` 返回 `DriverSnapshot.model_list` 的克隆；`builtins_for_reload()` 读快照的 tool_names 再映射到 runtime 工具表需保留原样，因此改为新增 `DriverCommand::BuiltinsForReload`。
2. **命令形状**：`FreezeTools { tools, reply }`、`ReopenToolsForRegate { reply }`、`BeginToolGating { reply }`、`ApplyPromptResources { context_files, system_prompt, append_system_prompt, reply }`、`ApplySkills { skills, reply }`、`ReplaceModelRegistry { registry, reply }`、`ClearToolFreeze { reply }`。
3. **同步 setter 的 async 化**：`freeze_tools`、`reopen_tools_for_regate` 当前是同步 `&mut self`。命令化需要 await，因此必须改 async。受影响调用点须同步调整。
4. **待处理槽删除**：`SharedRuntimeProbe.pending_workspaces` / `pending_tools` 删除；`set_tools` / `set_workspaces` 改 async 命令。
5. **组装期保留**：`install_base_tools` / `ensure_tool_table_frozen` 在 actor 未 spawn 时执行，保留 `state_mut()` 内联，加 `debug_assert` 确认 actor 为空。
6. **模型选择去重**：`select_model` / `restore_model` / `cycle_model` 已有部分命令化（`SelectModel` / `CycleModel`），本次把 `restore_model` 也并入 `SelectModel`，source 作为字段。

## 验收

- `mod.rs` 的 `state_mut()` 从 29 处降到个位数（目标 ≤ 8）。
- 待处理槽机制整体删除，`set_tools` / `set_workspaces` 返回时语义已生效。
- 全量 workspace 测试 4798+ 全绿。
- `just qa` 与 `just qa-e2e` 实跑绿。
