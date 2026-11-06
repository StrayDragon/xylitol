---
depends_on: []
title: 为工具结果增加结构化 details
---

## Why

c3220 需要显示 `run_code` 的外层结果、错误、截断标记与嵌套调用摘要。当前 `XyTool::execute` 只返回 `String`，`execute_as_parts` 只返回 `Vec<AgentPart>`；`ToolExecutionEnd` 只有 `result` / `is_error`，没有 `details`。TUI 只能看到扁平文本，无法可靠区分成功值、错误类别、截断与嵌套调用。

## What Changes

- 一步到位：用 `XyToolOutcome` 替换现有 `execute` / `execute_as_parts` / `XyToolCallResult` 路径，所有工具迁移；不保留旧入口或兼容 shim。
- `XyToolOutcome` 字段：`parts` + `details` + `is_error` + `error_kind`。
- `ToolExecutionEnd` 增加可选 `details`；wire 映射与 TUI bridge 透传。
- `XyToolErrorKind` 用 snake_case 枚举：`invalid_args` / `execution_failed` / `permission_denied` / `timeout` / `aborted`。
- `details` 是新增可选字段，wire 兼容；不 bump `PROTOCOL_VERSION`。
- `run_code` 提供 details：嵌套调用摘要（工具名 / 参数摘要 / 成功或错误 / 耗时）、截断标记、错误类别、脚本摘要。
- hooks / loop after_tool 修改结果时同步更新 details；不得让 details 与 parts 漂移。
- details 不做额外脱敏：本地 TUI 沿用现有 tool 参数展示策略，仅按长度截断。
- 测试：所有工具 outcome 迁移、details 透传、run_code 嵌套摘要、截断、错误类别、hook 修改。

## Capabilities

- `agent-tools`
- `agent-runtime`
- `app-tui`

## Impact

- SDK `XyTool` / `XyToolOutcome` / 事件类型；删除旧结果类型。
- runtime `run_one` / after_tool / 事件发射。
- wire 映射与 TUI bridge；协议版本按破坏性变更处理。
- 不改变工具权限、执行顺序或 codemode 隔离。

## Further Notes

c3220 依赖本 change 才能显示嵌套摘要。不保留旧客户端兼容；若 wire 形状变化，按 `PROTOCOL_VERSION` 规则处理。
