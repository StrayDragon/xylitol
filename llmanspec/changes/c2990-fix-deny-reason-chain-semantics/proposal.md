---
depends_on: []
title: 明确 before_tool 拒绝原因在中间件链上的语义
needs_specs_change: true
---

# 明确 before_tool 拒绝原因在中间件链上的语义

## Why

- c2980 新增 `BeforeToolCtx.deny_reason`，plan 只读闸设置后返回 `Continue`，runtime 读取链终态作为拒绝原因。
- 字段本身是普通 `Option`，后置中间件可以覆盖或清空它；当前没有 spec 规则，行为实际是“最后写入者赢”。
- 一旦后置中间件清空，前序拒绝会被静默取消，工具继续执行。官方中间件目前不这么做，但第三方插件开放后这是安全缺口。

## What Changes

- 明确 `deny_reason` 为“首次设置即锁定”：任一 `before_tool` 中间件设置后，后置中间件不得清空或改写，仍可读取该原因。
- runtime 在链结束时以首个非空原因作为最终拒绝原因，并在工具执行前回写错误结果。
- 更新 `agent-runtime` 的 `r2962`（multiple-before-tool-gates）并补单测。

## Capabilities

- `agent-runtime`：`before_tool` 链的拒绝语义。

## Impact

- `crates/xylitol-runtime/src/agent/runtime/middleware.rs`：`run_before_tool` 的链终态收敛。
- `llmanspec/specs/agent-runtime/agent-runtime.feature`：`r2962` 扩展。
- runtime 单测：先 deny 后 clear 的链行为。

## Out of Scope

- 不新增控制枚举或 SDK 字段。
- 不改 `stopped` 控制语义。
- 不改 plan 只读闸的 allowlist。
