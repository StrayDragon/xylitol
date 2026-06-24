# c84-refactor-tui-diff-engine — Tasks

- [x] 删除废弃变更目录 `llmanspec/changes/c83-migrate-tui-to-ratatui/`(经用户确认,不进 not-planning)
- [x] 修改 `src/interface/tui/mod.rs`:移除启动时的 `agent_loop.run(prompt)`,改为 `agent_stream: Option<AgentEventStream> = None`,仅当 `pending_prompt` 首次被用户提交时才创建(复用 c82 的 pending_prompt 机制);agent 分支 `None` 时用 `pending()` 防止 select! 饥饿
- [x] 修改 `src/interface/cli/mod.rs`:`run_tui_engine` 调用去掉 `prompt` 参数,只传 `session_id` + `model_name`
- [x] 验证编译:`cargo build --features ui-tui`
- [x] 验证 TUI 单元测试继续通过(无修改):`cargo test --features ui-tui --lib -- interface::tui`
- [x] BDD 回归:`cargo test --test bdd -- --test-threads=1`
- [x] 结构性验证 r48:无 TTY 启动 `cargo run --features ui-tui -- tui` 应在 agent 调用前报错退出,无 API 调用
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c84-refactor-tui-diff-engine --strict --no-interactive`
