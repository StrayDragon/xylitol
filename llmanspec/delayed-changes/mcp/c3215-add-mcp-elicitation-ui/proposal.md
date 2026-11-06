---
depends_on:
  - c3090-add-mcp-elicitation
title: "Add MCP elicitation form UI"
---

## Why

c3090 只实现默认 decline 与秘密字段拒绝，保证不收集秘密；用户无法在 TUI 看到表单、填写并 accept。MRTR 的 `input_responses` 回传也未接。

## What Changes

- TUI elicitation 表单槽位：渲染 `requestedSchema` 的 primitive 字段。
- 用户选择 accept / decline / cancel。
- accept 后把 `input_responses` 回传给 server，驱动 MRTR 重试。
- 秘密字段策略：password / secret / token / api_key / credential / private_key 继续拒绝。
- URL 模式：展示 URL，用户确认后继续或取消。
- 审计：记录 elicitation 请求 / 决策，不记录字段值。
- 超时 / 取消：与 ask 槽位一致。

## Scope

- 不收集秘密；不得把表单值写入日志或模型上下文。
- 不实现新的 elicitation 协议方法；复用 `elicitation/create` 与 MRTR。
- 表单 schema 只支持 primitive 类型。

## 延后原因

- 表单 UI 跨 TUI / protocol / runtime；accept 路径需要 schema 渲染与 MRTR 回传。
- 秘密边界与 URL 模式的安全策略未定；先保证默认 decline 不收集秘密。
- 主线优先交付 elicitation 的拒绝语义。

## 启动前置条件

- c3090 已归档（当前已满足）。
- TUI 有表单 / Choice 槽位与焦点管理。
- 有 MRTR elicitation fixture（stateful fixture 已提供 `elicit_name` 雏形）。
- 确定 secret-field 策略与 URL 模式处理。
- 确定审计字段与脱敏规则。

## Related

- c3090-add-mcp-elicitation（已归档）
- c3080-add-mcp-stateful-fixtures（fixture）
- c3095-add-mcp-custom-transports

## Open Questions

- 表单 schema 子集（string / number / boolean / enum）。
- URL 模式是否允许打开浏览器。
- 超时与 cancel 的语义。
- accept 值是否进入模型上下文。
