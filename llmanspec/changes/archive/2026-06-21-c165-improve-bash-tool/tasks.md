# c165-improve-bash-tool: Tasks

## Implementation

- [x] 在 `tools/bash.rs` 中定义 `BashOperations` trait
- [x] 在 `tools/bash.rs` 中实现 `RealBashOperations`（使用 c135 的 `find_bash()`）
- [x] 在 `tools/bash.rs` 中添加 `BashHooks`（pre_spawn, post_spawn）
- [x] 在 `tools/bash.rs` 中实现超时逐级降级（SIGTERM → 5s grace → SIGKILL）
- [x] 在 `RealBashOperations` 中集成 `find_bash()` 来自 c135
- [x] 在 `RealBashOperations` 中集成 `kill_process_tree()` 来自 c135（通过 sigterm_then_sigkill）

## Testing

- [x] 单元测试 — `BashOperations` trait 可注入 mock（MockBash 实现）
- [x] 单元测试 — pre_spawn hook 被调用（AtomicBool 验证）
- [x] 单元测试 — 超时路径
- [x] 现有 bash 测试继续通过（423 lib tests, +2 new）

## Verification

- [x] `cargo check` — 0 errors
- [x] `cargo test --lib` — 423 passed
- [ ] `cargo test --test bdd`（cancelled: pre-existing compilation errors）
- [x] `llman sdd validate c165-improve-bash-tool`
