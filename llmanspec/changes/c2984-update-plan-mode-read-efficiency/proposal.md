---
depends_on: []
title: 降低 plan 中间件每轮读取整条 leaf branch 的开销
needs_specs_change: false
---

# 降低 plan 中间件每轮读取整条 leaf branch 的开销

## Why

- `PlanModeMiddleware::before_llm_request` 每次模型迭代都 `load_leaf_branch` 整条分支，只为取最新 `plan_mode`。
- core 在同一迭代又为状态栏读一次 `load_leaf_branch`（`react/mod.rs:1122`）。
- 默认 `plan.enabled=true`，所以每个模型迭代有两次完整分支读取；长会话（多 segment、多条目）下是可感知的开销。
- 目标行为不变：仍须“读取最新 `plan_mode`”，只是消除重复读取。

## What Changes

- 让 core 在一次迭代内只读一次 leaf branch，并把 `latest_plan_mode` 结果传给 plan 中间件（或缓存供状态栏与中间件共用）。
- 或引入只读尾部条目的 store 查询（`latest_custom_entry`），避免整条分支加载。
- 保持 `plan_mode` 变更在下一迭代生效；会话切换/压缩后不得使用陈旧缓存。
- 补测试：用计数 store 断言每迭代读取次数下降；模式变更仍生效。

## Capabilities

- `agent-plan`：读取最新 `plan_mode` 的行为不变。
- `agent-runtime`：每迭代状态栏与中间件共享读取。

## Impact

- `plugins/xylitol-official-plugin-plan/src/middleware.rs`：`load_mode`。
- `crates/xylitol-runtime/src/agent/runtime/react/mod.rs`：状态栏读取与 ctx 构造。
- `crates/xylitol-plugin-sdk/src/loop_middleware.rs`：若在 `BeforeLlmRequestCtx` 增加 `plan_mode` 字段（SDK 变更）。
- `crates/xylitol-plugin-sdk/src/ports/session.rs`：若增加 `latest_custom_entry`。

## Out of Scope

- plan 文档本身的读取（`plan_write/read` 工具路径）。
- 压缩后状态保留（`c2975` 已覆盖）。

## Further Notes

- 由 PR #30 review F7 记录。
- 纯性能优化，不改产品语义；`needs_specs_change: false`。
