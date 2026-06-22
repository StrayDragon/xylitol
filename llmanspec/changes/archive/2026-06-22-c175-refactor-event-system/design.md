# Design: c175-refactor-event-system

## Approach

1. 保留现有 `EventBus`（channel-based）作为底层基础设施
2. 在 `EventBus` 上添加类型安全的生命周期事件 API，而不是替换它
3. 新增 `AgentLifecycleEvent` 枚举承载所有代理生命周期事件
4. `AgentSession` 在内部通过 `EventBus` 订阅并转发给监听者
5. 逐步废弃 `AgentEventBus`（broadcast），最终移除

## Architecture

```
AgentLoop               AgentSession               EventBus (channel)
   │                        │                          │
   │── emit_lifecycle() ──► │── on_lifecycle() ──────► │── handler1 (persist)
   │                        │                          │── handler2 (forward to listeners)
   │                        │── emit() ──────────────► │── handler3 (extensions)
```

## Key Decisions

- `AgentLifecycleEvent` 是 typed enum，不是泛型 Value（类型安全）
- `UnsubscribeHandle` 通过 drop 自动清理（RAII 模式）
- 所有生命周期事件走同一通道：持久化、扩展、UI 监听者都从同一事件源消费
