---
depends_on: []
title: Loop 示例插件演示 plan-as-state 驱动
needs_specs_change: true
branch: sdd/c2957-add-loop-example-plan-state
base_branch: main
base_sha: 7ded2ef2b8bc64ef7e90d347a0aea20d9a25ba8c
---

# Loop 示例插件演示 plan-as-state 驱动

## Why

`c2956` 记录了「不引入 `ReActStrategy` trait，先用中间件组合验证 Plan-then-Act」的决策。本 change 是该决策的**低成本实验**：

- 证明 plan 可以作为**持久工件**（`todo_rewrite` / `todo_update` 已落会话存储）；
- 证明中间件能读 plan 状态并驱动 `decide_continue`，**不换 runtime、不动 core**；
- 为将来是否值得做 core 级拓扑 seam 提供实测证据。

## What Changes

- `LoopExampleConfig` 新增 `plan_state: bool`（缺省 `false`），仅在启用示例插件时生效。
- 新增 `PlanStateMiddleware`，阶段为 `before_turn` / `after_tool` / `decide_continue` / `after_turn`：
  - `before_turn` 注入 plan 契约系统碎片；
  - `after_tool` 从 `todo_rewrite` / `todo_update` 的结构化结果更新 plan 进度；
  - `decide_continue` 在 plan 未完成且未到 settle 上限时继续，否则停止；
  - `after_turn` 记录 checkpoint 摘要（日志，不发新事件）。
- 启用 `plan_state` 时注册两个中间件（`loop-example` + `plan-state`）；卸载时一并注销。
- 同步 `configs/config.schema.json` 与 `configs/example.yaml` 注释示例。

## Capabilities

- `agent-runtime`：`r2925` 补「计划模式启用后注册计划中间件」场景；示例插件仍满足「至少一个只收紧的 `before_tool` 与一个自定义 `decide_continue`」。

## Impact

- crate：`plugins/xylitol-official-plugin-loop-example`（config / middleware / plugin）。
- 配置：`configs/config.schema.json` 重新生成；`configs/example.yaml` 注释补 `plan_state`。
- 测试：中间件单测 + 插件注册/卸载单测 + 1 个 BDD 场景。
- 文档：`c2956` 草案记录决策与 ROI；本 change 只提供实验证据。

## Out of Scope

- 不改 core，不新增 `ReActStrategy` trait。
- 不做 plan checkpoint / resume / 局部 replan 的持久化协议。
- 不改 `decide_continue` 的 `control` 闭集。
- 不把示例插件变成产品默认；仍由配置显式启用。
