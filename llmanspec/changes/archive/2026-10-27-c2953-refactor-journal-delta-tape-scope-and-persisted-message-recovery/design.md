# Design: c2953

## 0. 三条硬约束（不可破坏）

| # | 约束 | 理由 |
|---|---|---|
| 1 | **`seq` 不变量**：同一 `seq` 永久指向同一事件内容 | 环形缓冲只能 append。若同槽位被覆盖写，重放拿到的「501」不等于「501 时刻的真实状态」，后续所有依赖 `seq` 的逻辑都要重论证 |
| 2 | **tape 永不进 journal** | journal 语义是「断线续传的事实事件流」，不是「回合内观感」 |
| 3 | **落库先于 `MessageEnd` 下行** | 否则客户端收到完成事件时磁盘可能还没写，冷恢复读到缺条 |

## 1. 事件分级

### 判据

一个事件属于 **事实** 还是 **tape**，只看一条：**它是否描述「已经发生且不可变」的状态**。

| 事件 | 归类 | 判据 |
|---|---|---|
| `AgentStart` / `AgentEnd` | 事实 | 生命周期边界 |
| `TurnStart` / `TurnEnd` | 事实 | 回合边界 |
| `MessageStart` / `MessageEnd` | 事实 | 消息边界（`MessageEnd` 携带最终消息） |
| `ToolExecutionStart` / `ToolExecutionEnd` | 事实 | 工具边界（`ToolExecutionEnd` 携带最终结果） |
| `Compaction*` / `QueueUpdate` / `ModelSelect` / `Error` / `TodoUpdated` / `ContextTokenSettlement` / `AutoRetry*` | 事实 | 状态变更，非增量 |
| `TextDelta` / `ThinkingDelta` | **tape** | 纯增量，可被 `MessageEnd` 完全替代 |
| `MessageUpdate` | **tape** | 累积快照，回合内中间态 |
| `ToolExecutionUpdate` | **tape** | 工具输出增量，可被 `ToolExecutionEnd` 完全替代 |

### 实现点

`SessionSlot::append_and_push` 是唯一写入入口。改为：

```rust
pub async fn append_and_push(&self, event: &XyEvent) {
    if is_tape_event(event) {
        self.push_tape(event).await;   // 广播 + 更新 latest_tape，不碰 journal
        return;
    }
    // 事实事件：原有路径（append journal → broadcast）
}
```

`is_tape_event` 用穷举 `match`，不用字符串判断。

## 2. `latest_tape` 与 `subscribe.partial`

```rust
latest_tape: Mutex<Option<AgentMessage>>,
```

- `push_tape` 时：`MessageUpdate { message: Some(m) }` → 覆盖写入；其他 tape（`TextDelta` / `ThinkingDelta` / `ToolExecutionUpdate`）不单独存，因为它们的累积态已体现在最近一条 `MessageUpdate.message` 里。
- `AgentEnd` 到达时清空（回合结束，快照失效）。
- 覆盖式写入 → **O(1) 内存**，与回合长度无关。

`SessionSubscribedPayload` 增加：

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub partial: Option<Value>,
```

填充条件：`inflight && last_seq > 0`。`last_seq == 0`（冷订阅）不填——冷订阅本来就该走 `get_messages` 拿完整 transcript（`r1810`），不需要半截快照。

### 为什么不新增下行方法

| 项 | 新增方法 | `subscribe.partial`（选定） |
|---|---|---|
| 协议面 | +1 方法 + 1 payload 类型 | +1 可选字段 |
| 客户端改动 | 新增 `decode_downlink` 分支 | 已有 `inflight` 解析处加一行 |
| 时序 | 订阅成功后还要等一帧 | 应答即达，无竞态 |
| 兼容 | 旧端忽略未知通知 | 旧端忽略未知字段（`Event` 无 `deny_unknown_fields`，已核实） |

## 3. `MessageUpdate` 字段删除

```rust
// before
MessageUpdate { text: String, thinking: Option<String>, message: Option<AgentMessage> }
// after
MessageUpdate { message: Option<AgentMessage> }
```

`message` 已含 `AgentPart::Thinking` / `Text` / `ToolCall` 全部信息，是超集。

`PROTOCOL_VERSION` `4 → 5`。旧端反序列化缺 `text` 会失败 → 握手 `host.describe` 版本不匹配 → 按 `r1704` 断开，不降级。

**升级路径**：Host 与 TUI 同二进制发布，无跨版本共存场景，故不需要 `migrations/` 脚本；但须在 proposal 与本 design 记录「这是破坏性变更、无降级路径」这一事实。

## 4. journal 容量 1024

### 密度测算

```
每回合事实事件 = AgentStart + TurnStart
                + MessageStart(user) + MessageEnd(user)
                + MessageStart(asst) + MessageEnd(asst)
                + N × (ToolExecutionStart + ToolExecutionEnd)
                + TurnEnd + AgentEnd
              = 10 + 2N
```

| 场景 | N | 条/回合 | 1024 覆盖回合数 |
|---|---|---|---|
| 纯对话 | 0 | 10 | 102 |
| 典型（2 工具） | 2 | 14 | 73 |
| 重工具（8 工具） | 8 | 26 | 39 |

### 取舍

| 候选 | 覆盖回合 | 典型内存 | 问题 |
|---|---|---|---|
| 512 | 20-51 | ~0.5 MB | 长会话断线频繁 resync，重画更贵 |
| **1024** | **39-102** | **~1 MB** | **选定** |
| 4096 | 157-409 | ~4 MB | 无对应收益；真正长断线本就该走 `get_messages` |
| 10000 | 384-1000 | ~10 MB | 现状，二次方放大源头 |

**已知上界缺陷**：连续 1024 条 × 50 KiB `ToolExecutionEnd.result` ≈ 50 MB。这是病态输入，本 change 不做字节记账（见 Out of Scope）。

## 5. 落库时序

### 现状（三条路径）

```
yield MessageEnd { message }
persist_agent_message*  // 晚一步
```

### 改为

```
persist_agent_message*
yield MessageEnd { message }
```

涉及 `react/mod.rs`：
- user pending 消息循环（`~1083`）
- assistant 正常完成（`~1551`）
- assistant 中断（`~1321`）
- assistant middleware stop（`~1505`）

### 风险

落库失败时（`persist_agent_message` 内部只 `log::warn!`），改序后仍会 `yield MessageEnd`。行为不变——只是把「窗口」关掉，不引入新的失败语义。

## 6. `get_messages` 分页

### 协议形状

```rust
Command::GetMessages {
    #[serde(default)] limit: Option<usize>,
    #[serde(default)] before_entry_id: Option<String>,
}
```

语义：

```
limit 缺省 → 全量（向后兼容，但 TUI 不这么用）
limit=N, before_entry_id=None → 取最新 N 条
limit=N, before_entry_id=Some(id) → 取 id 之前（更早）的 N 条
```

**从最新一条向前取**，符合「打开会话看到最近对话」的直觉。

### 存储侧实现

`load_entries` 已是「sealed segments 顺序拼接 + active segment」。分页只需：

1. 从 `manifest.sealed_segments` 尾部 + `active_segment` 反向定位；
2. 用 `first_entry_id` / `last_entry_id` 跳过整段；
3. 只 `read_segment` 命中的段。

`SealedIndex.entry_ids` 可用于段内二分（可选优化，非必需）。

### TUI 侧

| 参数 | 值 |
|---|---|
| 首屏 | 最新 200 条 |
| 单页 | 200 条 |
| 内存窗口 | 800 条 |
| 触发门槛 | 盘上还有未加载条目时才出现提示行 |

```
打开会话 → get_messages(limit=200)            → 显示最近 200 条
向上滚到顶 → get_messages(limit=200, before=最旧 id) → 前置 200 条
窗口达 800 → 最旧 200 条替换为 ScrollNotice「↑ 加载更早的 200 条（还有 N 条）」
```

提示行居中、可点击，复用 `UiEntry::ScrollNotice` + `FoldTarget` 新增一个 `LoadEarlier` 变体。

**为什么不把窗口做成配置**：配置项 = schema + 文档 + 测试 + 维护成本。个人 harness 的会话长度分布高度同质，固定值够用。

## 7. 两条既有事实进 spec

### 折叠为端侧纯呈现

证据：`activity_fold/` 内零 `unary` / `Command::` / `host.` 调用；状态在 `ActivityFoldState` / `ScrollbackFold`；点击命中在 `FoldHitTable`；处理在 `UiRoot::toggle_fold_target`。

规则：**折叠/展开须为端侧纯呈现，不得往返 Host，不得要求 Host 记住折叠状态**。理由：折叠是视图状态，不是业务状态；往返会引入延迟与跨端不一致。

### 产品 TUI 不得直读 Host 存储

证据：`XyRemoteDriver` 经 `proto.rs:107` → `session_store()` 返回 `None`；`src/cli.rs:243` 产品 TUI 用 `XyRemoteDriver::new(attach_url, ...)`。embed / harness 走 in-process，允许直读（那是嵌入方特权）。

规则：**产品 TUI（attach 模式）不得直接访问 Host 会话存储，一切经协议 unary**。理由：跨进程边界是硬边界；旁路会让「Host 是唯一权威」失效。

## 8. 测试边界（seam）

| seam | 层 | 覆盖 |
|---|---|---|
| `EventJournal::append` / `replay_from` | 单测 | 容量 1024 环绕、seq 单调 |
| `SessionSlot::append_and_push` | 单测 | tape 不写 journal、事实事件写 journal |
| `subscribe` unary 应答 | BDD | `partial` 仅在 inflight+last_seq>0 出现 |
| `get_messages` unary | BDD | 分页边界（首屏/翻页/末尾） |
| `persist` → `MessageEnd` 顺序 | 单测 | 落库先于事件 |
| `render_scrollback` 窗口 | 单测 | 800 上限 + 提示行 |
| `MessageUpdate` 序列化 | 单测 | 只含 `message` |

## 9. 迁移与兼容

| 项 | 处置 |
|---|---|
| `PROTOCOL_VERSION` | 4 → 5；旧端握手失败断开（`r1704`） |
| `MessageUpdate.text` / `thinking` | 直接删除，无别名、无双解析路径 |
| `GetMessages` | 新参数带 `#[serde(default)]`，旧调用形状仍可用 |
| journal 容量 | 常量替换，无持久化影响（journal 本就不落盘） |
| 会话存储格式 | **不变**（分页只读现有 manifest/segment） |
