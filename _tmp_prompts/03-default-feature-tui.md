# 提示词：Cargo default features 含 tui（用后即弃 / 可并入 c460）

> 合约已要求（c450 ath / design D6）；**实现**建议落在 `c460-add-app-tui-host` apply 时，或单独 chore。

## 任务

1. 读根 `Cargo.toml` `[features]`：`default = ["cli"]`，`tui = ["dep:xylitol-tui"]`。
2. 改为 default 包含 `tui`（例如 `default = ["cli", "tui"]`），确保 `cargo build` / CI 不漏编。
3. 跑 `cargo check` / `just lint` 相关目标；修复因 tui 默认开启暴露的编译问题（占位 `run()` 应仍可编过）。
4. 更新任何文档里「需 --features tui」的过时说明。

## 禁止

- 实现真正的 TUI 循环（那是 c460）
