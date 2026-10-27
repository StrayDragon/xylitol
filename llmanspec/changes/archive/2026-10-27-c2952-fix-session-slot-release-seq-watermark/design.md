# Design: c2952

## 1. 释放时机

沿用既有写锁释放时机，不新造生命周期事件：

```text
最后一条订阅消失
→ cancel_write_linger
→ clean 或 linger==0：立即 release_write_lock + evict_slot
→ 否则：spawn 定时；到期且 run_inflight==false 且无订阅者 → release_write_lock + evict_slot
```

前置条件与写锁释放完全一致（`run_inflight == false` 且无订阅者），因此不会在回合中途释放。

## 2. 为什么必须保留 seq 水位

`EventJournal::new` 的 `next_seq` 固定从 1 起：

```rust
next_seq: AtomicU64::new(1)
```

而 `replay_from` 在空 buffer 下的判定是：

```rust
if from_seq > 0 && from_seq < min && self.buffer.len() >= self.capacity { return None; }
```

空 buffer 时 `buffer.len() = 0 < capacity`，所以**不会**返回 `None`，而是返回 `Some([])`。也就是说：**空 journal 不发 resync**。

若槽释放时连 seq 一起重置：

```text
客户端 last_seq = 500 → 槽释放 → 重建 seq=1
→ replay_from(500) = Some([]) → 客户端认为 500 之后无遗漏
→ 但服务端下一条事件 seq=1 → 客户端永久错位
```

故槽释放时记录 `next_seq`，重建时续接。这样：

- `r1806`（每会话 seq 单调递增）在槽重建后仍成立；
- `r1798`（last_seq 重放）语义不被破坏——重建后 journal 为空，客户端拿到的空重放是**如实**的（确实没有缓冲事件），后续事件 seq 正确续接。

## 3. 引用环

`materialize_writer_at` 装 `SlotAskGateway { slot: slot.clone() }` 进 driver：

```text
SessionSlot.driver → XyInProcessDriver.ask_gateway → SlotAskGateway.slot → SessionSlot
```

故 `evict_slot` 必须先把 `slot.driver` 置 `None` 才能让 `Arc<SessionSlot>` 归零。`ensure_mcp_resources_watch` 用的是 `Arc::downgrade`，不构成环，无需处理。

## 4. 水位表

`HostState.seq_watermarks: Mutex<HashMap<String, u64>>`，`evict_slot` 写入，`slot()` 读取。取不到时回落 1。

不持久化：Host 重启后 seq 从 1 起是既有行为（journal 本身也不持久化），本 change 不改变跨进程重启语义。

## 5. 测试

| 层 | 测试 |
|---|---|
| journal 单测 | `new_with_seq` 从水位续接、水位下限 1、空 journal + 水位不谎报遗漏 |
| 槽单测 | 释放后重建 seq 续接 |
| BDD 回归 | 重连回放、journal 环形缓冲、写占用四个场景必须仍绿 |

## 6. 不做什么

不动 journal 容量，不动 `MessageUpdate` 载荷，不引入会话删除语义。
