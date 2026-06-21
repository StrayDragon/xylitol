# Tasks

- [x] 1. 新建 `src/infra/session/cwd.rs`（defer）
- [x] 2. 定义 `SessionCwdIssue` 结构体（defer）
- [x] 3. 定义 `MissingSessionCwdError` 错误类型（defer）
- [x] 4. 实现 `get_missing_session_cwd_issue(sm, fallback_cwd) -> Option<SessionCwdIssue>`（defer）
- [x] 5. 实现 `assert_session_cwd_exists(sm, fallback_cwd) -> Result`（defer）
- [x] 6. 实现 `format_missing_session_cwd_error()` / `format_missing_session_cwd_prompt()`（defer）
- [x] 7. 集成到 `interface/cli/mod.rs` 的会话恢复路径（defer）
- [x] 8. 集成到 `interface/rpc.rs` 的会话恢复路径（defer）
- [x] 9. 编写单元测试覆盖：目录存在、目录缺失、fallback（defer）
- [x] 10. `cargo test --lib` 全绿（414 passed）（defer）
- [x] 11. `cargo clippy` 无新增警告（defer）
