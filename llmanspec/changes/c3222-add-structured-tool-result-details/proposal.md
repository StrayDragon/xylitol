---
depends_on: []
title: 为工具结果增加结构化 details
---

## Why

c3220 需要显示 `run_code` 的外层结果、错误、截断标记与嵌套调用摘要。当前 `XyTool::execute` 只返回 `String`，`execute_as_parts` 只返回 `Vec<AgentPart>`；`ToolExecutionEnd` 只有 `result` / `is_error`，没有 `details`。TUI 只能看到扁平文本，无法可靠区分成功值、错误类别、截断与嵌套调用。

## What Changes

- 在 SDK 增加结构化工具结果：`parts` + `details` + `is_error` + 错误类别（可选）。默认实现把现有 `execute` / `execute_as_parts` 适配为无 details。
- 在 `ToolExecutionEnd` 增加可选 `details`；wire 映射与 TUI bridge 透传。
- `run_code` 覆盖结构化结果：嵌套调用摘要（工具名 / 参数摘要 / 成功或错误 / 耗时）、截断标记、错误类别、脚本摘要。
- hooks / loop after_tool 修改结果时同步保留或替换 details；不得让 details 与 parts 漂移。
- 保持现有工具行为不变；没有 details 的工具按旧路径显示。
- details 不做额外脱敏：本地 TUI 沿用现有 tool 参数展示策略，仅按长度截断。
- 测试：默认适配、details 透传、run_code 嵌套摘要、截断、错误类别、hook 修改。

## Capabilities

- `agent-tools`
- `agent-runtime`
- `app-tui`

## Impact

- SDK `XyTool` / `XyToolCallResult` / 事件类型。
- runtime `run_one` / after_tool / 事件发射。
- wire 映射与 TUI bridge。
- 不改变工具权限、执行顺序或 codemode 隔离。

## Further Notes

c3220 依赖本 change 才能显示嵌套摘要。若只做扁平文本解析，会把呈现耦合到 `run_code` 的 JSON 格式，不采纳。
