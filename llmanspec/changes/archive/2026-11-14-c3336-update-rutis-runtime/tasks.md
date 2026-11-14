---
depends_on: []
title: "Upgrade rutis runtime to 0.21.0"
---

# Tasks — c3336 rutis 升级到 0.21.0

- [x] T1 调研：crates.io 版本、两版 `public-api.txt` 与源码 diff、迁移指南与 release notes；产出 `research/rutis-0.21.md`。
- [x] T2 依赖升级：`Cargo.toml` 两处精确锁改 `=0.21.0`；`cargo update` 刷新 `Cargo.lock`（`rutis` / `rutis-loader` / `rutis-macros` 均 0.21.0）。
- [x] T3 常量与文档同步：`RUTIS_LOCK`、`src/lib.rs` 契约测试、`src/AGENTS.md` 锁版本表述。
- [x] T4 编译门禁：`cargo check --workspace --all-features` 与 `just lint` / `just lint-all` 绿（导出 `WASI_SDK_PATH`）。
- [x] T5 存量 lint 修复：`just lint-all` 暴露的 `tests/bdd` 两处 `&PathBuf`、一处 `Option` clone、`crates/xylitol-tui/src/tui/tests.rs` 一处 `needless_range_loop` 告警（与本升级无关的存量问题，单独提交）。
- [x] T6 全量门禁：`just qa` 绿（fmt / lint / test 3858 passed / test-tui / doc / scripts / spec validate）。
- [x] T7 收口：`llman-sdd validate c3336 --strict` 绿并 `change finalize` 归档。
