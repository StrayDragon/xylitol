# c461 升格调研报告 — Steering/Follow-up 队列

> 基于 xylitol 源码探查 + pi `packages/agent` / `packages/coding-agent` 参考。
> 日期：2026-07-10

---

## 一、pi 参考架构

```
Agent.steer(msg)      → steeringQueue.enqueue(msg)
Agent.followUp(msg)   → followUpQueue.enqueue(msg)

runLoop():
  内层 while (一轮消息注入 + 模型调用 + 工具执行):
    pendingMessages = getSteeringMessages()   // steeringQueue.drain()
    inject pending → history → model call
    工具执行 → tool_calls 为空 → 退出内层
  外层 while (处理 follow-up):
    followUpMessages = getFollowUpMessages()  // followUpQueue.drain()
    有 → pendingMessages = followUpMessages → continue 外层
    无 → break → agent_end

coding-agent-session:
  _emitQueueUpdate()  → QueueUpdate { steer_count, follow_up_count }

UI:
  Enter       → prompt(msg, { streamingBehavior: "steer" })
  Alt+Enter   → followUp(msg)
  Esc         → abort + restore queued to editor
```

关键实现：
- `PendingMessageQueue` 类（`packages/agent/src/agent.ts:123`）：持有 `messages[]`, `mode`（`all` / `one-at-a-time`），提供 `enqueue()` / `drain()` / `clear()` / `hasItems()`
- `agent-loop.ts:runLoop`（约 167/259/263 行）：steering 在内层每轮前 drain，follow-up 在外层停前 drain
- `agent-session.ts:_emitQueueUpdate`（507 行）：queue mutation 后发射事件

---

## 二、xylitol 已有定义 vs 缺口

### ✅ 已存在（定义级完成）

| 位置 | 内容 | 状态 |
|---|---|---|
| `src/agent/runtime/hooks.rs:76` | `SteeringHooks` struct（`get_steering_messages`, `get_follow_up_messages`） | **定义存在，零引用，未 re-export** |
| `src/domain/lifecycle.rs:101` | `XyEvent::QueueUpdate { steer_count, follow_up_count }` | **定义存在，无生产者** |
| `src/infra/settings/types.rs:75-79` | `Settings { steering_mode, follow_up_mode: Option<SteeringMode> }` | **定义存在，未传入 runtime** |
| `src/infra/settings/types.rs:228` | `SteeringMode { All, OneAtATime }` | **定义完整** |
| `src/infra/settings/manager.rs:457-462` | `get_steering_mode()` / `get_follow_up_mode()` | **定义存在，无人调用** |

### ❌ 缺口

| 缺口 | 严重度 | 说明 |
|---|---|---|
| **`PendingMessageQueue` 不存在** | 🔴 blocking | 核心数据结构，需新建 |
| **ReAct 循环未接 queue** | 🔴 blocking | `react.rs:run_react_loop` 不 drain steering/follow-up，不 inject history，不 emit `QueueUpdate` |
| **`Agent` struct 无 queue 字段** | 🔴 blocking | 需要持有 `steer_queue` / `follow_up_queue` |
| **`Driver` trait 缺方法** | 🟡 major | 需加 `steer()` / `follow_up()` / `clear_queue()` / `queue_stats()` |
| **`Protocol::Command` 缺变体** | 🟡 major | 缺 `Steer` / `FollowUp` / `ClearQueue`（Remote 场景需要） |
| **`dispatch.rs` 缺分支** | 🟡 major | 需加对应 dispatch 路由 |
| **`mod.rs` 未 re-export `SteeringHooks`** | 🟢 minor | `src/agent/runtime/mod.rs` 未导出 |
| **`SteeringHooks` 未 thread 进 loop** | 🟢 minor | 定义可作为备选衔接方案，但更推荐直接持有 `PendingMessageQueue` |

---

## 三、建议实现顺序

```
┌─────────────────────────────────────────────────────┐
│ 1. PendingMessageQueue（新建模块 + 单元测试）      │
│    src/agent/runtime/queue.rs                       │
├─────────────────────────────────────────────────────┤
│ 2. Agent struct 加 queue 字段 + steer/followUp/    │
│    clear_queues/queue_stats 方法                    │
│    src/agent/session/mod.rs                         │
├─────────────────────────────────────────────────────┤
│ 3. ReAct 循环接线（每轮前 drain steering，          │
│    停前 drain follow-up，发射 QueueUpdate）         │
│    src/agent/runtime/react.rs                       │
├─────────────────────────────────────────────────────┤
│ 4. Driver trait + InProcessDriver 加队列方法        │
│    src/app/core/driver.rs                           │
├─────────────────────────────────────────────────────┤
│ 5. Command 变体 + dispatch 分支（可选，Remote）     │
│    src/protocol/command.rs + src/app/core/dispatch   │
├─────────────────────────────────────────────────────┤
│ 6. 设置 wiring：settings_mode → PendingMessageQueue  │
│    src/agent/builder.rs（构造时传 mode）            │
└─────────────────────────────────────────────────────┘
```

---

## 四、关键设计决策

| 决策 | 推荐（对齐 pi） | 理由 |
|---|---|---|
| **队列模态** | `Arc<Mutex<PendingMessageQueue>>` | `run_react_loop` 是 async move 闭包 + Driver 外部并发访问 |
| **队列挂在哪里** | `Agent` struct（session 级） | 与 session 生命周期一致 |
| **abort 行为** | 清空 steer_queue，保持 follow_up_queue | 对齐 pi Esc 行为（restore queued messages to editor） |
| **SteeringHooks vs 直持 queue** | 直持 `PendingMessageQueue` | queue 本身是状态对象，不需要函数回调间接层 |
| **mode 传入点** | `Agent::new()` 参数 / `AgentBuilder` | 从 settings 读取，构造时决定 |

---

## 五、参考代码路径

- **pi PendingMessageQueue**: `/home/l8ng/Projects/__straydragon__/pi/packages/agent/src/agent.ts`（123-155 行）
- **pi runLoop steer/followUp**: `/home/l8ng/Projects/__straydragon__/pi/packages/agent/src/agent-loop.ts`（167, 259, 263 行）
- **pi coding-agent steer/followUp**: `/home/l8ng/Projects/__straydragon__/pi/packages/coding-agent/src/core/agent-session.ts`（507, 1294-1332 行）
- **xylitol SteeringHooks**: `src/agent/runtime/hooks.rs`
- **xylitol ReAct loop**: `src/agent/runtime/react.rs`
- **xylitol Agent struct**: `src/agent/session/mod.rs`
- **xylitol Driver**: `src/app/core/driver.rs`
- **xylitol Command**: `src/protocol/command.rs`
- **xylitol dispatch**: `src/app/core/dispatch.rs`
- **xylitol settings mode**: `src/infra/settings/types.rs:75-79,228` / `src/infra/settings/manager.rs:457-462`

---

## 六、前置条件总结

- **无 blocking 依赖**：所有已有定义（`SteeringHooks`/`QueueUpdate`/`SteeringMode`）是纯类型级，无下层依赖
- **`Arc<Mutex<…>>` required**：跨 `run_with_id` 的 async move 闭包 + Driver 外部调用
- **settings wiring**：`Agent::new()` 需接收 `SteeringMode` 参数（builder 传下来）
