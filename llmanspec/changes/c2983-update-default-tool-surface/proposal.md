---
depends_on: []
title: 收敛默认工具面：plan/todo 工具常驻
needs_specs_change: true
---

# 收敛默认工具面：plan/todo 工具常驻

## Why

- `ToolFeatures::default()` 为 `todo=true`、`plan=true`，每个会话的工具表默认带 5 个 plan 工具（`plan_write/read/list/submit/mode`）+ 2 个 todo 工具，即使从不使用 plan/todo。
- 这会增加 provider 可见 `tools[]` 的 schema token 与模型决策面；对短会话/简单任务是不必要的固定成本。
- 现有 spec 只规定 todo 工具须在默认工具表且首轮冻表前可见（`agent-todo`），没有规定 plan 工具必须常驻。
- 工具表在首次 generate 后冻结，回合中途不得热加；因此“进入 plan mode 才注册 plan 工具”与现有冻表语义存在冲突，需要显式决策。

## What Changes

- 先量化成本：统计默认工具表 schema 的 token 占用量（plan/todo 各占多少）。
- 按量化结果选择（见 design）：
  - 保持现状并记录成本；
  - 默认 `plan.enabled=false`，由 `/plan` 开启（需要定义开启动态工具表的冻表/重定稿语义）；
  - 用独立的 tool-exposure 中间件在 execute 模式隐藏 `plan_*`（只影响模型可见 schema，不删工具）；
  - 进入 plan mode 时注册 plan 工具（需要改冻表语义，代价最大）。
- 若改变默认值或可见性，同步更新 `runtime-config` / `agent-plan` / `agent-todo` 的默认值与工具表条款。

## Capabilities

- `agent-plan`：plan 工具可见性/默认值。
- `agent-todo`：todo 工具默认可见性（保持现状或一并调整）。
- `runtime-config`：`plan.enabled` / `todo.enabled` 默认值。
- `agent-runtime`：工具表冻结与重定稿语义（若选择动态注册）。

## Impact

- `crates/xylitol-kernel/src/driver/in_process/mod.rs`：`ToolFeatures::default` 与 `compose_tools`。
- `plugins/xylitol-official-plugin-plan/src/tools.rs`：`plan_tools` 注册入口。
- `plugins/xylitol-official-plugin-config/src/types.rs`：默认配置。
- specs：默认值与工具表条款。

## Out of Scope

- MCP 工具的动态发现（已有 `mcp8` 冻表/重定稿语义）。
- 工具 schema 的具体文案压缩。

## Further Notes

- 由 PR #30 review F6 记录。
- 这是**决策型 change**：先测量，再决定是否改默认值/可见性。
