# c135-add-shell-process-mgmt: Tasks

## Implementation

- [ ] 创建 `src/infra/process/mod.rs` — 模块入口
- [ ] 创建 `src/infra/process/shell.rs` — bash 发现 + shell 环境
- [ ] 创建 `src/infra/process/group.rs` — 进程组管理（kill tree）
- [ ] 创建 `src/infra/process/child.rs` — 子进程可靠等待

## Integration

- [ ] 在 `agent/bash_executor.rs` 中集成 `kill_process_tree()`
- [ ] 在 `agent/bash_executor.rs` 中集成 `build_shell_env()`
- [ ] 在 `agent/bash_executor.rs` 中集成 `find_bash()` 用于 shell 路径
- [ ] 在 `agent/bash_executor.rs` 中集成 `wait_for_child()` 替代现有等待逻辑

## Testing

- [ ] 单元测试 — bash 发现（Unix/Windows 模拟）
- [ ] 单元测试 — shell env PATH 注入
- [ ] 单元测试 — 进程树终止
- [ ] 单元测试 — wait_for_child 管道滞留保护
- [ ] 集成测试 — bash_executor 现有测试继续通过

## Verification

- [ ] `cargo check`
- [ ] `cargo test --lib`
- [ ] `cargo test --test bdd`
- [ ] `llman sdd validate c135-add-shell-process-mgmt`
