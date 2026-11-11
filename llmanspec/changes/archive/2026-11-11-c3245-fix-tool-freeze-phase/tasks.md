# Tasks — 工具表冻结相位

> 实施者导读：design §0 语义与不变量、§3 转移矩阵是验收基准。收口走 finalize，不列为任务。

- [x] T1 spec 增补（design §4）
  infra-mcp first-turn-tool-freeze 增补「能力覆盖重定稿须以冻结收口」；agent-runtime mcp-tool-table-freeze-gate 回归确认。验证：llman-sdd validate infra-mcp --strict。
- [x] T2 装配线收口与不变量（design §2 第 1 条、§3）[blocked-by: T1]
  apply_pending_capability_overlay 补终步 freeze_tools；装配序列外 Gating 稳态 debug_assert；reopen 配对约束文档化。单测：转移矩阵全组合 + overlay 后 phase == Frozen。
- [x] T3 set_tools Outcome（design §2 第 2 条）[blocked-by: T1]
  set_tools / set_tools_defer_prompt 返回 ToolsWriteOutcome；全部调用点（kernel composition 等）显式处理。单测：RejectedFrozen / RejectedMidTurn 路径。
- [x] T4 capability 相位显式 + 快照一致性（design §2 第 3 条）[blocked-by: T2]
  OverlayPhase { Idle, Pending }；集合差降 debug_assert；LoadedResourcesSnapshot.tools_table_frozen 与实际相位一致性回归（含 TUI mcp_tools_pending 谓词场景）。
- [x] T5 全量回归 [blocked-by: T2, T3, T4]
  just fmt / just lint / just test；llman-sdd validate --all --strict；infra-mcp BDD 门禁场景全绿。
