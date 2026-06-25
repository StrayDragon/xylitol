# Tasks — c260-refactor-domain-architecture

> 每个阶段（P0~P5）独立可验证。**每阶段结束必须** `cargo build --all-features && cargo nextest run --profile ci && cargo clippy --all-features -- -D warnings && cargo test --test bdd -- --test-threads=1` **全绿才进下一阶段**。P0~P4 行为不变（BDD 是护栏），P5 行为新增。

## P0 — 收敛归位 + 词汇改名（行为不变）

- [x] T1 `git rm src/agent/types.rs`；从 `agent/mod.rs` 删 `pub mod types;`（0 引用，先 grep 确认）
- [x] T2 `git mv src/agent/model_manager.rs src/agent/model/manager.rs`；更新 `model/mod.rs` re-export；改 `session/mod.rs` 的 use 路径
- [x] T3 `git mv src/agent/compaction_orchestrator.rs src/agent/compaction/orchestrator.rs`；更新 `compaction/mod.rs` re-export；改 `session/mod.rs` + `session/stats.rs` 的 use 路径
- [x] T4 `git mv src/agent/skill_manager.rs src/agent/skills.rs`；改 `agent/mod.rs`；全局替换 `crate::agent::skill_manager`→`crate::agent::skills`
- [x] T5 `git mv src/agent/config_value.rs src/infra/config/value.rs`；`infra/config/mod.rs` 加 `pub mod value;`；全局替换 `crate::agent::config_value`→`crate::infra::config::value`
- [x] T6 `git mv src/agent/traits.rs src/agent/tools/definition.rs`；`tools/mod.rs` re-export；全局替换 `crate::agent::traits`（若残留）
- [x] T7 `git mv src/interface src/interactive`；`lib.rs` 改 `pub mod interactive;`；全局替换 `crate::interface::`→`crate::interactive::`；更新 feature flag/diff_review 注释
- [x] T8 验证：`grep -rn "agent::types\|r#loop\|crate::agent::config_value\|crate::agent::(model_manager|compaction_orchestrator|skill_manager|traits)\|crate::interface" src/` 全 0；build + nextest + clippy + BDD 全绿

## P1 — 杀 `r#loop` + runtime 目录（行为不变）

- [x] T9 建 `src/agent/runtime/` + `mod.rs`（声明 queue/react/retry/stdout_guard 并 re-export）
- [x] T10 `git mv` loop.rs→react.rs、queue.rs、retry.rs、output_guard.rs→stdout_guard.rs
- [x] T11 `agent/mod.rs` 删 4 个旧 `pub mod`，加 `pub mod runtime;`
- [x] T12 全局替换：`crate::agent::r#loop::{...}`→`crate::agent::runtime::{...}`；retry/queue/output_guard 引用改走 runtime re-export
- [x] T13 验证：`grep -rn "r#loop" src/` = 0；build + nextest + clippy + BDD 全绿

## P2 — agent 瘦身 + facade（行为不变）

- [x] T14 拆 `runtime/react.rs`：剪 `AgentEvent`/`AgentEventStream`→`runtime/event.rs`；剪 `AgentHooks`/`SteeringMode`/`FollowUpMode`→`runtime/hooks.rs`；react.rs 只留算法
- [x] T15 `git mv src/agent/bash_executor.rs src/agent/runtime/bash.rs`；全局替换引用
- [x] T16 建 `src/agent/prompt/`：mv prompt.rs→system.rs、commands.rs、templates.rs、skills.rs→prompt/{system,commands,templates,skills}.rs；建 `prompt/mod.rs` re-export；`agent/mod.rs` 改声明
- [x] T17 新增 `src/agent/facade.rs`：`pub struct Agent` 包装 `AgentLoop`，re-export `AgentEvent`/`AgentHooks`，提供 `new`/`with_hooks`/`run`/`session`/`abort`
- [x] T18 收口 `interactive/{rpc,print,cli}.rs`：改 `use crate::agent::facade::{Agent, AgentEvent}`；删对 `agent::runtime`/`session`/`tools` 的直接 import（cli 组合根构造期除外）
- [x] T19 验证：`grep -rn "crate::agent::runtime\|crate::agent::session::" src/interactive/` 仅出现在构造期；`wc -l runtime/react.rs` ≤350；build + nextest + clippy + BDD 全绿

## P3 — port 收口 + 运行时迁 infra（行为不变）

- [x] T20 `git mv src/core/traits.rs src/core/ports.rs`；`core/mod.rs` 改 `pub mod ports;`；全局替换 `crate::core::traits`→`crate::core::ports`
- [x] T21 `git mv src/agent/provider src/infra/provider`；`agent/mod.rs` 删声明、`infra/mod.rs` 加声明；全局替换 `crate::agent::provider`→`crate::infra::provider`
- [x] T22 provider 工厂移位：`agent/model/registry.rs` 若含 provider 构造逻辑，移到 `infra/provider/factory.rs` 或 cli 组合根
- [x] T23 `git mv` 内置工具 `agent/tools/{bash,read,write,edit,grep,find,ls,patch,mutation,accumulator,truncate,process,path_utils}.rs`→`infra/tools/`；`agent/tools/` 仅留 registry + definition
- [x] T24 组合根（cli）改为从 `infra::tools` 收集工具注入 `ToolRegistry`、从 `infra::provider` 构造 providers 注入 registry
- [x] T25 验证：`grep -rn "crate::agent::provider" src/` = 0；`grep -rn "infra::(provider|tools)::[A-Z]" src/agent/` = 0；build + nextest + clippy + BDD 全绿

## P4 — HC-2 修正 + 架构断言（行为不变）

> **P4 范围调整（HC-5 触发纪律）**: T26-T30（SessionStore/EventSink port + facade 重构）经评估为**投机抽象**：当前 593 测试用真实后端全绿、无 test-double 痛点，AgentSession 的全量 surface（fork/navigate/switch/stats/export/append_*）无法用窄 port 覆盖而不变成 god-trait。按 design §6.3「port 方法集应由真实需求驱动」与总览 HC-5，这些 port **推迟到 P5 server 托管暴露真实 API 需求时再立**（TDD-reverse：server 需要什么，port 就定义什么）。本阶段只做架构断言固化（T31），锁定 P0-P3 成果。

- [ ] T26 在 `core/ports.rs` 新增 `SessionStore` trait — 见上方 HC-5 说明 (defer → c265-add-server-runtime)
- [ ] T27 `infra/session::SessionManager` impl `SessionStore` + 内存双 (defer → c265-add-server-runtime)
- [ ] T28 `EventSink` trait + EventBus impl + 收集器双 (defer → c265-add-server-runtime)
- [ ] T29 facade 改 port 注入、去 session_id（HC-2） (defer → c265-add-server-runtime)
- [ ] T30 cli 组合根注入 port (defer → c265-add-server-runtime)
- [x] T31 扩展架构断言（src/tests.rs::arch_guard）：固化 (1) infra 不 import agent (2) agent 不 import provider 具体实现；未断言项（SessionManager/EventBus 具体耦合）标注 NOTE 待 P5 port 落地后启用
- [x] T32 验证：arch_guard 绿（infra→agent=0, agent→provider 具体=0）；build + nextest + clippy + BDD 全绿

## P5 — server 常驻 + protocol 统一交互（行为新增）

> **P5 拆分**: protocol 抽取（T33-T35）属重构、已在本 change 完成。server 进程化（T36-T48）与 HC-2 port（T26-T30）属新功能开发，按 SDD 原子性原则拆出到 `c265-add-server-runtime`（`depends_on: [c260]`）。本 change 到此 archive-ready。

- [x] T33 新建 `src/protocol.rs`（Command/Event enum 单文件 SSOT；envelope/error-code typing 推迟到 server 落地）
- [x] T34 把 `interactive/rpc.rs` 的 `RpcCommand`/`RpcEvent` 类型搬到 `protocol/`，重命名为 `Command`/`Event`
- [x] T35 `interactive/rpc.rs` 退化为 stdio transport：解析 stdin→Command，序列化 Event→stdout（0 enum 定义）
- [ ] T36 新建 `interactive/driver.rs`：`Driver` trait + `InProcessDriver` (defer → c265-add-server-runtime)
- [ ] T37 `interactive/{cli,print}` 改用 `Driver` (defer → c265-add-server-runtime)
- [ ] T38 `Cargo.toml` 加 HTTP/WS 框架；新增 `src/server/` + `lib.rs` 声明 (defer → c265-add-server-runtime)
- [ ] T39 `server/runtime.rs`：装配 infra 运行时注入 agent ports (defer → c265-add-server-runtime)
- [ ] T40 `server/rest.rs`：`/api/v1` control 路由 + 统一 envelope (defer → c265-add-server-runtime)
- [ ] T41 `server/ws.rs`：WS 帧协议 + 每 session 单调 seq (defer → c265-add-server-runtime)
- [ ] T42 `server/lock.rs`：单实例锁 + port-retry (defer → c265-add-server-runtime)
- [ ] T43 重连 journal + resync_required (defer → c265-add-server-runtime)
- [ ] T44 反向 RPC gateway（approval/question） (defer → c265-add-server-runtime)
- [ ] T45 `RemoteDriver`（WS+REST），与 InProcessDriver 事件序列一致 (defer → c265-add-server-runtime)
- [ ] T46 CLI 加 `server run` / `server install` 子命令 (defer → c265-add-server-runtime)
- [ ] T47 新增/更新 BDD：rpc.feature 适配 protocol 演进；新增 server 重连、反向 RPC 场景 (defer → c265-add-server-runtime)
- [ ] T48 验证：`grep -rn "crate::agent\|crate::infra" src/interactive/` 仅出现在 driver 组合根；architecture.rs 含 interactive 断言；build + nextest + clippy + BDD 全绿 (defer → c265-add-server-runtime)

## 收尾

- [x] T49 确认主 `docs/` 不含重构临时文档（refactor-notes 已在本 change 内）；内容核对无误后无需额外动作
- [x] T50 更新 `AGENTS.md` 的 Project Structure 段，同步新模块树（protocol/server/interactive/runtime 迁移）；refactor-notes 保留在 change 内随归档冻结
- [x] T51 `llman sdd validate c260-refactor-domain-architecture --strict --no-interactive` 通过
- [x] T52 全量校验：`cargo build --all-features && cargo nextest run --profile ci && cargo clippy --all-features -- -D warnings && cargo test --test bdd -- --test-threads=1 && cargo fmt --check`
