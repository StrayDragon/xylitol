# design — c540 wire / Remote 对齐

## 映射原则

- **闭集优先**：只映射跨面有意义的 `XyEvent`；不引入厂商专名。
- **可降级**：未映射变体 → tracing + 忽略（与 events.md 一致）。
- **队列**：`QueueUpdate` MUST 进 wire（远程 UI 徽章依赖）。

## XyEvent → Event 定稿（task 1）

| XyEvent | Wire | 说明 |
|---|---|---|
| Text/Thinking/Turn/Message/Tool/Model/Compaction/AgentEnd/Error | 已映射 | 保持 |
| **QueueUpdate** | **MUST → `Event::QueueUpdate`** | 远程徽章 / RemoteDriver 流 |
| AgentStart | 降级 | 进程内开场；远程用 Subscribe/Ack |
| AutoRetryStart / AutoRetryEnd | 降级 | 内部重试账本 |
| SessionInfoChanged | 降级 | 可后续加；本变更不扩 |
| ThinkingLevelChanged | 降级 | 以 REST state / 本地缓存为准 |

## RemoteDriver

| Driver 方法 | 远程策略 |
|---|---|
| `run` / `abort` | 已有；保持 |
| `steer` / `follow_up` / `clear_queue` / `queue_stats` | REST；回执含 counts |
| 模型 / 会话 / 导出 / bash / compact / commands | REST；与 Driver 对齐 |

## 验收

- 远程订阅流在 steer 后收到 queue 更新（或等价 Command 回执含 counts）
- `RemoteDriver` 无「not yet implemented」的生产路径（测试 stub 除外）
