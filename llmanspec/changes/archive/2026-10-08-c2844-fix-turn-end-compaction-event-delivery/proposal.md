---
depends_on: []
needs_specs_change: true
branch: pr/2026-10-bdd-infra-and-contracts
base_branch: main
base_sha: dc379efb8430dae278ffc66c7bdd7e56b3faa202
---

# 修复 turn-end 压缩事件送达与摘要 reasoning-only 判空

## Why

实测缺陷（served 拓扑实机 + Langfuse trace `f17c08c8…` 实证）：

1. **turn-end 压缩的 CompactionEnd 被流尾饿死丢弃**：`run_react_loop` 外层 `tokio::select!{ biased; react.next() …; side_rx … }` 在主流优先下，收尾压缩把 CompactionEnd 发进侧通道后**无穿插 await 直达 AgentEnd**（script hook bus 缺省）→ `react.next()` 到 None 即 break，侧通道残留事件永不消费。消费方（server run loop / remote downlink）都在 AgentEnd 处截断 → 产品面只见 CompactionStart，TUI 折叠块永久滞留 `[compaction] Compacting…`。
2. **摘要响应纯 reasoning 判空降级**：`generate_complete` 只累积 TextDelta，显式丢弃 ThinkingDelta → tufa `reasoning.encrypted_content` 模式下 2036 token 纯推理输出被判「empty response」→ 落到 `[Turn prefix: N entries]` fallback 占位。

## What Changes

1. `combine_run_streams`（抽自 build_live_react_stream，可测化）：**保序冲刷**——主流收 AgentEnd / 结束时，先把侧/队列通道已有事件 yield 出去再放行终态事件，保证 CompactionEnd/ContextTokenSettlement/QueueUpdate 在 AgentEnd 前送达，任何以 AgentEnd 截断的消费方都不丢尾。加 2 条单测钉顺序。
2. 摘要器：TextDelta 与 ThinkingDelta 双累积；文本空但推理非空 → 采用推理为摘要；两者皆空才判空。加 2 条单测（reasoning-only 成功 / 全空仍错）。

## Capabilities

- `agent`（react 流组合保序、compaction 摘要器）
- `test-infra`/`domain-compaction`（BDD 场景：turn-end overflow 事件流契约、摘要 reasoning-only）

## Impact / 风险

- 事件送达顺序修复对所有消费方保真（不重排、不丢）；摘要器只放宽「非空」判定（不改变成功摘要形状）。
- BDD 全量并行须保持绿（spec check 不再 --no-check）。
