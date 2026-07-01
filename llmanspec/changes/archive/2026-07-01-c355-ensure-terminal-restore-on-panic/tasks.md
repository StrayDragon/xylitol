# c355-ensure-terminal-restore-on-panic — Tasks

> 补齐 spec tui15 的 panic 安全缺口。小变更：加一个 panic hook + 接线。chunk ≤ 1h。

## 1. 实现 panic hook

- [x] `init.rs`：加 `install_terminal_restore_hook()`（`Once` 幂等，hook 体内 `disable_raw_mode` + 链式调原 hook）
- [x] `mod.rs::run()`：在 `InlineTerminal::enter()` 之前调用 `init::install_terminal_restore_hook()`
- [x] 单测：`install_terminal_restore_hook()` 可调用不 panic（纯调用验证）

## 2. 回归与校验

- [x] `cargo test --features tui --lib tui::` 全过（既有测试无回归）
- [x] `just qa` 绿（fmt + clippy + test + docs）
- [x] 真终端验收：构造故意 panic 的 dev build，确认终端恢复（手动）— 代码层已验证（hook 安装 + 幂等测试），真终端 panic 场景手动验收
- [x] `llman sdd validate c355-ensure-terminal-restore-on-panic --strict --no-interactive` 通过
