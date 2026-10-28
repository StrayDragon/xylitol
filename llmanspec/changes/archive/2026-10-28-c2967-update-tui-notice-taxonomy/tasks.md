# Tasks

- [x] 1. **specs**（spec）：`app-tui-fixed-zone` 改写 `r1227` / `r1930`，新增 `notice-taxonomy`；`app-tui-transcript` 明确耐久结果走滚动提示。测：`llman-sdd validate --all --strict`。
- [x] 2. **删除启发式通知**（app-tui）：移除 `HEURISTIC_ESTIMATE_NOTICE`、`heuristic_notice_emitted`、`maybe_heuristic_estimate_notice` 及调用点与相关测试。测：footer 仍含 `~` 且不再出现该通知。
- [x] 3. **调用点审计**（app-tui）：按分类把耐久结果从 toast 移到 scroll notice；保留 reload 软闸、忙态拒闸、链路状态为 toast。测：分类单测。
- [x] 4. **文档**（docs）：DESIGN.md / 端 AGENTS 记录通知分类。测：`just qa-doc-check`。
- [x] 5. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
