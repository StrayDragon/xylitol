# Tasks — c725-refactor-app-tui-shared-bang-loop

- [x] 1. 确认 c720 已 archive；`validate c725-… --no-interactive`
- [x] 2. 抽出/提升 `map_crossterm_item` / `on_agent_stream_item`；`mod.rs` 与 bang 去重
- [x] 3. 移除 Esc 敏感 `run_pending_bash`；RPB 测改走 shared loop（c715 已完成）
- [x] 4. 审计无第三份 bang Esc select；agent 在 bang 中仍可 poll
- [x] 5. BASE + bang harness + bdd；fmt
- [x] 6. `validate --strict`；准备 archive
