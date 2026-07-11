# design — c540 wire / Remote 对齐

## 映射原则

- **闭集优先**：只映射跨面有意义的 `XyEvent`；不引入厂商专名。
- **可降级**：未映射变体 → tracing + 忽略（与 events.md 一致）。
- **队列**：`QueueUpdate` MUST 进 wire（远程 UI 徽章依赖）。

## RemoteDriver

| Driver 方法 | 远程策略 |
|---|---|
| `run` / `abort` | 已有；保持 |
| `steer` / `follow_up` / `clear_queue` / `queue_stats` | REST 或 WS command |
| 模型 / 会话 / 导出 | REST；与 dispatch 对齐 |

## 验收

- 远程订阅流在 steer 后收到 queue 更新（或等价 Command 回执含 counts）
- `RemoteDriver` 无「not yet implemented」的生产路径（测试 stub 除外）
