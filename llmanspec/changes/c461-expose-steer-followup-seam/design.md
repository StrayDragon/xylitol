# Design — c461 steer / follow-up seam

## 背景

对齐 pi `PendingMessageQueue` + `runLoop`，打通 xylitol 已有但未接线的类型骨架。

## 决策

### D1 — 直持 queue，不以 SteeringHooks 为唯一路径

`SteeringHooks` 的函数回调适合「外部注入消息源」，但产品需要的是 **可变队列状态**（UI 随时 `steer`/`follow_up`）。采用 `Agent` 持有 `PendingMessageQueue`（steer + follow_up 各一）。hooks 若保留，仅作可选扩展，不得挡住直持路径。

### D2 — 挂载与并发

- 队列挂在 **session 级 `Agent`**，生命周期与会话一致。
- `run_react_loop` 为 async move；外部 `Driver::steer` 并发入队 → 使用 `Arc<Mutex<PendingMessageQueue>>`（或 `tokio::sync::Mutex`，以实现时编译约束为准；优先与现有 cancel/token 模式一致）。

### D3 — drain 时机（对齐 pi）

```text
内层（单次用户可见 run / 续跑）:
  每轮模型调用前: drain(steering) → 注入 user messages → model → tools
  tool_calls 空 → 准备结束
外层 / 将停前:
  drain(follow_up)；若非空 → 作为新一轮 pending 继续
  皆空 → AgentEnd
```

中间 `TurnEnd` 仍只表示 ReAct 迭代边界（见已归档 c450 design D4）。

### D4 — abort 与队列（用户锁定 3A）

| 动作 | steer 队列 | follow_up 队列 |
|---|---|---|
| `Driver::abort` | **清空** | **保留** |
| `clear_queue`（显式） | 可清一侧或两侧 | 可清一侧或两侧 |
| UI Esc（c480） | 调 abort；将保留的 follow_up（及可选 steer 快照）restore 到编辑器 | 由 UI 负责文案合并 |

理由：对齐 pi Esc「abort + restore queued to editor」；runtime 不清 follow_up，避免 UI 无法恢复。

### D5 — mode

`SteeringMode::{All, OneAtATime}` 已在 settings；构造时写入两个 queue 的 mode。默认与 pi 一致倾向 `one-at-a-time`（以实现时 settings 默认值为准）。

### D6 — 协议

为 RemoteDriver / 日后远控增加 `Command::{Steer, FollowUp, ClearQueue}`（命名可微调但语义固定）。`Prompt`/`Quit`/`Subscribe` 等仍由调用方特殊处理；新变体走 `dispatch`。

### D7 — QueueUpdate

入队、drain、clear、abort 清 steer 后均应发射 `XyEvent::QueueUpdate { steer_count, follow_up_count }`，便于 TUI status/chrome（后续 change）。

## 权衡

- **为何不把 queue 只放在 app 面**：ReAct 必须在迭代边界注入，属 agent 语义，不是 UI 缓冲。
- **为何 abort 不清 follow_up**：产品要 restore；若 abort 丢队列，Alt+Enter 排队在取消后无法找回。
