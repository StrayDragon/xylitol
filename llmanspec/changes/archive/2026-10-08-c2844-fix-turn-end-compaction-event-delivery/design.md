# 设计：turn-end 压缩事件送达保序 + 摘要 reasoning-only 非空

## 根因（实机 trace `f17c08c8…` 实证）

- 外围 `tokio::select!{ biased; react.next() …; side_rx … }` 主流优先：turn-end overflow 压缩把 CompactionEnd 经 `CompactionStreamTee` 发进侧通道后，主循环在**无穿插 await**（script hook bus 缺省）下直达 `yield AgentEnd` → 下一轮 `react.next()` 即 None→break，side_rx 残留 End 永不消费。server host.rs run 循环与 remote downlink 都在 AgentEnd 截断——产品面只见 Start，折叠块永滞留 Pending。
- 摘要器 `generate_complete` 只收 TextDelta、显式 `{}` 掉 ThinkingDelta → tufa `reasoning.encrypted_content` 纯推理响应当「空」→ fallback 占位。

## 方案

1. **`combine_run_streams`**（自 `build_live_react_stream` 抽出，独立可测）：
   - 主流收 `AgentEnd` 时，先 `try_recv` 清空 side/queue 通道已排队事件（yield 出去）再放行 AgentEnd；主流 None 时同样清尾再终止。
   - 不重排既有顺序（Start 等本就在运行期间经 await 间隙正常送达）；保证 End/ContextTokenSettlement/QueueUpdate 在终态前到达 → 所有以 AgentEnd 截断的消费方零丢失。
2. **摘要器**：`text` 与 `thinking` 双累积；`text` 空但 `thinking` 非空 → 返回 thinking（trim）；两者皆空才 Err。

## 测试

- `combine_run_streams`：End 必须先于 AgentEnd（回归钉死）、内流无终点时清尾（防御）。
- 摘要器：reasoning-only → 返回推理内容非空；全空 → 仍报 empty。
- spike（单测，全 run） + **BDD**：`turn-end-overflow-events-before-agent-end`（真实 AgentRuntime + overflow 错误模型 → Start/End 成对且 End 先于 AgentEnd）与 `reasoning-only-summary-not-empty`（Orchestrator + 纯推理模型 → 摘要采用推理、非 fallback）。

## 明确不做

- 不碰 TUI 折叠渲染（其处理链已正确：收到 End 即收敛）；只修源头送达。
- 不枚举消费方；保序在源头固定。
