# c135-add-shell-process-mgmt: Tasks

## Implementation

- [x] 创建 `src/infra/process/mod.rs` — 模块入口
- [x] 创建 `src/infra/process/shell.rs` — bash 发现 + shell 环境
- [x] 创建 `src/infra/process/group.rs` — 进程组管理（kill tree + 子进程跟踪）
- [x] 创建 `src/infra/process/child.rs` — 子进程可靠等待（管道滞留保护）

## Integration

- [x] 在 `agent/tools/process.rs` 中委托到 `infra::process::group::kill_process_tree()`
- [x] 在 `agent/bash_executor.rs` 中集成 `find_bash()` 用于 shell 路径
- [ ] 在 `agent/bash_executor.rs` 中集成 `build_shell_env()`（cancelled: 当前 bash_executor 不需要显式 env 注入）

## Testing

- [x] 单元测试 — bash 发现（Unix 检测）
- [x] 单元测试 — shell env PATH 注入
- [x] 单元测试 — 进程树终止
- [x] 单元测试 — wait_for_child 管道滞留保护
- [x] 集成测试 — bash_executor 现有测试继续通过（421 passed）

## Verification

- [x] `cargo check` — 0 errors
- [x] `cargo test --lib` — 421 passed (9 new process tests, 412 existing)
- [ ] `cargo test --test bdd`（cancelled: pre-existing compilation errors unrelated to this change）
- [x] `llman sdd validate c135-add-shell-process-mgmt`
