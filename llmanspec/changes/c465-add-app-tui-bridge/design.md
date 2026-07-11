# design — c465 bridge（Track B 开闸后 P0）

> 来源：原 readiness 笔记。**产品 TUI 已开闸（2026-07-11）**；本文件供 apply 使用。

## P0 阻塞

### 1. TUI 必须消费 `Driver::run`

`run(_driver)` 丢弃 Driver → 无 EventStream。c465 本体：host `select!` 合流 Tick + 输入 + **agent 事件**。

### 2. QueueUpdate

c525/c540 后：入队写入活跃 EventStream；wire 亦有 `QueueUpdate`。Bridge **只听 EventStream**（及本地 `queue_stats` 如需即时徽章）。

### 3. Bridge 行为

- 单缝 `apply_xy_event`；未知变体 tracing，不 panic。
- `ToolExecutionEnd`：`name == "edit"` 时解析 JSON 取 `display_diff`（形状脆弱，可后续 typed）。
- 生命周期：中间 `TurnEnd` ≠ 用户轮结束；`AgentEnd` + 无 follow-up 才 idle。

## 已 Ready、仅未接线

- abort 清 steer、留 follow_up（c461）
- `steer` / `follow_up` / `clear_queue` / `queue_stats` on Driver
- `dispatch`（Server 将由 c550 先消费；TUI slash 后续）
- `execute_bash`、fork/switch session

## P1+（非本 change 必做）

- follow_up 正文 peek 或 UI 自持快照
- slash / 会话树 / DESIGN 视觉堆砌（后续 Track B changes）
