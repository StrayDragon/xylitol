# Tasks：驱动运行时命令面收口

- [ ] T1: 只读查询走快照——`model_registry()` 读快照，新增 `BuiltinsForReload` 命令。
- [ ] T2: 工具表命令化——`FreezeTools`/`ReopenToolsForRegate`/`BeginToolGating`/`ClearToolFreeze` 四个变体，同步 setter 改 async。
- [ ] T3: 资源与模型命令化——`ApplyPromptResources`/`ApplySkills`/`ReplaceModelRegistry`/`SelectModel(source)` 变体。
- [ ] T4: 删除待处理槽——`pending_workspaces`/`pending_tools` 删除，`set_tools`/`set_workspaces` 改 async 命令。
- [ ] T5: 组装期保留确认——`install_base_tools`/`ensure_tool_table_frozen` 加 `debug_assert` 确认 actor 为空。
- [ ] T6: 回归——全量 workspace 测试 + BDD + occupancy + MCP 全绿；`just qa` 与 `just qa-e2e` 实跑。
- [ ] T7: 收尾——`state_mut()` 计数对比回写；`llman-sdd validate --strict` 通过。
