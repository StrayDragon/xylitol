---
depends_on:
  - c3090-add-mcp-elicitation
title: "Add MCP elicitation form UI"
branch: main
base_branch: origin/main
base_sha: 4e422320e5d2e5648f48d06ca1321f78d3c569d9
---

## Why

c3090 只实现默认 decline 与秘密字段拒绝，保证不收集秘密；用户无法在 TUI 看到表单、填写并 accept。MRTR 的 `input_responses` 回传也未接。

## What Changes

- TUI elicitation 表单槽位：渲染 `requestedSchema` 的 primitive 字段（string / number / integer / boolean / enum）。
- 用户选择 accept / decline / cancel；accept 时校验必填与类型。
- accept 后把 `ElicitResult.content` 回传给 server，驱动 MRTR 重试。
- 秘密字段策略：password / secret / token / api_key / credential / private_key 命中则整表 decline，不渲染。
- URL 模式仍 decline；不做浏览器自动化。
- 审计：记录 server / 字段名 / 动作 / 时间；不记录值或 message。
- 超时：等待用户响应 5 分钟，超时 decline。

## Scope

- 不做嵌套 object / array schema；MCP primitive 子集之外 decline。
- 不收集秘密字段。
- 不把表单值写日志或 session 历史。

## Impact

- `xylitol-official-plugin-mcp`：`create_elicitation` 异步等待 UI 响应。
- `xylitol-serve` / `xylitol-wire`：elicitation 请求事件与响应 unary。
- `xylitol-app-tui`：表单槽位、焦点、校验。

## Further Notes

- 2026-11-07 从 delayed 提升为 active。
- c3090 已归档；本 change 只补 accept 路径与表单 UI。
