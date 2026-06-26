# Tasks: c265-add-server-runtime

> 范围：HC-2 port 落地 + Driver 抽象（server 进程化本身在 c270）。每阶段后全测试套件绿灯才进下一阶段。

## P1 — HC-2 port 定义 + 实现（行为不变）

- [x] T1 在 `core/ports.rs` 新增 `SessionStore` trait（load_context, append, exists）——仅 agent loop/compaction 调用的方法，不抄 SessionManager 全量 surface
- [x] T2 在 `core/ports.rs` 新增 `EventSink` trait（emit_lifecycle）
- [x] T3 `infra::session::SessionManager` impl `SessionStore`（load_context 委托 build_session_context, append 委托 append_entry, exists 委托 exists）
- [x] T4 `infra::event::EventBus` impl `EventSink`
- [x] T5 新增 `tests/support/in_memory_store.rs`：InMemorySessionStore（HashMap-based，用于 agent 单测）
- [x] T6 新增 `tests/support/recording_sink.rs`：RecordingSink（Vec<AgentLifecycleEvent>，用于 agent 单测）
- [x] T7 改 `agent/facade.rs`：`Agent::new(store: Arc<dyn SessionStore>, sink: Arc<dyn EventSink>, ...)`；`run(&mut self, prompt: &str)` 不再传 session_id
- [x] T8 改 `interactive/cli` 组合根：构造 `Arc<dyn SessionStore>` / `Arc<dyn EventSink>` 注入 Agent
- [x] T9 改 `interactive/rpc` 的 `run()` 改为接收 Agent 或 InProcessDriver（port 已由 cli 注入）
- [x] T10 验证：`Agent::new` 签名无 SessionManager/EventBus；build + nextest + clippy + BDD 全绿

## P2 — Driver 抽象（行为不变）

- [x] T11 新建 `interactive/driver.rs`：`Driver` trait（send + subscribe）+ `InProcessDriver`（持有 Arc<dyn SessionStore> + Arc<dyn EventSink> + ...，获取 Agent）
- [x] T12 `interactive/{print,rpc,cli}` 的运行时改为用 `InProcessDriver`
- [x] T13 验证：interactive 代码接 InProcessDriver 前后事件序列不变；BDD 全绿

## P3 — 架构断言扩展

- [x] T14 扩展 `src/tests.rs::arch_guard`：新增 `interactive_only_from_driver` 测试（interactive/ 文件仅 driver.rs 可 import agent/infra）
- [x] T15 更新 arch_guard 的 NOTE 移除"待 P5 port 落地后启用"说明
- [x] T16 验证：arch_guard 全 3 个测试绿；全量 QA 绿

## 收尾

- [x] T17 `llman sdd validate c265-add-server-runtime --strict --no-interactive` 通过
- [x] T18 全量校验：cargo build --all-features && cargo nextest run --profile ci && cargo clippy --all-features -- -D warnings && cargo test --test bdd -- --test-threads=1 && cargo fmt --check
