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

- `/mcp prompt`：列出当前 scope 的 prompt，Choice 选择后 `prompts/get`，把结果插入 editor 供用户编辑/发送，不自动提交。
- `@` 资源选择器：列出已连接 server 的 resources；选择后插入 `@server:uri` 引用，发送时解析 `resources/read` 并作为上下文块附加。
- 连接后按 `subscriptions/listen` 订阅 `resources/list_changed` 与已选 resource URI；更新 catalog 与缓存。
- 资源读取权限：用户显式选择即同意；远程内容标记为不可信上下文，不自动执行。
- 缓存：按 connection key + URI 缓存；LRU 上限、TTL、更新失效。
- 错误与诊断：读取失败、订阅失败、缓存失效均可观察。

## Scope

- 不改模型工具名；prompt/resource 仍以模型工具存在。
- 不做资源内容自动注入或自动执行。
- 不实现 WebSocket。

## Impact

- `xylitol-official-plugin-mcp`：catalog、subscription、resource read/cache。
- `xylitol-serve` / `xylitol-wire`：prompt/resource 读取与订阅事件。
- `xylitol-app-tui`：slash / `@` 选择器与上下文块呈现。

## Further Notes

- 2026-11-07 从 delayed 提升为 active。
- c3070 已归档；c3213 只补 TUI 与订阅，不重做模型工具。
- trust 不另设项目门禁；MCP server 由配置启用，资源内容按不可信输入处理。
