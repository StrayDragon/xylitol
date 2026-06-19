# Tasks

- [x] 1. 新增 `src/agent/bash_executor.rs::BashExecutor`（execute/on_chunk/abort/truncate/spill）
- [x] 2. `src/infra/session/types.rs` 增加 `SessionEntry::BashExecution` 变体（serde 向后兼容）
- [x] 3. `src/agent/session.rs` 增 `execute_bash` / `record_bash_result` / `abort_bash` + `!`/`!!` 前缀路由
- [x] 4. LLM 上下文构建处过滤 `exclude_from_context == true` 的 BashExecution
- [x] 5. 单元测试：流式 / 取消 / 截断 spill / 记录 / 入上下文与不入上下文
- [x] 6. 覆盖说明：`!`/`!!` 前缀解析与入/不入上下文由 lib 单测覆盖（端到端 BDD 需交互模式入口，待 c115 RPC 接入）
- [x] 7. Run `cargo test --lib` 与 `cargo test --test bdd -- --test-threads=1`
- [x] 8. Run `cargo fmt` 与 `cargo clippy`
- [x] 9. Run `llman sdd validate c95-add-bash-executor --strict --no-interactive`
- [x] 10. Run `just qa`
