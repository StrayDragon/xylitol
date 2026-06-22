---
id: c175-refactor-event-system
title: "Refactor Event System — unify AgentEventBus and EventBus into event-driven AgentSession lifecycle"
depends_on: []
---

## Why

当前存在两套事件系统：

1. `AgentEventBus`（broadcast channel）— 用于 Agent 事件，但从未被 AgentSession 真正集成
2. `EventBus`（channel-based + tokio::spawn）— 用于扩展间通信，但也不被 AgentSession 消费

这导致：turn_start/turn_end 等生命周期事件没有订阅者、session 持久化没有事件驱动、重试没有事件通知。

pi 使用 `agent.subscribe()` + `_handleAgentEvent()` 模式：Agent 核心发出原始事件 → AgentSession 订阅并处理持久化/扩展/重试 → 再转发给 UI 监听者。

## What Changes

1. 废弃 `AgentEventBus`（broadcast），统一使用 `EventBus`（channel-based）作为底层传输
2. 重新定义 `AgentLifecycleEvent` 类型枚举（turn_start/message_start/…等 15+ 事件）
3. 在 `EventBus` 上添加类型安全的事件调度（`emit_lifecycle()` / `on_lifecycle()`）
4. 保留 `AgentEventBus` 的 drop-based unsubcribe 模式（对应的 UnsubscribeHandle）
5. 移除旧 AgentEvent（loop.rs 中的冗余枚举），改为使用 AgentLifecycleEvent
6. 所有 Agent 事件走 EventBus → AgentSession 订阅 → 持久化/扩展/转发

## Capabilities

- event-system

## Impact

- `src/infra/event/mod.rs`：扩展 EventBus 添加生命周期事件支持
- `src/agent/event.rs`：废弃 AgentEventBus，路由到新 EventBus
- `src/agent/loop.rs`：事件类型替换
- `src/agent/session.rs`：添加事件订阅 + 持久化处理

## Definition of Done

- [ ] `AgentEventBus` 完全移除
- [ ] `AgentLifecycleEvent` 枚举定义（15+ 变体）
- [ ] `EventBus.emit_lifecycle()` / `on_lifecycle()` 类型安全 API
- [ ] UnsubscribeHandle（drop-based unsubscribe）
- [ ] `cargo test` 通过
