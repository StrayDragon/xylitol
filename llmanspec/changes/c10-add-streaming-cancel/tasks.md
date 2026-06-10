# Tasks: 工具进程取消

## 共享

- [ ] T1: 创建 `src/agent/tools/process.rs` 并将 `kill_tree()` 从 `bash.rs` 移入
- [ ] T2: 更新 `src/agent/tools/mod.rs` 导出 `process` 模块

## Grep

- [ ] T3: 重写 `GrepTool::execute()` — spawn child + pid + cancel kill
- [ ] T4: 单元测试: cancel 立即触发 → Aborted 错误

## Find

- [ ] T5: 重写 `FindTool::execute()` — 同 grep 模式
- [ ] T6: 单元测试: cancel 立即触发 → Aborted 错误

## Bash

- [ ] T7: 重构 `BashTool::execute()` 使用共享 `kill_tree`

## BDD

- [ ] T8: 更新 `tests/bdd.rs` 中 bash/grep/find cancel 步骤定义
- [ ] T9: `cargo test --test bdd bash_cancel` 通过
- [ ] T10: `cargo test --test bdd grep_cancel` 通过
- [ ] T11: `cargo test --test bdd find_cancel` 通过

## 验证

- [ ] T12: `cargo test -p xylitol` 通过
- [ ] T13: `just qa` 通过
- [ ] T14: `llman sdd validate c10-add-streaming-cancel --strict --no-interactive`
