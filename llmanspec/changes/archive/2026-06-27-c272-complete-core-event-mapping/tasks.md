# Tasks: c272-complete-core-event-mapping

## P1 — 补全 AgentEvent → protocol::Event 映射

- [x] T1 `protocol.rs::Event` 增补 TurnStart、TurnEnd、MessageStart、MessageEnd、MessageUpdate、ToolExecutionUpdate、CompactionEnd
- [x] T2 `server/rest.rs::agent_event_to_protocol()` 补全 7 个变体映射
- [x] T3 `interactive/driver.rs::proto_to_agent()` 补全 7 个变体映射
- [x] T4 `interactive/rpc.rs::run_prompt()` Event 匹配补全新变体
- [x] T5 验证：build + nextest + BDD 全绿 + 新 Event 序列化测试

## P2 — AgentSession 消费 SessionStore/EventSink ports

- [x] T6 `agent/session/io.rs::SessionIO` 从 `SessionManager` 改为 `Arc<dyn SessionStore>`
- [x] T7 `agent/session/mod.rs::AgentSession::new` 接受 `store: Arc<dyn SessionStore>` + `sink: Arc<dyn EventSink>`；loop 内通过 sink port 发射生命周期事件
- [x] T8 `agent/facade.rs::Agent::with_ports` 移除 `session_mgr`，去掉 `_store`/`_sink` 下划线，将 store/sink 传入 AgentSession
- [x] T9 更新所有调用点：`interactive/cli/mod.rs`、`interactive/rpc.rs`、`server/runtime.rs` 适配新签名
- [x] T10 验证：build + nextest + BDD + arch_guard 全绿

## P3 — RPC 模式缓存 Agent

- [x] T11 `interactive/rpc.rs::RpcState` 增加 `cached_agent: Option<Agent>`，`ensure_agent()` 懒初始化 + 复用
- [x] T12 重建触发逻辑：session_id / model_id / thinking_level 变化时重建
- [x] T13 验证：连续 3 个命令只构造一次 Agent，性能提升可测量

## P4 — print 模式事件完备

- [x] T14 `interactive/print.rs::run_print` 补全 TurnStart/TurnEnd/MessageStart/MessageEnd/MessageUpdate/ToolExecutionUpdate/CompactionEnd 渲染
- [x] T15 验证：build + nextest + BDD 全绿

## 收尾

- [x] T16 全量 QA：`cargo build --all-features && cargo nextest run --profile ci && cargo test --test bdd -- --test-threads=1`
- [x] T17 `llman sdd validate c272-complete-core-event-mapping --strict --no-interactive` 通过
