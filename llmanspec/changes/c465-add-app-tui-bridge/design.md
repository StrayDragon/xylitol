# design — c465 bridge 开闸前 P0（自 readiness 并入）

> 来源：原 `docs/tui-research/2026-07-10-src-core-tui-readiness.md`（已删）。
> 产品 TUI 仍冻结；本文件供开闸后 apply 使用。

## P0 阻塞

### 1. TUI 必须消费 `Driver::run`

`run(_driver)` 丢弃 Driver → 无 EventStream。c465 本体：host `select!` 合流 Tick + 输入 + **agent 事件**。

### 2. QueueUpdate 双通道（必须修）

```text
入队/清队 ──► EventBus.emit_lifecycle ──► 生产路径无人订
drain 后  ──► react yield QueueUpdate ──► Driver::run 流 ✅
```

只听 EventStream 的 bridge **看不到**流中 Enter 入队的即时 `steer:N`。

**方向**：入队/abort 清队也写入**当前活跃 run** 的事件通道（产品语义见 [`docs/architecture/queue-and-interrupt.md`](../../../../docs/architecture/queue-and-interrupt.md)；实现见 c525 design）。勿再依赖无人订阅的 EventBus 旁路。

### 3. Bridge 行为

- 单缝 `apply_xy_event`；未知变体 tracing，不 panic。
- `ToolExecutionEnd`：`name == "edit"` 时解析 JSON 取 `display_diff`（形状脆弱，可后续 typed）。
- 生命周期：中间 `TurnEnd` ≠ 用户轮结束；`AgentEnd` + 无 follow-up 才 idle。

## 已 Ready、仅未接线

- abort 清 steer、留 follow_up（c461）
- `steer` / `follow_up` / `clear_queue` / `queue_stats` on Driver
- `dispatch` 映射（产品零调用）
- `execute_bash`、fork/switch session

## P1+（非本 change 必做）

- follow_up 正文 peek 或 UI 自持快照
- Compaction/AutoRetry/AgentStart 进流或删死类型
- 会话树 Driver list/branch
- `Driver::EventStream` 与 crossterm 撞名 → 见 c500/命名债
