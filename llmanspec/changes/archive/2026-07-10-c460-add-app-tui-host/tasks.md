# Tasks — c460-add-app-tui-host

- [x] 1. debug 默认日志（`logging.rs`）+ 更新 `src/app/tui/AGENTS.md`
- [x] 2. `HostEvent` / `HostSession` + min-size 策略（纯逻辑）
- [x] 3. `TerminalGuard`（Drop + panic hook + 信号退出标志）
- [x] 4. 空壳 `Shell` 组件（transcript/editor/footer）
- [x] 5. `run()`：真终端 host 循环（select! / poll）
- [x] 6. Harness 测试：min-size、dispatch 不经 start、resize 切回 shell
- [x] 7. Cargo：`tui` feature 带 `crossterm`；`llman sdd validate --strict`
- [x] 8. `just lint` + 相关 `cargo test`
