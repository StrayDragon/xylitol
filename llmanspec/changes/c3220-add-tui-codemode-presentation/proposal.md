---
depends_on:
  - c3106-add-codemode-tool
  - c3222-add-structured-tool-result-details
title: 在 TUI 呈现 run_code 运行结果
---

## Why

`run_code` 是 `ModelOnly` 工具，TUI 目前只按通用 tool 事件显示扁平文本。用户看不到脚本摘要、成功值、错误类别、截断标记与嵌套调用摘要；`run_code` 返回的 JSON 文本也不应让 TUI 解析。用户不需要额外的 `/codemode` 命令或状态短标识：启用 codemode 并选择 `run_code` 后，展示应自动生效。

## What Changes

- `run_code` 外层 tool 事件显示脚本摘要、结果、错误类别、截断标记、耗时与嵌套调用摘要。
- 数据来自 c3222 的结构化 `details`；不解析 `run_code` 的 JSON 文本，不依赖被抑制的嵌套事件。
- 不新增 `/codemode` 命令、面板或 header / status 短标识；不新增执行路径。
- 没有 `details` 时回退到现有通用文本显示；不影响其他工具。
- 测试：成功、错误、截断、嵌套摘要、无 details 回退。

## Capabilities

- `app-tui`
- `app-tui-host`
- `agent-codemode`

## Impact

- TUI bridge / layout / widgets 的 tool 结果渲染。
- 不新增协议命令或状态快照；不改变 `run_code` schema / 权限 / 隔离。

## Further Notes

该 change 只补展示，不改变 codemode 激活、catalog 或声明语义。声明边界见 `c3218-update-codemode-declarations-from-runtime-toolset`。
