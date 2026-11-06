---
depends_on:
  - c3106-add-codemode-tool
  - c3222-add-structured-tool-result-details
title: 在 TUI 呈现 codemode 运行与目录
---

## Why

c3106 的 `run_code` 是 `ModelOnly` 工具，TUI 目前只按通用 tool 事件显示，没有 codemode 专属状态、catalog 视图或嵌套调用反馈。用户启用 `codemode` 后看不到“已启用 / catalog 有多少工具 / 哪些 deferred / 本次脚本调用了什么”；`/capabilities` 只覆盖 session 工具与 MCP。缺少可观察面会让 PTC 能力像“没有注册”。

## What Changes

- 目录面：新增 `/codemode` 面板（或扩展现有 capabilities 面板）展示 enabled、runtime、timeout、fuel、memory、catalog 数量、inline / deferred 工具、tool-exposure 过滤结果。
- 不新增 codemode 启用状态短标识；header / status 不因 codemode 单独换文案。
- 运行面：`run_code` 外层 tool 事件显示脚本摘要、结果/错误、截断标记；嵌套工具调用以结构化摘要呈现，不伪造第二套执行路径。
- 交互面：面板只读；关闭不 abort agent；不得从 TUI 直调 runtime / 读 session store；所有数据经现有 `XyDriver` / 协议快照。
- 测试：TUI harness 快照、BDD 场景、无 codemode 零显示、deferred / tool-exposure 展示、嵌套调用摘要。

## Capabilities

- `app-tui`
- `app-tui-host`
- `agent-codemode`

## Impact

- TUI commands / effects / bridge / layout / widgets。
- 可能需要 codemode catalog / status 快照经 `XyDriver` 暴露；不在本 change 改 guest ABI。
- specs：app-tui-host、app-tui-commands、app-tui-fixed-zone 可能新增条款。

## Further Notes

该 change 只补展示与交互，不改变 `run_code` 的权限、隔离或 catalog 语义。声明边界见 `c3218-update-codemode-declarations-from-runtime-toolset`。
