# Design

## 决策

不新增 `ReActStrategy` trait。loop 拓扑的差异先由**中间件组合 + 配置**表达；core 仍持 turn 边界、取消、权限终裁与落盘格式。

## 为什么是文档变更

本 change 不改变任何可验证行为：

- 七阶段契约已在 `r2920`–`r2926` 定稿；
- `c2957` 已证明 plan-as-state 可纯插件实现；
- 本 change 只把「不做 trait、什么条件下才做」写成稳定边界，避免后续把 Strategy 误读成插件替换 ReAct 的入口。

因此 `needs_specs_change: false`，只改 `src/AGENTS.md`。

## 触发条件（写进文档）

只有同时满足才评估 core 级策略槽：

1. 中间件表达不了的拓扑（并行 model call、跨 resume 的 plan DAG 执行器）；
2. 至少两个独立实现/消费者；
3. plan-as-state 实测失败（plan starvation / replan thrash / 上下文压不住）。

即使满足，也优先数据/配置驱动，而不是 Rust trait。

## 不做什么

- 不新增 trait / 注册表 / 生命周期。
- 不改 `LoopStage` / `LoopControl` 闭集。
- 不把 `handoff` 从「未支持」改成已支持。
