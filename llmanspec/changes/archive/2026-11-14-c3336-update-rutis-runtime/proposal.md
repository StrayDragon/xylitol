---
depends_on: []
title: "Upgrade rutis runtime to 0.21.0"
branch: deps/rutis-0.21
base_branch: main
base_sha: 4a9f9fc345adaf6b461e876e0ca6dafef917161b
needs_specs_change: false
---

# rutis 运行时升级到 0.21.0

## Why

本仓以精确锁 `=0.20.1` 使用 `rutis` 与 `rutis-loader`（`Cargo.toml:59-60`）。crates.io 已发布 `0.21.0`（2026-11-06），带来可读错误（`bad-value` 携带 `reason`、新增 `bad-result` 码、`no-method` 命名提供者）与 `FiberView::running` 新 API。0.21 有三处不兼容：Rust `rutis::Error::Value` 改为带 `reason` 的结构体变体、收到函数的调用改为异步（Node/Bun/Python）、绝大多数错误文案改写。本 change 调研升级面、执行升级并让全量门禁通过。

## What Changes

- 工作区依赖从 `=0.20.1` 升到 `=0.21.0`：`rutis`、`rutis-loader`（`rutis-macros` 随传递依赖升级），刷新 `Cargo.lock`。
- `crates/xylitol-plugin-sdk/src/rt.rs` 的 `RUTIS_LOCK` 常量与 `src/lib.rs` 的契约测试同步到 `0.21.0`。
- `src/AGENTS.md` 中「精确锁 `=0.20.1`」改为 `=0.21.0`。
- 不涉及 live spec：本仓不构造或匹配 `rutis::Error::Value`，也不断言 rutis 自产的错误文案；loader 未启用 `runtimes` feature，故 0.21 的跨进程错误与运行时状态改动不进入编译面。

## Capabilities

- 无（仓库 meta 依赖升级；`needs_specs_change: false`）。

## Impact

- `Cargo.toml`、`Cargo.lock`。
- `crates/xylitol-plugin-sdk/src/rt.rs`、`crates/xylitol-plugin-sdk/src/lib.rs`。
- `src/AGENTS.md`。
- `tests/bdd/steps_infra_plugin.rs`、`tests/bdd/steps_agent_runtime.rs`、`crates/xylitol-tui/src/tui/tests.rs`：`just lint-all` 暴露的存量 clippy 告警（`&PathBuf` ×2、`Option` clone ×1、`needless_range_loop` ×1），与本升级无关，单独提交修复以保持 pre-PR 门禁绿。

## Further Notes

- 调研底稿：`llmanspec/changes/c3336-update-rutis-runtime/research/rutis-0.21.md`。
- 一手来源：crates.io `rutis`/`rutis-loader` 版本元数据；rutis 仓库 `docs/migration-0.21.en.md`、`docs/releases/0.21.0.en.md`；两版 crate 源码 `public-api.txt` 与逐文件 diff。
