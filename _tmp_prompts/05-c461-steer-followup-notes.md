# c461 升格笔记（探查，非 full）

## pi 真值（`packages/agent` + `coding-agent`）

| API | 语义 |
|---|---|
| `agent.steer(msg)` | 入 steering 队列；**当前 assistant turn 结束后**注入（下一 ReAct 迭代） |
| `agent.followUp(msg)` | 入 follow-up 队列；**agent 本将停止时**再跑 |
| `getSteeringMessages` | 每轮 prepare 时 `steeringQueue.drain()`（可 skip 首次） |
| `getFollowUpMessages` | agent 将停时 `followUpQueue.drain()` |
| mode | `one-at-a-time` \| `all`（settings） |
| UI | 流中 Enter → `prompt(..., { streamingBehavior: "steer" })`；Alt+Enter → followUp |

## xylitol 缺口

| 已有 | 缺失 |
|---|---|
| `SteeringHooks { get_steering_messages, get_follow_up_messages }` | **零调用点**（react 未接） |
| `XyEvent::QueueUpdate { steer_count, follow_up_count }` | 无生产者 |
| settings `steering_mode` / `follow_up_mode` | 未接到 runtime 队列 |
| `Driver::run` / `abort` | 无 `steer` / `follow_up` / `clear_queue` |

## 建议实现形状（升格 full 时）

1. `agent` 内 `PendingMessageQueue`（对齐 pi），挂在 `ReActAgent` 或 session。
2. ReAct 迭代边界调用 `get_steering_messages`；将停前调用 `get_follow_up_messages`（或内建 drain）。
3. 入队/出队发 `QueueUpdate`。
4. `Driver::{steer, follow_up, clear_queue, queue_stats}`；`protocol::Command` 可选加变体供 Remote。
5. abort：是否清空队列 — **建议对齐 pi Esc：abort + restore queued to editor（产品 c480）**；c461 至少定义 `clear_queue` 与 abort 是否联动。

## 参考路径

- `/home/l8ng/Projects/__straydragon__/pi/packages/agent/src/agent.ts`
- `/home/l8ng/Projects/__straydragon__/pi/packages/coding-agent/src/core/agent-session.ts`（`steer`/`followUp`/`_emitQueueUpdate`）
- xylitol：`src/agent/runtime/hooks.rs`、`src/agent/runtime/react.rs`、`src/app/core/driver.rs`、`src/domain/lifecycle.rs`
