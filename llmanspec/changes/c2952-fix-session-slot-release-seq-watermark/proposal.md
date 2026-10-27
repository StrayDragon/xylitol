---
depends_on: []
title: 会话槽释放与事件序号水位续接
---

# 会话槽释放与事件序号水位续接

## Why

`HostState.sessions` 是**只增不删**的 map：唯一一处写操作是 `entry().or_insert_with()`。`SessionSlot` 也没有 `Drop`，`on_last_subscriber_gone` 只释放**写锁**，槽本身永久留在 map 里。

后果是每会话常驻内存不回收，且被两处放大：

- `EventJournal` 容量 **10000 条**（`DEFAULT_JOURNAL_CAPACITY`）；
- `MessageUpdate` 每次流式 delta 都发一条，且**携带完整累积文本**（`react/assistant.rs`），单次长回答的 journal 累计可达数 MB（二次方增长）。

估算：8000 字回答 / 12 字一个 delta ≈ 666 条 `MessageUpdate`，平均每条累积 4000 字（12 KB），单次回答 journal ≈ 7.6 MB。容量 10000 条意味着能装下多次这样的回答，**常驻内存随会话数线性增长**。

## What Changes

- 会话槽在**最后一条订阅连接消失且宽限到期**后从 `sessions` map 移除（与既有写锁释放同一时机），并清空其 driver 以断开 `SessionSlot → driver → SlotAskGateway → SessionSlot` 的引用环。
- **事件序号水位保留**：槽释放时把该会话的 `next_seq` 记入 `HostState` 水位表；同会话重建槽时从水位续接，不得回到 1。
- 前置条件沿用既有：`run_inflight == false` 且 `live_subscriber_count == 0`，不得在回合进行中释放。

## Out of Scope

- 降低 `EventJournal` 容量。
- 改 `MessageUpdate` 载荷形状（不再携带累积全文）——那是协议行为变更，独立评估。
- 会话删除语义（`delete_session`）：槽释放不等于删除会话数据。

## Capabilities

- `server-core`：新增会话槽释放与序号水位续接 MUST + 场景。

## Impact

- crate：`xylitol-serve`（槽释放、水位表、journal 构造）。
- 测试：journal 水位单测；槽重建 seq 续接单测；既有「重连回放」「journal 环形缓冲」「写占用」BDD 回归。
- 文档：`src/AGENTS.md` 或 `crates/xylitol-serve/src/AGENTS.md` 补槽生命周期。

## Further Notes

来源：c2946 落地时发现的独立隐患（`sessions` map 只增不删）。

关键约束（实现时确认）：`EventJournal::new` 的 `next_seq` 固定从 1 起，且 `replay_from` 在空 buffer 下返回 `Some([])` 而非 `None`。若槽释放时连 seq 一起重置，客户端带 `last_seq=500` 重连会拿到空重放且**不发** `session/resync_required`，事件流永久错位（违反 `r1798` / `r1806`）。故必须保留水位。
