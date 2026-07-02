# c390 Design — TUI Debug 日志管线（draft 占位）

## 待定

方案未定。候选见 proposal.md。

## 硬约束（无论方案）

- TUI 代码不 import infra（trace 初始化放 `main.rs` 或 `app::cli` 入口，TUI 只调 `tracing::` 宏）。
- 日志输出不干扰 ratatui buffer（文件写入天然满足）。
- `tracing` 已在 Cargo.toml，不重复加。
