---
change_id: c390-add-tui-debug-log
title: 为应用面装配 tracing subscriber（文件日志 MVP）
status: ready
priority: 390
depends_on: []
author: agent
---

# c390-add-tui-debug-log

> **状态: ready** — 方案已定稿（见 design.md 决策表），可进入 apply。

## Why

`tracing = "0.1.44"` 已是无条件依赖（`Cargo.toml:21`），`infra/`+`agent/` 里已有 ~12 处 `tracing::warn!/info!/debug!` 调用（event、permission、mcp、settings、hooks、compaction），但**全仓库没有任何 subscriber 初始化、没有 `RUST_LOG` 引用**（rg 验证）——这些埋点今天全是静默的。

TUI（`Viewport::Inline` + raw mode + 每帧 DSR 光标查询）下 stdout/stderr 被渲染管线独占，任何 `println!`/`eprintln!`/stderr 日志都会与光标响应交错、毁屏。业务复杂后（多轮 tool call、stream 取消、mpsc 竞态）肉眼观测不可持续，需要一条**不干扰渲染**的观测管线。

本变更不是「从零加日志」，而是**装 subscriber 激活既有埋点**：最小代价、立竿见影。

## What Changes

**file-only 同步 writer，env-only 激活，不碰 CliArgs。** 详见 design.md。

- 新增 `src/app/cli/logging.rs`：`init_logging()` 装一个 `tracing_subscriber::fmt::layer().with_writer(file)`，文件 writer = append-only `~/.xylitol/logs/xylitol.log`（`OpenOptions::append`，unix `mode(0o600)`），**同步**写入（非 `non_blocking`，避免 `panic="abort"` 下 guard 来不及刷盘）。
- 在组合根 `src/app/cli/mod.rs::run()`（`CliArgs::parse()` 之后、子命令分发之前）调一次 `init_logging()`，覆盖 print/TUI/RPC/subcommand 所有面。
- 激活优先级：`RUST_LOG` > `XYLITOL_DEBUG=1`（启用默认 filter）> 不装 subscriber（off）。**无 CLI flag、无 settings.json 字段**。
- 既有 `infra/`/`agent/` 埋点自动激活；TUI 层按需新增 seam 埋点（`driver.run` 提交/起/止/abort，`tui/mod.rs:194,250`），仅调 `tracing::` 宏、不跨 seam。

## Capabilities

- `debug-log`（新增 spec）：观察管线「不毁屏 + env 激活 + 文件落点」的不变量。

## Impact

- 低。纯新增：1 个 init 模块 + 1 个无条件依赖（`tracing-subscriber` env-filter）+ 组合根一行调用 + 可选 TUI 埋点。
- 不改变既有行为：subscriber 未装时所有 `tracing::` 宏仍是 no-op；装了只写文件，不碰 stdout/stderr。
- 不引入 feature flag（与既有 `tracing` 无条件依赖保持一致）。

## 来源

- 参考 codex（`tui/src/lib.rs:1189-1221` non-blocking file layer + `with_ansi(false)` + `RUST_LOG`）、pi/kimi-code（全部 file-only）、ratatui 官方 recipe 与社区共识。
- 用户决策（2026-07-03）：env-only 激活、无条件依赖、同步 writer。
