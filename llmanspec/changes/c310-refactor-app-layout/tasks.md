# c310-refactor-app-layout — Tasks

每个 task ≤2 小时聚焦工作。每步后先验证再推进。

## T0 — 基线确认

- `git status` 干净
- 无活跃 change 冲突（`llmanspec/changes/` 仅 archive/ 与 not-planning/）
- `cargo test --lib arch_guard` 通过（基线绿）
- `llman sdd validate c310-refactor-app-layout --strict --no-interactive` 通过（工件已就绪）

**Validation**: 上述四项全部通过。

## T1 — (A) `print.rs` → `cli/print.rs`

- `git mv src/app/print.rs src/app/cli/print.rs`
- `src/app/mod.rs`：删 `pub mod print;`
- `src/app/cli/mod.rs`：加 `mod print;`（或 `pub(crate) mod print;`），调用点 `crate::app::print::run_print` → `crate::app::cli::print::run_print`
- `src/app/cli/print.rs` 内部 `use crate::app::driver::{Driver, EventStream}` → `use crate::app::core::driver::{Driver, EventStream}`（若 T3 已完成；否则先指向 driver 临时位置，T3 后统一修正）—— **建议 T1 在 T3 之后执行以避免二次修改**，见 T3 备注
- `cargo check --features cli` 通过

**Validation**: `cargo check --features cli` passes.

## T2 — (B) `ServerSubcommand` + `run_server` → `server/subcommand.rs`

- 新建 `src/app/server/subcommand.rs`（`#[cfg(feature = "server")]`）
- 把 `cli/mod.rs` 的 `ServerSubcommand` enum 与 `run_server` fn 迁入，改名 `run`（或保留 `run_server`）；`CliCommand::Server` 仍引用该 enum（通过 `use crate::app::server::subcommand::ServerSubcommand`）
- `cli/mod.rs` 的 dispatch 缩为一行：`Some(CliCommand::Server { action }) => crate::app::server::subcommand::run(action).await`
- `cargo check --features server` 通过；`cargo check --features cli` 通过（非 server 构建下 `CliCommand::Server` 分支被 cfg gate）

**Validation**: `cargo check --features cli` 和 `cargo check --features server` 都通过。

## T3 — (C) 新建 `app/core/`，迁入 `composition.rs` + `driver.rs`

> 建议执行顺序：先 T3（建立 core/ 与新路径），再 T1（print 直接引用新路径 `core::driver`，避免二次修改）。

- 新建 `src/app/core/mod.rs`，声明 `pub(crate) mod composition; pub(crate) mod driver;`
- `git mv src/app/composition.rs src/app/core/composition.rs`
- `git mv src/app/driver.rs src/app/core/driver.rs`
- `src/app/mod.rs`：`pub mod composition;` `pub mod driver;` → `pub(crate) mod core;`
- 全局路径迁移（`rg` 确认零残留）：
  - `crate::app::composition` → `crate::app::core::composition`（调用点：`cli/mod.rs`、`rpc.rs`、`server/runtime.rs`）
  - `crate::app::driver` → `crate::app::core::driver`（调用点：`cli/mod.rs`、`print.rs`）
  - `driver.rs` 内 `use crate::app::server::ws::...` 路径不变（server 未动）
- `cargo check --features cli` / `--features server` 通过

**Validation**: `rg "crate::app::composition\b|crate::app::driver\b" src/` 零命中；两个 check 通过。

## T4 — 同步 `src/tests.rs::arch_guard`

- `exempt_prefixes`：`["cli/", "server/", "tui/diff_review/"]` → `["cli/", "server/", "core/", "tui/diff_review/"]`
- `exempt_files`：`["driver.rs", "rpc.rs", "composition.rs", "print.rs"]` → `["rpc.rs"]`
- `app_driver_does_not_import_infra`：`scan("src/app/driver.rs", ...)` → `scan("src/app/core/driver.rs", ...)`
- 更新守卫顶部注释的文件地图（driver/print/composition 归位说明）
- `cargo test --lib arch_guard` 通过

**Validation**: `cargo test --lib arch_guard` passes（三个测试全绿：`app_only_from_driver`、`app_driver_does_not_import_infra`、`agent_does_not_import_infra_in_production`）。

## T5 — 更新 `AGENTS.md`

- 项目结构描述：`app/` 段落把 `composition.rs`/`driver.rs` 归入 `core/`；移除顶层 `print.rs`（归入 `cli/`）；`server/` 补 `subcommand.rs`
- 保持托管块 `LLMANSPEC:START/END` 不动

**Validation**: `AGENTS.md` 与实际 `src/app/` 树一致（人工 diff 核对）。

## T6 — spec 校验

- `llman sdd validate c310-refactor-app-layout --strict --no-interactive`
- `llman sdd validate --all`

**Validation**: 两条 validate 命令全过。

## T7 — 全量 QA

- `cargo fmt --check`
- `cargo clippy --lib --features cli`
- `cargo clippy --lib --features server`
- `cargo clippy --lib --features tui`
- `cargo test --lib`
- `cargo test --test bdd -- --test-threads=1`
- `cargo test --lib arch_guard`
- `cargo doc --no-deps --all-features`
- `cargo run -- --help`（默认 cli feature）
- `cargo run --features server -- server --help`（等价验证 server 子命令仍在）

**Validation**: 所有检查全绿。

## T8 — 提交与归档

- 提交：`refactor(app): group seams under core/, move print to cli/, relocate server subcommand`
- `llman sdd archive run c310-refactor-app-layout`

**Validation**: change 已归档，`llman sdd validate --all` 仍全过。
