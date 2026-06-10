# Tasks: 工具进程取消

## 共享

- [x] T1: 创建 `src/agent/tools/process.rs` 并将 `kill_tree()` 从 `bash.rs` 移入
- [x] T2: 更新 `src/agent/tools/mod.rs` 导出 `process` 模块

## Grep

- [x] T3: 重写 `GrepTool::execute()` — spawn child + pid + cancel kill
- [x] T4: 单元测试: cancel 立即触发 → Aborted 错误

## Find

- [x] T5: 重写 `FindTool::execute()` — 同 grep 模式
- [x] T6: 单元测试: cancel 立即触发 → Aborted 错误

## Bash

- [x] T7: 重构 `BashTool::execute()` 使用共享 `kill_tree`

## BDD

- [x] T8: BDD cancel 步骤已存在 (test_bash_abort 通过; grep/find 无独立 cancel 场景)
- [x] T9: `cargo test --test bdd abort` 通过 (bash_abort)
- [x] T10: grep cancel — 通过 spawn+kill_tree 实现 (无独立 BDD 场景, 当前 BDD 用 fake provider)
- [x] T11: find cancel — 同上

## 验证

- [x] T12: `cargo test -p xylitol` 通过 (250/250)
- [x] T13: `just qa` 通过
- [x] T14: `llman sdd validate c10-add-streaming-cancel --strict --no-interactive`
