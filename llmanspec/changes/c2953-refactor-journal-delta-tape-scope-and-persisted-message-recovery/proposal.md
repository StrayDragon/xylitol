---
depends_on: []
title: 流式 tape 不进 journal，历史改由落库快照与分页承载
needs_specs_change: true
---

# 流式 tape 不进 journal，历史改由落库快照与分页承载

## Why

会话 journal 当前把**流式增量**与**事实事件**混在同一个环形缓冲里（`DEFAULT_JOURNAL_CAPACITY = 10_000`），而 `MessageUpdate` 每条都携带**完整累积文本**（`react/assistant.rs::streaming_message_update` → `partial_assistant_message`）。

于是内存随内容二次方增长：8000 字回答、12 字一个 delta ≈ 666 条 `MessageUpdate`，每条平均累积 4000 字 ≈ 7.6 MB；容量 10000 足以装下多次这样的回答。`c2952` 只让会话槽可被释放，没有动 journal 内容本身——只要会话活着，这份放大就常驻。

根因是**职责错配**：journal 的语义是「断线续传的事实事件流」，却被用来承载「回合内的实时观感」。tape 的生命周期本应与回合一致；回合结束后客户端需要的 transcript 是**已落库的完整消息**，不是重放几百条累积快照。

同时暴露三个既有缺口：

- **落库晚于终结事件**：三条路径（user / assistant / 中断）都是先 `yield MessageEnd` 再 `persist_agent_message*`，存在「客户端已收到完成、磁盘尚未写入」的窗口。
- **`MessageUpdate` 三份重复载荷**：`text`（累积全文）/ `thinking`（累积全文）/ `message`（完整 `AgentMessage` 快照）。`message` 是前两者的超集，而 `text` / `thinking` **生产消费方为零**（`cli-print` `r52` 明文禁止 Print 消费累积载荷）。
- **历史无分页**：`Command::GetMessages {}` 无参数、返回全量 entries，长会话会撞 `MAX_BODY_BYTES = 4 MiB` 直接失败；TUI `UiModel.entries` 无上限，每帧 `partition_segments` 全量遍历。

## What Changes

### 1. journal 只存事实事件

事件按生命周期分级：

| 归 journal（事实，不可变） | 归瞬时（tape，回合内） |
|---|---|
| `AgentStart` / `AgentEnd` / `TurnStart` / `TurnEnd` | `TextDelta` / `ThinkingDelta` |
| `MessageStart` / `MessageEnd` | `MessageUpdate` / `ToolExecutionUpdate` |
| `ToolExecutionStart` / `ToolExecutionEnd` | |
| `Compaction*` / `QueueUpdate` / `ModelSelect` / `Error` / `TodoUpdated` / `ContextTokenSettlement` / `AutoRetry*` | |

tape 仍对当前订阅者实时下发（观感不变），但**不占 `seq`、不写 journal**。`seq` 不变量（同一 `seq` 永久指向同一内容）因此完整保留。

### 2. 中段重连经 `subscribe` 应答补当前回合

`SessionSlot` 持 `latest_tape: Mutex<Option<AgentMessage>>`——只存**最后一帧**的累积快照（覆盖式，O(1)），`AgentEnd` 时清空。

`subscribe` 应答（既有 `session_id` / `seq` / `inflight`）新增可选字段 `partial`：仅当 `inflight` 且 `last_seq > 0` 时携带。客户端据此立刻恢复当前回合的文本与工具行，无需新增下行方法、无需重放 tape。

### 3. `MessageUpdate` 收敛为单载荷

删除 `text` / `thinking`，只留 `message`。线协议帧体缩小约 2/3，语义单一（`message` 是唯一真源）。破坏性变更 → `PROTOCOL_VERSION` `4 → 5`；旧端握手失败断开（`r1704` 既有口径，不做降级）。

### 4. journal 容量下调

`DEFAULT_JOURNAL_CAPACITY` `10_000 → 1024`。tape 摘除后典型回合约 14 条事实事件，1024 覆盖约 73 回合，典型内存约 1 MB/活跃会话；配合 `c2952` 的空闲槽释放，常驻进一步下降。

### 5. 落库先于终结事件

三条持久化路径改为**先 `persist_agent_message*` 再 `yield MessageEnd`**，使客户端收到完成事件时磁盘已可读。冷恢复（`r1810`）因此永不会读到缺条。

### 6. `get_messages` 分页

`Command::GetMessages` 增加 `limit` 与 `before_entry_id`：**从最新一条向前取**，`before_entry_id` 指向已加载的最旧条目，用于继续向前翻页。TUI 侧：

- 首屏加载**最新 200 条**（从尾部向前数，符合「打开会话看到最近对话」的直觉）；
- 向上滚动到顶时逐页加载更早 **200 条**；
- 内存窗口上限 **800 条**，超出后最旧部分回落为顶部提示行；
- 提示行居中可点击（`↑ 加载更早的 200 条（还有 N 条）`），复用既有 `UiEntry::ScrollNotice` 形态；**仅在盘上仍有未加载条目时出现**，短会话永不出现。

分页元数据已存在（`SessionManifest.sealed_segments` 的 `first_entry_id` / `last_entry_id` / `generation` 与 `SealedIndex.entry_ids`），不需要新存储格式。

### 7. 钉住两条既有事实

- **折叠/展开是端侧纯呈现**：`activity_fold` 零 Host 往返，`FoldHitTable` + `toggle_fold_target` 全在客户端。须进 spec，防后续误加 Host 依赖。
- **产品 TUI 不得直读 Host 存储**：attach 模式 `session_store()` 返回 `None`，一切经协议 unary。须进 spec，防后续为省一次往返而旁路。

## Out of Scope

- journal 按**字节**记账（最坏情况：连续千条巨型工具结果仍可撞容量上界；根治需改 journal 结构，独立评估）。
- 会话删除语义、journal 跨重启持久化。
- `get_messages` 快照形状变更（除分页参数）。
- 分页参数做成配置项。
- 观景态（`r2895`）与重放定向（`r2896`）的既有语义，除本 change 明确要改的部分。

## Capabilities

- `server-core`：tape 分级、`subscribe.partial`、journal 容量、`get_messages` 分页、落库与终结事件顺序。
- `protocol-app`：`PROTOCOL_VERSION` bump；`MessageUpdate` 字段删除；`GetMessages` 分页参数。
- `agent-runtime`：落库时序；流式事件发射语义不变。
- `app-tui-bridge`：折叠为端侧纯呈现；产品 TUI 不得直读 Host 存储；`r1174` 重连场景改由 `partial` 恢复。
- `app-tui-fixed-zone`：滚动窗口 800 / 页 200 / 顶部加载提示行。
- `cli-print`：`r52` 不受影响（本就只消费增量 `TextDelta`），仅需回归确认。

## Impact

- crate：`xylitol-serve`（journal 分级 / `latest_tape` / `subscribe.partial` / 分页）、`xylitol-runtime`（落库时序）、`xylitol-protocol` + `xylitol-wire`（字段删除与版本）、`xylitol-app-tui`（分页窗口 / 提示行 / `partial` 消费）、`xylitol-kernel`（`GetMessages` 分页透传）。
- 测试：journal 分级单测；`subscribe.partial` BDD；分页边界单测；落库时序测试；既有「重连回放」「journal 环形缓冲」「观景态」「print 无累积载荷」回归。
- 文档：`src/AGENTS.md` 占用段补 tape 分级与 `seq` 不变量；`crates/xylitol-app-tui/src/AGENTS.md` 补端侧折叠与禁止直读。

## Further Notes

来源：`c2952` 落地时确认的独立隐患（`sessions` map 只增不删 → 已修；journal 内容放大 → 本 change）。

已核实事实（2026-10-27）：

| 项 | 事实 |
|---|---|
| journal 写入点 | `SessionSlot::append_and_push` 唯一入口，**无事件过滤** |
| tape 事件 | `TextDelta` / `ThinkingDelta` / `MessageUpdate` / `ToolExecutionUpdate` |
| `MessageUpdate.text` / `thinking` | **生产消费方为零**；`r52` 禁止 Print 消费 |
| `MessageUpdate.message` | TUI `r1174` tool intent upsert 唯一消费 |
| 落库点 | `MessageEnd` **之后** `persist_agent_message*`（三条路径一致） |
| 冷恢复口径 | `r1810` 已要求 `get_messages` 为投影源 |
| 折叠实现 | `activity_fold/` **零** Host 往返 |
| 产品 TUI 存储访问 | `XyRemoteDriver` → `session_store()` = `None` |
| 分页元数据 | `SessionManifest.sealed_segments` + `SealedIndex` 已具备 |
| 单帧上限 | `MAX_BODY_BYTES = 4 MiB` |
| `Event` 反序列化 | 无 `deny_unknown_fields`（新增字段对旧端安全，删除不安全） |

决策（2026-10-27 用户确认）：`PROTOCOL_VERSION` 4→5；journal 容量 1024；分页 首屏 200 / 页 200 / 窗口 800（**从最新一条向前计数**）；中段重连走 `subscribe.partial` 字段而非新增下行方法。
