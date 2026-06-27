# c278 Future / Deferred Items

## P3 — AgentSession 瘦身评估（c278 内评估，结论：保留现状）

评估结论（2026-06-27）：

- **`session/stats.rs` / `session/steering.rs`**：是 `impl AgentSession` 的方法模块
  （分文件组织，非字段耦合）。保留在 `agent/session/` 是合理的职责分组，无需迁移。
- **`session/io.rs`**：`SessionIO` 已是纯 `Arc<dyn SessionStore>` 转发 wrapper
  （`load_context`/`append_entry`/`exists` 全部经 port）。无需改动。

c278 已完成的瘦身：
- 删除 `session_manager: SessionManager` 字段（→ `Arc<dyn SessionStore>` port）
- 删除 `event_bus: EventBus` + `lifecycle_handle` 字段（死代码，零订阅者）
- 删除 5 个零调用方法：`start_new_session` / `resume_session` / `navigate_tree`
  / `switch_session` / `send_custom_message` + `session_manager()` accessor
- 删除 `agent/session/events.rs` 整模块（8 个死方法）+ `agent/session/export.rs` 转发层

## 后续候选（later，触发后再开）

- **abort lifecycle 通知**：`abort()` 当前不 emit 任何事件（旧 EventBus 零订阅者，emit
  是死操作已移除）。若未来需要 abort 通知，扩 `core::ports::LifecycleEvent`（目前仅
  CompactionStarted/Ended）并经 `EventSink` port 发射。触发信号：有 sink 消费者
  需要感知 abort。
- **`Driver::abort` async 化**：当前 `abort()` sync（fire-and-forget 受限）。若需可靠
  的 async lifecycle 投递，需把 `Driver::abort` trait method 改 async（波及
  interactive trait 契约）。单独提案。
- **`SessionManager` 管理操作 port 化**：`fork`/`create` 已上提 `SessionStore`（最小集）。
  其余管理操作（`navigate_tree`/`switch_session`/`get_tree`）对应的方法在 c278 中被
  删除（零调用）。若未来需要会话树管理 UI，重新设计这些操作的 port 边界。
