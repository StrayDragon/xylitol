# c278-slim-agent-session-and-merge-export Tasks

> 顺序执行；每完成一块跑该块验证。最终 arch_guard 白名单归零。
> 注：`--strict` 为归档闸门（要求 task 全勾选或 defer），实现完成后逐项勾选再过。

## P1 — 合并 export 转发层

- [x] 1.1 尝试将 `infra::session::export` 的纯函数（`render_html`/`render_jsonl`/`parse_jsonl`/`write_to`/`share_guidance_message`）上提到 `core::session_export`（纯转换、无 IO、无 SessionManager 依赖）；若函数体引用 infra 类型则就地迁移到 core
- [x] 1.2 删除 `src/agent/session/export.rs`
- [x] 1.3 `AgentSession::export_to_html`/`export_to_jsonl`/`import_from_jsonl`/`share_as_gist`（`session/mod.rs:804–829`）改为内联：`self.store.load_entries(sid).await?` 后调 `core::session_export::render_*`（或 `infra::session::export::*`，视 1.1 结果）
- [x] 1.4 确认 `interactive/rpc.rs:460` 调用点签名未变，编译通过

  验证：`cargo check --lib`；`rg 'agent::session::export' src/` 零命中

## P2.1 — SessionStore port 扩容（最小集 A）

- [x] 2.1 在 `core::ports::SessionStore` 新增 `load_entries(sid) -> Result<Vec<SessionEntry>, String>` / `append_session_entry(sid, &SessionEntry) -> Result<(), String>` / `build_session_context(sid) -> Result<SessionContext, String>`（类型取自 `core::session_types`，确认已 pub）
- [x] 2.2 `SessionManager` impl 这三个方法（转调既有同名 `load`/`append`/`build_session_context`）
- [x] 2.3 扫描所有 `SessionStore` 的其他 impl（测试 fake/stub），补齐新方法

  验证：`cargo check --lib`；`rg 'dyn SessionStore' src/ tests/` 所有用点编译通过

## P2.2 — 消除 session_manager 具体字段

- [x] 2.4 `AgentSession`：删字段 `session_manager`；删 accessor `session_manager()`（零调用方）
- [x] 2.5 `AgentSession` 内部所有 `self.session_manager.*` 改走 `self.store.*`（`exists`/`load_entries`/`append_session_entry`/`build_session_context`）
- [x] 2.6 `facade.rs::with_ports`：移除 `session_mgr: SessionManager` 参数；组合根（cli/rpc/server）内部构造 `SessionManager` 并 `Arc<dyn SessionStore>` 注入
- [x] 2.7 `compaction/mod.rs::compact_session`：`mgr: &SessionManager` → `store: &dyn SessionStore`；`mgr.load`/`mgr.append` → `store.load_entries`/`store.append_session_entry`
- [x] 2.8 `compaction/orchestrator.rs`：`compact`/`maybe_auto_compact` 的 `session_manager: &SessionManager` → `store: &dyn SessionStore`；`build_session_context` 经 store
- [x] 2.9 `agent/session/io.rs`：确认 wrapper 方法（`load_context`/`append_entry`/`exists`）仅经 store；管理类方法（若经 AgentSession 调用）逐一改组合根直连或上提 port

  验证：`cargo nextest run -p xylitol --lib`；`rg 'crate::infra::session::manager' src/agent/` 零命中

## P2.3 — 消除 event_bus 具体字段 + 死 API

- [x] 2.10 `AgentSession`：删字段 `event_bus: EventBus` 与 `lifecycle_handle: Option<UnsubscribeHandle>`
- [x] 2.11 删除 `src/agent/session/events.rs`（若三个死方法是其全部内容）或移除它们；更新 `session/mod.rs` 的 `use` 与模块声明
- [x] 2.12 `abort()`：sync 签名保留；内部 `event_bus.emit_lifecycle(AgentEnd{aborted})` 改为 `tokio::spawn` fire-and-forget `self.sink.clone().emit(AgentEnd{reason:"aborted"})`（捕获 sink Arc clone + sid）
- [x] 2.13 `dispose()`：移除 `lifecycle_handle.take()`；保留 `abort_bash` + 清队列

  验证：`rg 'crate::infra::event' src/agent/` 零命中；`cargo nextest run -p xylitol --lib`

## P3 — AgentSession 瘦身评估（非阻塞）

- [x] 3.1 评估 `session/stats.rs`、`session/steering.rs` 是否可脱离 AgentSession 字段；改动可控则收敛，否则记 `future.md`
- [x] 3.2 核对 `io.rs` 管理方法调用方；非编排职责的上提 port 或下沉组合根（不强制全做）

  验证：`cargo nextest run -p xylitol --lib`（不引入回归）

## 收尾 — 白名单归零

- [x] 4.1 删除 `src/tests.rs::AGENT_INFRA_ALLOWLIST` 中 c278 的全部 12 条，清空为 `&[]`（保留注释说明 c278 已清空）
- [x] 4.2 更新 API baseline snapshot（`with_ports`/`new`/`abort`/export 方法签名）
- [x] 4.3 跑全套验证：
  ```
  cargo nextest run -p xylitol --lib
  cargo test bdd -- --test-threads=1
  cargo nextest run -p xylitol --lib arch_guard
  rg 'crate::infra::(session|event)' src/agent/   # 期望零命中
  cargo clippy --lib
  cargo fmt --check
  ```
- [x] 4.4 `llman sdd validate c278-slim-agent-session-and-merge-export --strict --no-interactive` 通过（此时 task 已全勾选）
