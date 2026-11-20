# c3401 tasks — c3385 落点实施

单 task ≤ 2 小时。本 change 只做前置落点（见 design Q0）。

## T0.1 建 sdk 翻译合约模块

- [x] 在 `crates/xylitol-plugin-sdk/Cargo.toml` 加 `xylitol-protocol = { workspace = true }`（sdk → protocol 单向边，已被 `src/AGENTS.md` 允许）。
- [x] 新建 `crates/xylitol-plugin-sdk/src/dispatch.rs`（模块名以实施为准）：迁入 `DispatchOutcome`（25 变体）、`SessionCommandExecutor` trait、`DispatchRejection` 小类型、`validate_nonempty_thinking_level`、`transport_variant_error`、`variant_name`。
- [x] 错误类型改缝（Q0/Q3 已定）：sdk 侧 trait 与辅助函数签名统一为 `Result<_, DispatchRejection>`（原两侧各返回本地 `XyDriverError`，如 kernel `dispatch.rs:162/189`、app-tui `dispatch.rs:62/89`）；两端在 T0.3/T0.4 各加 `From<DispatchRejection> for XyDriverError`，`log_failure` / `detail_kind` / flatten 行为零变化。
- [x] 在 `crates/xylitol-plugin-sdk/src/lib.rs` 导出上述符号。
- 验证：`cargo check -p xylitol-plugin-sdk` 通过；sdk 模块内无对两端 `XyDriverError` 的引用（`rg 'XyDriverError' crates/xylitol-plugin-sdk/src/dispatch.rs` 零命中）。

## T0.2 下沉 5 个数据类型

- [x] 把 `ModelInfo` / `SessionState` / `CommandInfo` / `RuntimeReloadReport` / `LoadedResourcesSnapshot` 迁入 sdk（候选模块 `ports/driver.rs` 或既有 `session.rs`，实施期定）。
- [x] `crates/xylitol-runtime/src/driver/types.rs` 与 `crates/xylitol-app-tui/src/driver/types.rs` 改为 `pub use` 再导出。
- 验证：`rg 'pub struct (ModelInfo|SessionState|CommandInfo|RuntimeReloadReport|LoadedResourcesSnapshot)' crates/` 仅命中 sdk 一处；`cargo check --workspace --all-features` 通过。

## T0.3 kernel 删本地副本

- [x] `crates/xylitol-kernel/src/dispatch.rs` 删除 `DispatchOutcome` / `SessionCommandExecutor` / `validate_nonempty_thinking_level` / `transport_variant_error` / `variant_name` 本地定义，改 `use xylitol_plugin_sdk::dispatch::{...}`。
- [x] 保留 kernel 独有 `outcome_to_value`（RPC 结果序列化）。
- [x] `crates/xylitol-kernel/src/driver/command_exec.rs` 的 `impl` 改为实现 sdk trait（经本地 `CommandExecutorView` 包装，`XyInProcessDriver` 属 runtime，kernel 直 impl 外部 trait 触发 orphan rule E0117）。
- [x] `crates/xylitol-kernel/src/occupancy/factory.rs` 的 `execute_command` 改 import sdk。
- 验证：`rg 'pub enum DispatchOutcome' crates/xylitol-kernel/` 零命中；`cargo nextest run -p xylitol-kernel` 全绿。

## T0.4 app-tui 删本地副本

- [x] `crates/xylitol-app-tui/src/dispatch.rs` 删除本地 `DispatchOutcome` / `SessionCommandExecutor`，改 import sdk。
- [x] `crates/xylitol-app-tui/src/driver/proto.rs` 的 supertrait 改指 sdk 的 `SessionCommandExecutor`（经 `crate::dispatch` 再导出解析，字面零改动）。
- [x] `crates/xylitol-app-tui/src/harness.rs` 的 `impl SessionCommandExecutor for ScriptedDriver`（:1002）改实现 sdk trait（含 `XyRemoteDriver` 与测试 `OAuthDriver`/`StubDriver` 两处同步；行号已漂移）。
- 验证：`rg 'pub enum DispatchOutcome' crates/xylitol-app-tui/` 零命中；`cargo nextest run -p xylitol-app-tui` 全绿。

## T0.5 全量回归与门禁

- [x] `cargo nextest run -p xylitol-kernel -p xylitol-runtime -p xylitol-app-tui -p xylitol-serve` 全绿，记录输出（1548 passed / 7 skipped）。
- [x] `cargo nextest run --all-features --test bdd` 全绿，记录输出（1229 passed；crate 图门禁 allowlist 同步 sdk→protocol 边）。
- [x] `just fmt` + `just lint`，记录输出（fmt 重排导入，lint exit 0）。
- [x] 前后对比：`rg -c 'pub enum DispatchOutcome' crates/` 由 2 处降为 1 处；`rg -c 'pub trait SessionCommandExecutor' crates/` 由 2 处降为 1 处。
- [x] `llman-sdd validate c3401-refactor-runtime-driver-surface-contract-home` failed 0。

## 移出本 change（另立 change）

原 tasks 的阶段 1–4（`XyDriver` trait 收敛、`McpReadyState` 六态、`XyDriverError` 词表、`DriverSpec` 装配面、`OccupancyDriver` 删除、`ScriptedDriver` 瘦身）**不在本 change 实施**。设计输入保留在 `design.md` Q1–Q4，供后续 change 引用。