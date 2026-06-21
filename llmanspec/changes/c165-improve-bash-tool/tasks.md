# c165-improve-bash-tool: Tasks

## Implementation

- [ ] 在 `tools/bash.rs` 中定义 `BashOperations` trait
- [ ] 在 `tools/bash.rs` 中实现 `RealBashOperations`
- [ ] 在 `bash_executor.rs` 中添加 `BashHooks`（pre_spawn, post_spawn）
- [ ] 在 `bash_executor.rs` 中实现超时逐级降级（SIGTERM → 5s → SIGKILL）
- [ ] 在 `bash_executor.rs` 中集成 `build_shell_env()` 和 `find_bash()` 来自 c135
- [ ] 在 `bash_executor.rs` 中集成 `kill_process_tree()` 和 `wait_for_child()` 来自 c135

## Testing

- [ ] 单元测试 — `BashOperations` trait 可注入 mock
- [ ] 单元测试 — pre_spawn hook 被调用
- [ ] 单元测试 — 超时逐级降级路径
- [ ] 现有 bash 测试继续通过

## Verification

- [ ] `cargo check`
- [ ] `cargo test --lib`
- [ ] `cargo test --test bdd`
- [ ] `llman sdd validate c165-improve-bash-tool`
