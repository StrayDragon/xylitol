---
depends_on:
  - c2944-add-loop-middleware
title: decide_continue 压缩决策带指令与结算刷新
---

# decide_continue 压缩决策带指令与结算刷新

## Why

c2944 的 `compact` 决策只能表达「压一次」，无法表达「为什么压」：

- `force_compact_once` 传 `instructions = None`，插件无法指定压缩焦点（如「保留最近 N 条工具结果」）。
- 压缩事件固定 `reason = "manual"`，观测上无法区分「用户手动压缩」与「中间件决策压缩」。
- 压缩后不重新发 `ContextTokenSettlement`，插件与端都看不到压缩后的占用。

## What Changes

- `DecideContinueCtx` 增加可选 `compact: CompactRequest`（`instructions: Option<String>`、`reason` 标签）。
- 中间件返回 `compact` 时，core 用该请求执行压缩；`reason` 进入 `CompactionStart.reason`（稳定字符串，如 `loop_middleware:<id>`）。
- 压缩成功后 core 发一次 `ContextTokenSettlement`（`reason` 为压缩后结算），并把重载后的 history 交回循环。
- 硬上限沿用 c2944：连续 `compact` 且无进展超过上限 → `LoopMiddlewareLimit`。
- 压缩失败仍以错误结束本 run（沿用 c2944 语义），但错误消息带 middleware id。

## Out of Scope

- 让中间件直接调用 `XyCompaction`（压缩仍只由 core 调度）。
- 压缩策略（阈值、keep tokens）由中间件改写——那属于 `before_model` 的 model 参数面，不在本 change。

## Capabilities

- `agent-runtime`：`compact` 决策带指令、事件 reason 可辨识、压缩后结算 MUST + 场景。

## Impact

- crate：`xylitol-plugin-sdk`（`CompactRequest` 载荷）、`xylitol-runtime`（`force_compact_once` 传参 + 结算 + 事件）。
- 测试：BDD（compact 决策 → `CompactionStart.reason` 含 middleware id；压缩后出现结算事件）；runtime 单测（上限仍生效）。
- 文档：`src/AGENTS.md`「Loop 中间件」段补 compact 口径。

## Further Notes

来源：c2944 verify 的缺口 H。
