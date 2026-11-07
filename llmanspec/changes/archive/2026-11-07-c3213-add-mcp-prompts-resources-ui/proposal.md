---
depends_on:
  - c3070-add-mcp-prompts-resources
title: "Add MCP prompts and resources UI"
branch: main
base_branch: origin/main
base_sha: 4e422320e5d2e5648f48d06ca1321f78d3c569d9
---

## Why

c3070 已把 prompts / resources 暴露为模型工具（`mcp__server__prompt__name`、`mcp__server__resource__read`），但 TUI 的 slash prompt 选择器、`@` 资源选择器、`subscriptions/listen` 与资源读取权限/缓存均未实现。用户仍无法从 TUI 主动选择 prompt 或附加 resource。

## What Changes

- `/mcp prompt`：列出当前 scope 的 prompt；有参数时用 `ChoicePrompt` 自定义输入收集参数；`prompts/get` 结果插入 editor，不自动提交。
- `/mcp resource` / `@mcp:`：列出已连接 server 的 resources；选中后加入 editor 上方附件 tray，不插入可解析文本 token。
- 附件 tray：支持点击 `×`、Backspace、快捷键移除；发送时按附件读取，不解析未加引号的 `@mcp:` 文本。
- 发送时经 host `mcp.resource.read` 读取；成功则 `XyDriver::run_with_context` 把资源作为 `CustomMessage` 上下文条目写入会话；失败阻止发送。
- 可选显式文本语法 `@mcp:"<server>/<uri>"`：只在完整引号形式下解析；未加引号按普通文本；未知资源阻止发送。
- 连接后按 `subscriptions/listen` 订阅 `resources/list_changed` 与已附加 URI；更新 catalog 与缓存。
- 资源读取权限：用户显式选择即同意；远程内容标记为不可信上下文，不自动执行。
- 缓存：按 connection key + URI 缓存；LRU 上限、TTL、更新失效。
- 错误与诊断：读取失败、订阅失败、缓存失效均可观察。

## Scope

- 不改模型工具名；prompt/resource 仍以模型工具存在。
- 不做资源内容自动注入或自动执行。
- 不实现 WebSocket。
- 首版不解析未加引号的 `@mcp:...`；该形式只是普通文本。

## Impact

- `xylitol-official-plugin-mcp`：catalog、subscription、resource read/cache。
- `xylitol-runtime` / `xylitol-plugin-sdk`：`run_with_context`、`CustomMessage` 上下文条目。
- `xylitol-serve` / `xylitol-wire` / `xylitol-protocol`：prompt/resource 读取、context 载荷、订阅事件。
- `xylitol-app-tui`：prompt/resource 选择器、附件 tray、`@mcp:` 补全、context 块呈现。

## Further Notes

- 2026-11-07 从 delayed 提升为 active；同日按用户侧形态收敛为“结构化附件 + context seam”。
- c3070 已归档；c3213 只补 TUI 与订阅，不重做模型工具。
- trust 不另设项目门禁；MCP server 由配置启用，资源内容按不可信输入处理。
