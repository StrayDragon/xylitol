# c175-refactor-event-system: Tasks

## Event Type Definition

- [x] 定义 `AgentLifecycleEvent` 枚举（所有 pi AgentSessionEvent 变体）
- [x] 包含：agent_start/end, turn_start/end, message_start/update/end, tool_execution_start/update/end, compaction_start/end, model_select, thinking_level_changed, queue_update, auto_retry_start/end, session_info_changed
- [x] 每个变体携带类型化 payload（非泛型 Value）

## EventBus Extension

- [x] 添加 `emit_lifecycle(event: &AgentLifecycleEvent)` 方法到 EventBus
- [x] 添加 `on_lifecycle(handler) -> UnsubscribeHandle` 方法到 EventBus
- [x] 保留现有 string-channel emit/on 用于一般用途
- [x] 重用现有 `UnsubscribeHandle`（drop-based cleanup，通过 `lifecycle:*` 通道）

## Removal of AgentEventBus

- [x] 新建 `src/infra/event/lifecycle.rs` 模块定义 `AgentLifecycleEvent`
- [x] 废弃 `src/agent/event.rs` 模块（已完成）
- [x] 将所有用户迁移到新 EventBus API（已完成）
- [x] 删除 broadcast sender/receiver 代码（已完成）

## AgentSession Integration

- [x] AgentSession subscribe 实现（已完成）
- [x] 订阅方法使用 EventBus.on_lifecycle()（已完成）
- [x] AgentSession 自动处理 message_end 持久化（已完成）
- [x] AgentSession 事件转发（已完成）

## Verification

- [x] `cargo build` — 0 errors
- [x] `cargo test` — 事件系统测试通过
- [x] `llman sdd validate c175-refactor-event-system`
