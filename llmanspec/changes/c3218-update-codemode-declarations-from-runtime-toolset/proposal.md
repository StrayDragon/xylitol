---
depends_on:
  - c3106-add-codemode-tool
title: 用运行期工具快照生成 codemode 声明
---

## Why

c3106 的声明中间件在插件 `apply` 时读取 `HostRegistries.tools` 快照。该快照早于 `compose_tools` 注入 session 专属工具，导致 system prompt 的内联声明可能缺少 `todo` / `workflow` / `ask`；运行期 `ToolSet` 与 guest `ALL_TOOLS` 却包含这些工具。声明与可执行 catalog 不一致，模型需要额外发现步骤，且不同会话/装配路径可能得到不同声明。

## What Changes

- 把声明生成移到 `compose_tools` 之后，或在 `BeforeLlmRequest` 用运行期 `ToolSet` 快照重建声明；不得继续依赖插件 `apply` 时的静态注册表快照。
- 声明、guest `ALL_TOOLS`、dispatch catalog 使用同一份 catalog 快照；session 工具、MCP 工具、exposure / tool-exposure / exposure overrides 只计算一次。
- 保留 inline budget 与 deferred 回退；声明顺序稳定，不得重复列出同一工具。
- 声明随 freeze / reload / resume / 切会话更新；不得把旧快照粘到新会话。
- 测试覆盖：session 工具出现在声明与 `ALL_TOOLS`；tool-exposure 过滤同时作用于声明与 dispatch；reload 后声明更新；无 MCP 时零额外开销。

## Capabilities

- `agent-codemode`
- `agent-runtime`
- `agent-tools`

## Impact

- kernel driver 装配：声明生成时机与 `ToolSet` 快照传递。
- codemode 插件：移除 apply 时 `HostRegistries` 声明快照，改为运行期输入。
- specs / BDD：补充声明与 catalog 一致性场景。

## Further Notes

该 change 只修声明边界，不改 Wasmtime guest ABI、run_code schema 或 TUI 呈现。TUI 呈现另见 `c3220-add-tui-codemode-presentation`。
