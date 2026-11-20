# c3401 research — 双份形状实测证据

全部断言带 `路径:行号`，证据来自 2026-11-20 对当前工作树（base `99d9132`）的实测。

## R1 `XyDriver` trait 双份

- runtime：`crates/xylitol-runtime/src/driver/proto.rs:32` — `pub trait XyDriver: Send`，方法区间 `:33-270`。
- app-tui：`crates/xylitol-app-tui/src/driver/proto.rs:54` — `pub trait XyDriver: crate::dispatch::SessionCommandExecutor + Send`，方法区间 `:55-295`。
- `diff` 实测分歧：
  - `run_turn` 仅 runtime（`proto.rs:51-56`）。
  - `detach_session` 仅 app-tui（`:130-132`）。
  - `exposes_session_store` 仅 app-tui（`:221-223`）。
  - `bind_debug_scene` 仅 app-tui（`:295`）。
  - `LinkPhase` 枚举仅 app-tui（`:21-43`）。
  - 其余方法逐字相同。

## R2 配套类型双份

- runtime `driver/types.rs`：16 项公开声明。
- app-tui `driver/types.rs`：19 项公开声明。
- `diff` 实测分歧：
  - app-tui 独有 `CAPABILITY_PENDING_CUE`（`:108`）。
  - app-tui 独有 `McpReadyState` 六态枚举（`:110-119`）。
  - 同一份 `LoadedResourcesSnapshot` 数据上：runtime `mcp_tools_pending() -> bool`（`:111-138`）与 app-tui `mcp_ready_state() -> McpReadyState`（`:121-149`）是两套算法，判定顺序不同。

## R3 错误与翻译合约双份（引用 c3370 实测）

- `XyDriverError`：`runtime/driver/error.rs:11` 与 `app-tui/driver_error.rs:10`；7 变体 + 8 个共享 `From` 逐字相同，差异仅 runtime `From<RuntimeControlError/HookBlockedError>` 与 app-tui `From<ClipboardError>`。
- `DispatchOutcome` / `SessionCommandExecutor`：`kernel/src/dispatch.rs:13` 与 `app-tui/src/dispatch.rs:13`；25 变体逐字零差异；kernel 独有 `outcome_to_value` 序列化层。

## R4 装配面薄模块

- `crates/xylitol-kernel/src/occupancy/factory.rs:191-354`：`OccupancyDriver` 38 个方法声明，其中 25 处为 `XyDriver::xxx(&mut self.inner)` 纯转发。
- `crates/xylitol-runtime/src/driver/in_process/`：20 个 `set_*` / `attach_*` / `enable_*` 注入器（prepare.rs:44,48,52,59,67,71,85,93,109,117；mod.rs:153,157,161,169,225,260,264,268,294,393）。
- `crates/xylitol-kernel/src/driver/in_process/mod.rs:82-170`：`assemble_driver` → `assemble_driver_with_features_and_context` → `assemble_driver_full` 三层传参接力。
- `XyInProcessDriver` 结构体（runtime `in_process/mod.rs:62-93`）有 30+ 字段，其中 `agent` / `store` 为 `pub`。

## R5 落点可行性

- `crates/xylitol-plugin-sdk/src/ports/` 已有 16 个 port trait（`rg "^pub trait" crates/xylitol-plugin-sdk/src/ports/*.rs`），`XyDriver` 同族落点一致。
- `crates/xylitol-plugin-sdk/Cargo.toml` 已含 `tokio`（sync/macros/rt/time/process）/ `tokio-util` / `async-trait` / `futures` / `serde_json` / `serde`，`XyDriver` 异步签名零新增依赖。
- `rg "DispatchOutcome|SessionCommandExecutor|DispatchRejection" crates/xylitol-plugin-sdk/src/` → **零命中**，证明 c3385 决策未落地。
- `rg "ModelInfo|SessionState|CommandInfo|RuntimeReloadReport|LoadedResourcesSnapshot" crates/xylitol-plugin-sdk/src/` → **零命中**，c3385 Q3① 五件下沉同样未落地。

## R6 规则约束

- `llmanspec/specs/layer-architecture/layer-architecture.feature` `@req:r1535`：「进程内驱动必须自身实现端口，不得经 Deref 包装把调用绕过端口。」
- `src/AGENTS.md`「三层契约与 `Xy*`」：`XyDriver` + `XyEvent` 流 + `XyDriverError` 属「应用协议 → `xylitol-runtime`」；端口（`XyModel` / `XyTool` / `XySessionStore`）属 plugin-sdk。
- `src/AGENTS.md` crate 表：`xylitol-protocol`「不得认识端口；不得认识 `XyEvent`」；`xylitol-app-tui`「不得依赖 runtime、官方插件」。
- `find llmanspec/specs -name design.md` → **零命中**，无既有 ADR 可冲突。

## R7 回归面

- `rg "impl .*XyDriver for" crates/ --glob "*.rs"` → 8 处：runtime `proto.rs:280`（Stub 测试）、runtime `in_process/mod.rs:538`（生产）、kernel `dispatch.rs:214`（测试桩）、kernel `occupancy/factory.rs:198`（`OccupancyDriver`）、serve `occupancy/host.rs:3021`（ScriptedEngine 测试）、app-tui `dispatch.rs:107`（测试桩）、app-tui `harness.rs:525`（`ScriptedDriver`）、app-tui `effects/pending_ui/mod.rs:142`（`OAuthDriver` 测试）。
- 生产实现 2 处（runtime `XyInProcessDriver`、kernel `OccupancyDriver`），其余 6 处为测试桩——迁移后测试桩统一实现 sdk trait 一份。
