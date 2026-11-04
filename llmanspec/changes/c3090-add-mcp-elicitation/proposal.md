---
depends_on: [c3055-add-mcp-scope-pool]
title: "MCP elicitation"
---

## Why

MCP elicitation 与内置 `ask` 是两种处理路径；复用 `AskHostGateway` / `EditorSlot::Choice` 会把两者耦合，后续难拆解与迭代。选择独立完整处理。

## What Changes

- client capability 声明 elicitation。
- form 模式：JSON Schema 表单、默认值、校验、用户可修改后提交。
- url 模式：展示目标域名、征得同意后导航；敏感信息不得走 form。
- 独立 `ElicitationSlot`；独立 host↔client RPC 方法；不复用 `EditorSlot::Choice` 语义。
- 按 session / workspace 路由；显示 server 身份；支持 decline / cancel。
- 权限与审计；日志脱敏。
- TUI 固定区接线；host 反向 RPC。

## Capabilities

- `infra-mcp`：elicitation 协议与能力。
- `app-tui-fixed-zone`：ElicitationSlot。
- `protocol-app`：elicitation 反向 RPC。
- `domain-security`：敏感信息与同意。

## Impact

MCP plugin、host、TUI、protocol。

## Further Notes

- 用户决策：独立完整处理；MCP 与 ask 分开。
- 依赖 `c3055-add-mcp-scope-pool`（session/workspace 路由）。
- 规范：elicitation form/url 两模式；form 不得收集 secrets；url 用于敏感交互。

## Open Questions

- form 渲染范围（flat primitive schema）与默认值。
- url 同意的 UI 与域名展示。
- 权限模型：每次同意 / session 内记住 / 按 server 记住。
