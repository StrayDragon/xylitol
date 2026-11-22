# Tasks：驱动运行时命令面收口

- [x] T1: 只读查询走快照——`model_registry()` 从槽位取回 runtime 后直读；`builtins_for_reload()` 保持同步语义（过滤 MCP 工具），每次读取刷新并发布到共享槽。
- [x] T2: 工具表命令化——新增 `SetTools`/`FreezeTools`/`ReopenToolsForRegate`/`BeginToolGating`/`ClearToolFreeze` 五个变体；`set_tools`/`freeze_tools`/`reopen_tools_for_regate`/`begin_tool_gating`/`clear_tool_freeze` 改 async。
- [x] T3: 资源与模型命令化——新增 `SetWorkspaces`/`ApplyPromptResources`/`ApplySkills`/`ReplaceModelRegistry`/`BuiltinsForReload`/`ScriptHookCancel`/`ScriptHookObserve`/`EnsureSession` 八个变体；`apply_prompt_resources`/`apply_skills`/`restore_model`/`select_model`/`cycle_model`/`set_thinking_level`/`compact`/`apply_pending_capability_overlay` 全部改走命令。
- [x] T4: 回退分支删除——`set_tools`/`freeze_tools`/`reopen_tools_for_regate`/`set_workspaces`/`apply_prompt_resources`/`apply_skills` 的 `if self.actor.is_some() { state_mut() }` 内联分支全部删除，统一走 `ensure_actor_spawned` + 命令。
- [x] T5: 组装期保留——`install_base_tools`/`install_product_tool_set`/`apply_prepare_tools`/`ensure_tool_table_frozen`/`arm_tool_freeze_gate`/`poll_tool_gate` 保留内联，因它们需要与 prepare hub 同步推进且不能重入 actor。
- [x] T6: 冻结状态发布——`SharedRuntimeProbe.tools_frozen` 新增；actor 每条命令后、内联冻结路径、`settle_tool_gate` 均发布，资源面快照据此保持一致。
- [x] T7: 回归——workspace 全量 4798/4798 通过；occupancy 48/48；MCP 21/21；composition 8/8；`just lint` exit 0；`just qa` 61/61 exit 0；`just qa-e2e` PTY 32/32 exit 0。
- [x] T8: 收尾——`state_mut()` 从 35 处降到 7 处（`mod.rs` 29→1，`prepare.rs` 5，`session.rs` 1）；`DriverCommand` 变体从 11 个增到 24 个。

## 剩余 7 处的性质

| 位置 | 数量 | 性质 |
|---|---|---|
| `mod.rs` `state()` 转发 | 1 | 公开接缝，转发到 `state_mut()` |
| `prepare.rs` | 5 | prepare 流水线内部（`apply_prepare_tools`、`ensure_tool_table_frozen` 相关、`poll_tool_gate`、`settle_tool_gate`），必须与 prepare hub 同步推进，不能重入 actor |
| `session.rs` | 1 | `session_tree` 的 `ensure_runtime_session` 之后的 runtime 借用 |

这 7 处都是「必须内联」的路径：actor 命令是串行的，而 prepare 流水线需要在同一调用栈内推进 hub 状态，走命令会死锁（本 change 实施中实测到两次死锁并已回退）。
