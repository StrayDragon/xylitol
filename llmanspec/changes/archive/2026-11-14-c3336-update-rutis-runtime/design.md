---
depends_on: []
title: "Upgrade rutis runtime to 0.21.0"
---

# Design — c3336 rutis 升级到 0.21.0

## 决策

### D1 直接升到 0.21.0，保持精确锁

`rutis` 与 `rutis-loader` 维持 `=` 精确锁，从 `0.20.1` 改 `0.21.0`。理由：本仓对 rutis 的 API 依赖集中在 `xylitol-plugin-sdk::rt` 与 kernel 的 boot/admit；0.21 的破坏面（`Error::Value` 形状）本仓未触及，精确锁可避免隐式漂移。`rutis-macros` 由 `rutis` 传递约束，随 `cargo update` 同升。

### D2 不改 live spec（`needs_specs_change: false`）

0.21 的三处不兼容中，只有「错误文案改写」可能触及产品可观察行为。本仓 kernel 的等待原因文案由 `describe_wait`（`crates/xylitol-kernel/src/boot.rs:488`）按 `WaitReason` 枚举自行措辞，不复制 rutis 文案；仓内无对 rutis 自产错误文案的断言。故升级不改变任何 spec 的需求语义。

### D3 不采纳 0.21 的新文案与新 API

kernel 的 `describe_wait` 文案保持不变（不改成 `nothing provides it yet` 等 0.21 措辞），避免无需求驱动的用户可见变更；`FiberView::running` 暂不接入（本仓已有自检路径 `describe_unready`）。两者都可在后续 change 按需采用。

### D4 常量与文档同步

`RUTIS_LOCK`（`rt.rs`）与 `src/lib.rs` 契约测试同改 `0.21.0`；`src/AGENTS.md` 的精确锁表述同步。不新增兼容别名或双版本解析。

### D5 存量 lint 与本次升级分开提交

`just lint-all`（pre-PR 本地门禁，不在 `just qa` 内）在升级前即因四处存量 clippy 告警失败：`steps_infra_plugin.rs` 两处 `&PathBuf`、`steps_agent_runtime.rs` 一处 `Option<AgentEndReason>` clone（该类型自 `lifecycle.rs` 即派生 `Copy`，本次未改）、`crates/xylitol-tui/src/tui/tests.rs` 一处 `needless_range_loop`。为让升级分支在两条 clippy 口径下都绿，按最小改动修复，并以独立 `test:` 提交与升级提交分离。

## 验证

- `cargo check --workspace --all-features`（导出 `WASI_SDK_PATH`）。
- `just lint` / `just lint-all`。
- `just qa` 全量门禁（fmt / lint / test / test-tui / doc / scripts / spec validate）。
- `llman-sdd validate c3336 --strict` 绿。

## 回退

还原 `Cargo.toml` 两行与 `RUTIS_LOCK`，`cargo update -p rutis --precise 0.20.1`。
