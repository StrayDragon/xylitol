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
- [ ] T17 新增 `src/agent/facade.rs`：`pub struct Agent` 包装 `AgentLoop`，re-export `AgentEvent`/`AgentHooks`，提供 `new`/`with_hooks`/`run`/`session`/`abort`
- [ ] T18 收口 `interactive/{rpc,print,cli}.rs`：改 `use crate::agent::facade::{Agent, AgentEvent}`；删对 `agent::runtime`/`session`/`tools` 的直接 import（cli 组合根构造期除外）
- [ ] T19 验证：`grep -rn "crate::agent::runtime\|crate::agent::session::" src/interactive/` 仅出现在构造期；`wc -l runtime/react.rs` ≤350；build + nextest + clippy + BDD 全绿

## P3 — port 收口 + 运行时迁 infra（行为不变）

- [ ] T20 `git mv src/core/traits.rs src/core/ports.rs`；`core/mod.rs` 改 `pub mod ports;`；全局替换 `crate::core::traits`→`crate::core::ports`
- [ ] T21 `git mv src/agent/provider src/infra/provider`；`agent/mod.rs` 删声明、`infra/mod.rs` 加声明；全局替换 `crate::agent::provider`→`crate::infra::provider`
- [ ] T22 provider 工厂移位：`agent/model/registry.rs` 若含 provider 构造逻辑，移到 `infra/provider/factory.rs` 或 cli 组合根
- [ ] T23 `git mv` 内置工具 `agent/tools/{bash,read,write,edit,grep,find,ls,patch,mutation,accumulator,truncate,process,path_utils}.rs`→`infra/tools/`；`agent/tools/` 仅留 registry + definition
- [ ] T24 组合根（cli）改为从 `infra::tools` 收集工具注入 `ToolRegistry`、从 `infra::provider` 构造 providers 注入 registry
- [ ] T25 验证：`grep -rn "crate::agent::provider" src/` = 0；`grep -rn "infra::(provider|tools)::[A-Z]" src/agent/` = 0；build + nextest + clippy + BDD 全绿

## P4 — HC-2 修正 + 架构断言（行为不变）

- [ ] T26 在 `core/ports.rs` 新增 `SessionStore` trait（append/load/exists，仅 agent loop/compaction 调用的方法）
- [ ] T27 `infra/session::SessionManager` impl `SessionStore`；新增 `tests/support/in_memory_store.rs`（或 cfg(test) 双）
- [ ] T28 在 `core/ports.rs` 新增 `EventSink` trait；`infra::event::EventBus` impl 之；新增 test 收集器双
- [ ] T29 改 `agent/facade.rs`：`Agent::new(store: Arc<dyn SessionStore>, sink: Arc<dyn EventSink>, ...)`；`run(&mut self, prompt: &str)` 不再传 session_id（session_id 在 interactive 层绑定 store）
- [ ] T30 改 `interactive/cli` 组合根：构造 `Arc<dyn SessionStore>`/`Arc<dyn EventSink>` 注入 Agent
- [ ] T31 新增 `tests/architecture.rs`：固化 HC-1 全部 grep 断言（infra 不 import agent；agent 不 import infra 具体类型；interactive 不 import agent/infra 非 driver 根）
- [ ] T32 验证：architecture.rs 绿；`Agent::new` 签名无 Session/session_id；build + nextest + clippy + BDD 全绿

## P5 — server 常驻 + protocol 统一交互（行为新增）

- [ ] T33 新建 `src/protocol/` + `lib.rs` 声明；建 `command.rs`/`event.rs`/`envelope.rs`/`error.rs`
- [ ] T34 把 `interactive/rpc.rs` 的 `RpcCommand`/`RpcEvent` 类型搬到 `protocol/`，整理为 `Command`/`Event` enum（含 Run/Cancel/SwitchModel/.../ApprovalRequired/...）
- [ ] T35 `interactive/rpc.rs` 退化为 stdio transport：解析 stdin→Command，序列化 Event→stdout
- [ ] T36 新建 `interactive/driver.rs`：`Driver` trait（send/subscribe）+ `InProcessDriver`（持有 Agent 直接调）
- [ ] T37 `interactive/{cli,print}` 改用 `Driver`（先只 InProcessDriver）
- [ ] T38 `Cargo.toml` 加 HTTP/WS 框架（候选 axum + tokio-tungstenite）；新增 `src/server/` + `lib.rs` 声明
- [ ] T39 `server/runtime.rs`：装配 infra 运行时注入 agent ports（第二个组合根）
- [ ] T40 `server/rest.rs`：`/api/v1` control 路由 + 统一 envelope
- [ ] T41 `server/ws.rs`：WS 帧协议（hello/ack/event/resync_required）+ 每 session 单调 seq
- [ ] T42 `server/lock.rs`：单实例锁 + port-retry
- [ ] T43 重连 journal：server 维护事件 journal，`subscribe(sid, last_seq)` 回放，溢出推 resync_required
- [ ] T44 反向 RPC gateway：approval/question 推送 + call_id 匹配挂起 turn 恢复
- [ ] T45 `interactive/driver.rs` 加 `RemoteDriver`（WS+REST），验证与 InProcessDriver 事件序列一致
- [ ] T46 CLI 加 `server run` / `server install` 子命令
- [ ] T47 新增/更新 BDD：rpc.feature 适配 protocol 演进；新增 server 重连、反向 RPC 场景
- [ ] T48 验证：`grep -rn "crate::agent\|crate::infra" src/interactive/` 仅出现在 driver 组合根；architecture.rs 含 interactive 断言；build + nextest + clippy + BDD 全绿

## 收尾

- [ ] T49 确认主 `docs/` 不含重构临时文档（refactor-notes 已在本 change 内）；内容核对无误后无需额外动作
- [ ] T50 更新 `AGENTS.md` 的 Project Structure 段，同步新模块树（protocol/server/interactive/runtime 迁移）；refactor-notes 保留在 change 内随归档冻结
- [ ] T51 `llman sdd validate c260-refactor-domain-architecture --strict --no-interactive` 通过
- [ ] T52 全量校验：`cargo build --all-features && cargo nextest run --profile ci && cargo clippy --all-features -- -D warnings && cargo test --test bdd -- --test-threads=1 && cargo fmt --check`
