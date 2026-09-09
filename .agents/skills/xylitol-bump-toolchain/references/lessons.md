# Bump lessons

按时间倒序。每条只记**这次新发现**，不复述 SKILL 正文。

## 2026-09-09 — 1.97.1 stable → nightly-2026-09-08

- **Pin**：`rust-toolchain.toml` `channel = "nightly-2026-09-08"`。
- **rustc**：`1.100.0-nightly (cea272fa3 2026-09-07)`。滚动 `nightly` 在同日 `rustup update` 后是 `(4aa1fbcf4 2026-09-08)`，**比 dated channel 新一 commit**。
- **rust-version**：`1.98.1`（当时 stable）。未写成 `1.100.0`。
- **残缺安装**：第一次 `rustup toolchain install` 中途杀掉 → `rustc` 找不到 `librustc_driver-*.so`，`cargo check` 报 `can't find crate for core`。处理：卸掉 dated toolchain、清目录、完整重装后再编。
- **Cargo lint**：`cargo::manual_readme`（删根 `readme = "README.md"`）；`cargo::unused_dependencies`（根直接依赖 `hex` 已无引用，删除；digest 用 `src/agent/tools/freeze.rs` 的 `hex_lower`）。
- **clippy (新 deny)**：`clippy::needless_late_init`（`src/app/cli/mod.rs` match 改成绑定元组）；`needless_range_loop`（`FanoutReporter::report` 改 `split_last_mut`；`xylitol-tui` vt harness 空白行改 `fill`）；`unnecessary_get_then_check`（compaction obs 测试改 `contains_key`）。
- **lockfile**：`cargo update` 锁了 187 个兼容升级。根 Cargo.toml 精确 pin 同步到 lock（tokio 1.53.1、clap 4.6.6、minijinja 2.24.0 等）。
- **推迟的 major**：`async-openai` 0.41.3 → 0.42.0；`jsonschema` 0.46.10 → 0.55.1。不要和本次 rustc pin 捆在一起。
- **future-incompat**：`nix v0.28.0`（传递依赖，未在本次升级）。`nix v0.31.3` 在 update 后不再出现在报告里。
- **验证**：`just qa normal` 绿（fmt / clippy / nextest / live-provider skip-or-pass / test-tui / doc / scripts / prek）。nextest 对 `test_ds_hook_kill_on_timeout` 报了 LEAK，未当失败。
