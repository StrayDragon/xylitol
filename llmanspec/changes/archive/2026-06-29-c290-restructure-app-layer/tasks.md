# c290-restructure-app-layer — Tasks

Each task targets ≤2 hours of focused work. Validate after every task before moving on.

## T0 — 备份当前 `src/` 状态并确认 c285 已归档

- `git status` 干净
- `llman sdd list --json` 显示 c285 状态为 archived
- `cargo test --lib arch_guard` 通过（基线）

**Validation**: 上述三项全部通过。

## T1 — 创建 `app/` 层，迁移 `interactive/`

- `git mv src/interactive src/app`
- 把 `src/app/diff_review/` 移到 `src/app/tui/diff_review/`
- 更新 `src/lib.rs`：`pub mod interactive;` → `pub mod app;`
- 全局替换 `crate::interactive::` → `crate::app::`
- `cargo check --features cli` 通过

**Validation**: `cargo check --features cli` passes.

## T2 — 迁移 `server/` 到 `app/server/`

- `git mv src/server src/app/server`
- 更新 `src/lib.rs`：移除 `pub mod server;`
- 全局替换 `crate::server::` → `crate::app::server::`
- `cargo check --features server` 通过

**Validation**: `cargo check --features server` passes.

## T3 — 拆分 `protocol.rs` 为 `protocol/` 目录

- 创建 `src/protocol/mod.rs`、`src/protocol/command.rs`、`src/protocol/event.rs`、`src/protocol/transport.rs`
- 把 `Command` 移入 `command.rs`，`Event` 移入 `event.rs`
- `protocol/mod.rs` 重新导出公共符号
- 删除 `src/protocol.rs`
- `cargo check --features cli` 通过

**Validation**: `cargo check --features cli` passes.

## T4 — 提取 `app::composition`

- 新建 `src/app/composition.rs`
- 定义 `BuildAgentOptions` 和 `build_agent(...)`
- 把 `app::cli::run` 中的 Agent wiring 替换为 `build_agent`
- 把 `app::rpc::run` 中的 Agent wiring 替换为 `build_agent`
- 把 `app::server::runtime::start` 中的 Agent wiring 替换为 `build_agent`
- `cargo test --lib` 通过

**Validation**: `cargo test --lib` passes.

## T5 — 添加 Cargo features 并迁移 `ui-review` → `tui`

- 更新根 `Cargo.toml` `[features]`：`cli`/`rpc`/`server`/`tui`/`gui`
- `src/app/mod.rs` 用 `#[cfg(feature = "...")]` 包裹 `cli`/`server`/`tui`/`gui`
- 把 `ui-review` feature 改名为 `tui`
- 把 `src/app/tui/diff_review/` 用 `#[cfg(feature = "tui")]` 包裹
- 更新 `src/lib.rs` 中 `run_review_demo` 的 feature 名
- 更新 `justfile`、`README`、CI 中所有 `ui-review` 引用
- `cargo check --features cli` 通过
- `cargo check --features server` 通过
- `cargo check --features tui` 通过

**Validation**: All three `cargo check --features <mode>` pass.

## T6 — 更新架构守卫 `src/tests.rs::arch_guard`

- 扫描路径 `src/app/` 替代 `src/interactive/`
- 扫描路径 `src/app/server/` 替代 `src/server/`
- 更新 composition-root 例外列表：`app/cli/`、`app/server/`、`app/rpc.rs`、`app/composition.rs`、`app/print.rs`
- 确保 `app/driver.rs` 不直接 import `crate::infra`
- `cargo test --lib arch_guard` 通过

**Validation**: `cargo test --lib arch_guard` passes.

## T7 — 更新文档与 specs

- 更新 `AGENTS.md` 项目结构描述为 `app/` 层
- 运行 `llman sdd validate c290-restructure-app-layer --strict --no-interactive`
- 运行 `llman sdd validate --all`

**Validation**: Both `llman sdd validate` commands pass.

## T8 — 全量验证

- `cargo fmt --check`
- `cargo clippy --lib --features cli`
- `cargo clippy --lib --features server`
- `cargo clippy --lib --features tui`
- `cargo test --lib`
- `cargo test --test bdd -- --test-threads=1`
- `cargo test --lib arch_guard`
- `cargo doc --no-deps --all-features`
- `cargo run -- --help`（默认 `cli` feature）
- `cargo run --features server -- server --help` 或等价验证
- `cargo run --features tui -- --help` 或等价验证

**Validation**: All checks green.

## T9 — 提交与归档

- 提交：`feat(app): restructure interactive into app layer, relocate server, split protocol, add composition`
- `llman sdd archive run c290-restructure-app-layer`

**Validation**: Change archived and `llman sdd validate --all` still passes.
