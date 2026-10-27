---
depends_on: []
title: journal 只存事实事件，流式 tape 改为回合内瞬时
needs_specs_change: true
---

# journal 只存事实事件，流式 tape 改为回合内瞬时

## Why

会话 journal 当前把**流式增量**与**事实事件**混在同一环形缓冲里，容量 `DEFAULT_JOURNAL_CAPACITY = 10_000`，单条 `MessageUpdate` 携带**完整累积文本**（`react/assistant.rs::streaming_message_update` 走 `partial_assistant_message`）。

后果是二次方级内存放大：8000 字回答、12 字一个 delta ≈ 666 条 `MessageUpdate`，每条平均累积 4000 字 ≈ 7.6 MB；容量 10000 足以装下多次这样的回答。c2952 只解决了「会话槽能否释放」，没有动 journal 内容本身——只要会话活着，这份放大就常驻。

而流式 tape 的生命周期本应与回合一致：`MessageUpdate` / `TextDelta` / `ThinkingDelta` / `ToolExecutionUpdate` 只是**当次回合的实时观感**；回合结束后，客户端需要的 transcript 是**已落库的完整消息**，不是重放几百条累积快照。

## What Changes（候选项，promote 时逐条重估）

- **journal 事件分级**：区分「事实事件」（生命周期、工具结果、完成消息、队列/模型同步等）与「流式 tape」（`TextDelta` / `ThinkingDelta` / `MessageUpdate` / `ToolExecutionUpdate`）。
- **流式 tape 不占 journal 容量**：tape 仅在**回合内**对当前订阅者实时下发，不写环形 journal，或至多保留**每回合一份最新合并快照**（保证中段重连能拿到当前部分状态，而非全量历史快照）。
- **完成即落库**：`MessageEnd` 之后完整消息必须已写入会话存储，使 resume / 切会话可直接用 `get_messages` 重建 transcript，无需依赖 tape 重放。
- **落库先于终结事件**：现状是 `yield MessageEnd` 在前、`persist_agent_message*` 在后（`react/mod.rs` 的 user / assistant / 中断三条路径均如此），存在「客户端已收到完成事件、但磁盘尚未写入」的窗口；重建路径若恰好落在该窗口会读到缺条。须收敛为先落库（或至少在同一 await 序列内保证可读）。
- **恢复路径口径统一**：冷恢复（`last_seq=0` 或无效）继续走 `get_messages` 快照（`r1810` 已如此规定）；中段重连（`last_seq>0`）在 tape 不入 journal 后，须明确「拿到最新合并快照 + 后续实时 tape」还是「直接转观景态」，不得静默丢当前回合内容。
- **兼容与降级**：旧客户端不认识新行为时须可忽略且不 panic；是否 bump 协议版本在 propose 阶段按信封形状变化判定。

## 待决点（propose 时须先定）

1. **中段重连语义**：tape 不入 journal 后，`last_seq>0` 的重连如何补齐当前回合？候选：a) 每回合保留一份最新 `MessageUpdate` 快照入 journal（内存 O(1)/回合）；b) 重连时直接回落到观景态（`r2895`）+ `get_messages`。
2. **工具意图行**：`r1174` 依赖 `MessageUpdate` 的 `message` 做实时 tool 行 upsert；tape 不入 journal 后，重连端在 `ToolExecutionStart` 之前看不到工具意图行，是否可接受（或需在快照里带 tool 意图）。
3. **journal 容量**：tape 摘除后 10000 是否仍合适，还是可下调；与 `resync` 触发频率的权衡。
4. **`MessageUpdate` 载荷形状**：是否保留累积 `text` / `thinking`（端侧目前只在 TUI 用 `message` 做 tool intent，Print 明确不用累积载荷），或改为纯增量。

## Out of Scope

- 会话槽释放与 seq 水位（已由 `c2952` 落地）。
- 会话删除语义、跨进程重启后的 journal 持久化。
- 改动 `get_messages` 的快照形状（除非 promote 时确认必要）。
- 观景态（`r2895`）与 journal 重放定向（`r2896`）的既有语义，除本 change 明确要改的部分。

## Capabilities

- `server-core`：journal 事件分级、tape 生命周期、重连补齐口径、落库与终结事件顺序。
- `agent-runtime`：`MessageEnd` 落库时序；流式事件发射语义不变（实时观感不得退化）。
- `protocol-app`：若信封形状调整，须同步 `XyEvent ↔ Event` 唯一映射与兼容口径。
- `app-tui-bridge`：`r1174` 工具意图 upsert 在重连场景下的降级语义。
- `cli-print`：`r52` 不受影响（Print 本就只消费增量 `TextDelta`）。

## Impact

- crate：`xylitol-serve`（journal 分级/容量/重放）、`xylitol-runtime`（落库时序）、`xylitol-wire` + `xylitol-protocol`（若改形状）、`xylitol-app-tui`（重连降级）。
- 测试：journal 内存/容量边界单测；中段重连 BDD；落库先于终结事件的时序测试；既有「重连回放」「journal 环形缓冲」「观景态」回归。
- 文档：`src/AGENTS.md` 占用段补 journal 分级与 tape 生命周期。

## Further Notes

来源：`c2952` 落地时确认的独立隐患（`sessions` map 只增不删 → 已修；journal 内容放大 → 本 change）。

已核实的事实（2026-10-27）：

| 项 | 事实 |
|---|---|
| tape 事件 | `TextDelta` / `ThinkingDelta` / `MessageUpdate` / `ToolExecutionUpdate` |
| `MessageUpdate` 载荷 | `text` + `thinking` + 可选完整 `message`（累积快照） |
| journal 写入点 | `SessionSlot::append_and_push` 唯一入口，**无事件过滤** |
| 落库点 | `MessageEnd` 之后 `persist_agent_message*`，**晚于事件下行** |
| 冷恢复口径 | `r1810` 已要求 `get_messages` 为投影源，journal 重放不得冒充 |
| 端侧消费 | TUI 只用 `message` 做 tool intent upsert；Print 明确不消费累积 `text` |

本 change 为草案：只记录方向与待决点，落实时走 `llman-sdd-propose`。
