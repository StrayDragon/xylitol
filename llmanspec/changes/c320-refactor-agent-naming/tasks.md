# Tasks — c320-refactor-agent-naming

> 每个阶段（P0~P5）独立可验证。**每阶段结束必须** `cargo build --all-features && cargo nextest run --profile ci && cargo clippy --all-features -- -D warnings && cargo test --test bdd -- --test-threads=1` **全绿才进下一阶段**。P0~P2 行为不变（改名/删空壳/取消 facade），P3 行为不变（拆分是内部重组），P4 删 dead（零调用方），P5 文档+QA。
> 注：`--strict` 为归档闸门（要求 task 全勾选或 defer），实现完成（`llman-sdd-apply`）后逐项勾选再过。

## P0 — `AgentSession → Agent`（机械改名 + 快照重生成）

- [ ] T1 `agent/session/mod.rs`：`pub struct AgentSession` → `pub struct Agent`；`impl AgentSession` → `impl Agent`；inline test helper `fn make_session() -> AgentSession` 同步改名(defer → c320-refactor-agent-naming)
- [ ] T2 `agent/session/steering.rs:7`：`impl super::AgentSession` → `impl super::Agent`(defer → c320-refactor-agent-naming)
- [ ] T3 `agent/builder.rs`：`use crate::agent::session::AgentSession` + `AgentSession::new(...)` 构造点改名(defer → c320-refactor-agent-naming)
- [ ] T4 `agent/runtime/react.rs`：`use crate::agent::session::AgentSession` + 字段 `session: AgentSession` + 3 处 inline test `AgentSession::new(...)` 改名(defer → c320-refactor-agent-naming)
- [ ] T5 `agent/facade.rs`：`session()`/`session_mut()` 返回类型全限定路径 `&crate::agent::session::AgentSession` → `&crate::agent::session::Agent`(defer → c320-refactor-agent-naming)
- [ ] T6 app 层调用点改名：`app/core/composition.rs`、`app/core/driver.rs`、`app/rpc.rs`、`app/server/rest.rs`、`app/cli/mod.rs`（仅类型名，方法调用不变）(defer → c320-refactor-agent-naming)
- [ ] T7 `tests/bdd.rs`：`use xylitol::agent::session::AgentSession` + 3 处 `AgentSession::new(...)` 改名(defer → c320-refactor-agent-naming)
- [ ] T8 重生成 API 快照：`cargo insta review`（`tests/snapshots/agent_session_api_baseline__*.snap` 更新为 Agent）(defer → c320-refactor-agent-naming)
- [ ] T9 验证：`rg -n "\bAgentSession\b" src/ tests/` = 0；build + nextest + clippy + BDD 全绿(defer → c320-refactor-agent-naming)

## P1 — `AgentLoop → ReActAgent` + 删空壳 `Agent` + 合并方法

- [ ] T10 `agent/runtime/react.rs`：`pub struct AgentLoop` → `pub struct ReActAgent`；`impl AgentLoop` → `impl ReActAgent`；inline test `AgentLoop::new(...)` 改名(defer → c320-refactor-agent-naming)
- [ ] T11 `agent/runtime/mod.rs:19`：`pub use react::AgentLoop` → `pub use react::ReActAgent`(defer → c320-refactor-agent-naming)
- [ ] T12 `agent/builder.rs`：`use crate::agent::runtime::AgentLoop` + `AgentLoop::new(session)` 改名；`build()` 内 `Ok(Agent { loop_: AgentLoop::new(session) })` 调整（此阶段先保留空壳 Agent 结构，下阶段删）(defer → c320-refactor-agent-naming)
- [ ] T13 `tests/bdd.rs`：`use xylitol::agent::runtime::{AgentLoop, ...}` → `ReActAgent`；2 处 `AgentLoop::new(...)` + `fn make_agent(...) -> AgentLoop` 改名(defer → c320-refactor-agent-naming)
- [ ] T14 验证：`rg -n "\bAgentLoop\b" src/ tests/` = 0；build + nextest + clippy + BDD 全绿(defer → c320-refactor-agent-naming)

## P2 — 取消 facade 层，类型上提 `agent/mod.rs`

- [ ] T15 删 `agent/facade.rs` 里空壳 `Agent`（1 字段 `loop_` 纯转发）；其方法（`run`/`run_with_id`/`abort`/`cancel_token`/`session`/`session_mut`/`set_tools`/`replace_hooks`/`add_hook`/`set_permission`/`set_tool_mode`/`set_system_prompt`）合并进 `ReActAgent`（react.rs）(defer → c320-refactor-agent-naming)
- [ ] T16 `agent/facade.rs` 的 re-export 表面（`AgentBuilder`/`AgentHooks`/`BeforeToolHook`/`XyEventStream`/`XyEvent`）迁到 `agent/mod.rs`(defer → c320-refactor-agent-naming)
- [ ] T17 删 `agent/facade.rs` 文件；`agent/mod.rs` 移除 `pub mod facade;`(defer → c320-refactor-agent-naming)
- [ ] T18 `agent/mod.rs` 加 `//!` 模块文档：明确"库用户从 `agent/`（mod 级）进门，ReActAgent + Agent 是入口，其余子模块是内部"(defer → c320-refactor-agent-naming)
- [ ] T19 全局 import 替换 `crate::agent::facade::*` → `crate::agent::*`：`app/core/driver.rs`、`app/rpc.rs`、`app/server/rest.rs`、`app/core/composition.rs`、`agent/builder.rs`、`agent/runtime/react.rs`（test helper 全限定路径 `crate::agent::facade::Agent`）(defer → c320-refactor-agent-naming)
- [ ] T20 验证：`rg -n "agent::facade" src/ tests/` = 0；`ls src/agent/facade.rs` 不存在；build + nextest + clippy + BDD 全绿(defer → c320-refactor-agent-naming)

## P3 — 五块拆分（god-object 瘦身）

- [ ] T21 `agent/session/export.rs`：加 `pub struct SessionExporter { io: Arc<dyn XyExportIo> }` + 构造；搬 `export_to_html`/`export_to_jsonl`/`import_from_jsonl`/`share_as_gist`（接收 `&dyn XySessionStore` + sid，纯函数风格）；`Agent` 持 `exporter: SessionExporter`，方法变 delegate(defer → c320-refactor-agent-naming)
- [ ] T22 新建 `agent/session/bash.rs`：`pub struct BashExecHandler { executor: Option<Arc<dyn XyBashExecutor>>, cancel: Option<CancellationToken> }`；搬 `execute_bash`/`record_bash_result`/`abort_bash` + 自由函数 `record_bash_result`（从 mod.rs 迁入）；`Agent` 持 `bash: BashExecHandler`（execute_bash 需 `&mut`）(defer → c320-refactor-agent-naming)
- [ ] T23 新建 `agent/session/permission.rs`：`pub struct PermissionGate { engine: Arc<dyn XyPermission> }` + `get_permission() -> Arc<dyn XyPermission>`（react.rs:77 路由契约不变）+ `set_permission`；**删** 3 个 dead `check_permission_read/write/network`；`Agent` 持 `permission: PermissionGate`(defer → c320-refactor-agent-naming)
- [ ] T24 `agent/session/stats.rs`：加 `pub async fn compute(store: &dyn XySessionStore, sid: &str) -> Result<SessionStats, String>`（从 mod.rs `get_session_stats` 体下沉）；`Agent::get_session_stats` 变 1 行 delegate(defer → c320-refactor-agent-naming)
- [ ] T25 新建 `agent/session/trust.rs`：`pub fn save_trust_decision(cwd: &str, trust_store: &dyn XyTrustStore, trusted: bool) -> Result<bool, String>`（从 mod.rs 方法降级，去掉 self）；`Agent` 不再持此方法；更新调用点（若有）(defer → c320-refactor-agent-naming)
- [ ] T26 `agent/session/mod.rs`：`Agent` 字段从 25 瘦身（`export_io`/`bash_executor`/`bash_cancel`/`permission` 移入 collaborator，改为持 `exporter`/`bash`/`permission` collaborator 字段）；`new()` 签名调整；子模块 `mod bash; mod permission; mod trust;` 声明(defer → c320-refactor-agent-naming)
- [ ] T27 确认核心 surface 调用点零改或最小改：rpc.rs（export_to_*/execute_bash/get_session_stats/get_permission）、react.rs（get_permission/tools/hooks）、原 facade→mod setter；必要时加 delegate 方法保持兼容(defer → c320-refactor-agent-naming)
- [ ] T28 验证：build + nextest + clippy + BDD 全绿；API 快照若变（新增 collaborator 类型）则 `cargo insta review`(defer → c320-refactor-agent-naming)

## P4 — 清 dead code（逐个 grep 核实零调用方后删）

- [ ] T29 删整个 steering 域：`agent/session/steering.rs` 文件 + `mod steering;` 声明 + 10 个方法（steer/follow_up/clear_queue/pending_message_count/has_pending_steer/get_steering_messages/get_follow_up_messages/drain_queued_messages/message_queue/message_queue_mut）；先 `rg -n` 核实零生产调用方(defer → c320-refactor-agent-naming)
- [ ] T30 删 prompt 分发 dead：`process_prompt`（核实 CLI slash 分发未走它）(defer → c320-refactor-agent-naming)
- [ ] T31 删 model 边缘 dead：`cycle_forward`/`cycle_backward`/`registry_clone`(defer → c320-refactor-agent-naming)
- [ ] T32 删其他 dead：`share_as_gist`/`set_append_prompt`/`compact_current_session`/`compaction_threshold`/`compaction_settings`/`register_template`/`register_templates`/`register_command`/`register_skill_commands`/`expand_skill_command`/`set_skills`/`dispose`/`abort`(session 版)(defer → c320-refactor-agent-naming)
- [ ] T33 删散落的 `#[allow(dead_code)]`（逐个核实该符号确已删除或确有调用方后去掉 allow）(defer → c320-refactor-agent-naming)
- [ ] T34 验证：`cargo build --all-features` 无 dead_code warning（除刻意保留的）；nextest + clippy + BDD 全绿(defer → c320-refactor-agent-naming)

## P5 — arch_guard 文字 + 文档 + 全 QA

- [ ] T35 `src/tests.rs::arch_guard`：守卫规则不变，更新注释文档里引用 `agent::facade`/`AgentSession`/`AgentLoop` 的文字为 `agent/`(mod)/`Agent`/`ReActAgent`(defer → c320-refactor-agent-naming)
- [ ] T36 `cargo doc --no-deps --all-features` 通过（ReActAgent/Agent/SessionExporter/BashExecHandler/PermissionGate 文档渲染）(defer → c320-refactor-agent-naming)
- [ ] T37 全 QA：`just qa`（fmt check + clippy + nextest + docs + prek hooks）全绿(defer → c320-refactor-agent-naming)
- [ ] T38 归档前：`llman sdd validate c320-refactor-agent-naming --strict --no-interactive` 通过；`llman sdd archive run c320-refactor-agent-naming`(defer → c320-refactor-agent-naming)
