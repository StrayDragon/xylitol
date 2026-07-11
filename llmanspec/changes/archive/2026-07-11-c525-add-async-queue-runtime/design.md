# design — c525 异步可并发队列运行时

> 产品语义 SSOT：[`docs/architecture/queue-and-interrupt.md`](../../../../docs/architecture/queue-and-interrupt.md)。
> 本文只写**实现选型与落地**。

## 1. 选型结论（灵活 · 性能 · 体验 · 可维护）

在个人 coding agent 规模下（队列深度通常个位数），采用：

**推荐方案：每通道 `tokio::sync::Mutex<VecDeque<T>>` + 短临界区 + 入队/清空后向「当前活跃 run」的事件发送端投递 `QueueUpdate`。**

| 维度 | 为何选它 |
|---|---|
| **灵活** | 通道按名（steer / follow_up）隔离；`QueueMode`（全部 / 一次一条）在 drain 时解释；以后加通道只加实例 |
| **性能** | 队列极短；锁持有仅微秒级 push/pop/clear；不引入无界增长；避免为小队列上 mpsc+watch 双机构 |
| **体验** | 入队**立刻**往活跃 EventStream 发 `QueueUpdate`（修双通道）；UI 与对话进度同一条流 |
| **可维护** | 一种结构、两种实例；drain-all / drain-one / clear / len 语义直观；比 mpsc「拉空」+ 另挂 stats 更少状态机 |

### 明确不选（本阶段）

| 方案 | 原因 |
|---|---|
| 每通道 `mpsc` + `watch` stats | 灵活过度；`OneAtATime` / clear / len 要额外状态；收益在本产品深度下不明显 |
| 无界 channel / 工作窃取池 | 过重；非目标 |
| 继续 EventBus 旁路发 QueueUpdate | 体验坑（bridge 听不见） |

### 演进阀值（写进 NOTE，勿预支）

若出现：持续高并发生产者、或单通道深度经常 > 几十且持锁可测变慢 → 再评估「有界 mpsc + watch」。在此之前 **MUST NOT** 为想象负载上双机构。

## 2. 结构

```text
AsyncQueueRuntime
  ├── steer:  QueueChannel { mode, buf: Mutex<VecDeque<Item>>, events: EventTx }
  └── follow_up: 同上（独立 buf / mode）

EventTx = 当前 Driver::run 绑定的 XyEvent 发送端（无活跃 run 时可 no-op 或仅更新可读 stats）
```

- **多生产者**：`enqueue` 短锁 push + unlock 后 `try_send(QueueUpdate)`。
- **单消费者**：ReAct 在约定时机 `drain`（按 mode）。
- **abort**：`cancel_token.cancel()` + `steer.clear()`；**不清** `follow_up`。

## 3. 必须保持的产品语义

| 语义 | 实现约束 |
|---|---|
| 插话消化时机 | 每轮模型调用**前** `steer.drain()` |
| 续跑消化时机 | 将停（无 tool_calls）**前** `follow_up.drain()` |
| abort | 清 steer；保留 follow_up；取消进行中工具 |
| QueueUpdate | **仅**经活跃 EventStream（或 run 绑定的同构通道）；禁止只写无人订 EventBus |

## 4. API 形状（实施可微调命名）

```rust
struct QueueStats { steer_count: usize, follow_up_count: usize }

impl QueueChannel {
    async fn enqueue(&self, item: Item);
    async fn drain(&self) -> Vec<Item>; // 尊重 QueueMode
    fn clear(&self);
    fn len(&self) -> usize;
}
```

`Driver::queue_stats() -> QueueStats`（替换裸元组，可与 c500 命名债一并）。

## 5. 迁移步骤

1. 引入 `QueueChannel` / `AsyncQueueRuntime`；行为单测（并发 enqueue、mode、abort）。
2. 接线：入队/clear → 活跃 EventStream 的 `QueueUpdate`；删除生产路径对 EventBus 队列旁路的依赖。
3. `Agent` / `Driver` / ReAct 改用新类型；保持 c461 语义测试全绿。
4. 文档：产品侧只维护 `docs/architecture/queue-and-interrupt.md`；实现变更只改本 design + 代码。

## 6. 与其它 change

| Change | 关系 |
|---|---|
| c461 | 语义不变；本 change 换实现 |
| c465 | 开闸 P0 依赖「入队即时可见」——本 change 或同 PR 修双通道 |
| c500 | `QueueStats` 可一并 |
| c515 | MCP 重载不共用消息通道；可借鉴「可取消编排」思路另议 |
