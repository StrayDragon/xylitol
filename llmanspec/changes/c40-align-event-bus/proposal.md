---
depends_on: []
---

# c40-align-event-bus: 对齐 pi EventBus（channel-based）

## Why
当前 xylitol 的 `AgentEventBus` 使用类型化枚举，与 pi 的 channel-based 模型不兼容，无法支持扩展系统。pi 使用 string channels 作为事件命名空间。

## What Changes
- **新增** `src/infra/event/mod.rs`：channel-based `EventBus`
- **删除** 旧的 `src/agent/event.rs` 全部代码（`AgentEventBus` + `AgentEvent` 枚举）
- 所有 emit/subscribe 调用点直接使用新 API

## Capabilities
- agent-session

## Impact
- 旧 `AgentEvent` 枚举完全删除
- 旧 `AgentEventBus` 完全删除
- 事件类型不再用枚举，改用 `(channel: &str, payload: Value)`
