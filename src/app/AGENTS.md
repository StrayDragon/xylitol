# src/app/ 应用面

入口索引。分层 / 应用面状态 / seam：**只**在 `src/AGENTS.md`；全局与 AGENTS 写法：根 `AGENTS.md`。

| 任务 | 去哪 |
|---|---|
| 新增或改造应用面 | `write-surface` skill |
| 死代码分诊 | `audit-dead-code` skill |
| TUI 产品面 | `src/app/tui/AGENTS.md` + `write-tui` skill；对照 pi 刻意差异 → `src/app/tui/PI_DELTAS.md` |
| TUI 包内验证 | `test-tui-harness` skill |
| 通用 TUI 引擎/组件 | `packages/xylitol-tui/AGENTS.md` |
| steer / follow-up seam | `llmanspec/changes/c461-expose-steer-followup-seam/`（XyDriver 队列，非 UI） |
