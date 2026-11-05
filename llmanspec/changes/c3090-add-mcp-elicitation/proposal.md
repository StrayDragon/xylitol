---
depends_on: [c3055-add-mcp-scope-pool]
title: "MCP elicitation"
---

## Why

MCP elicitation 与内置 `ask` 是两种处理路径；复用 `AskHostGateway` / `EditorSlot::Choice` 会把两者耦合，后续难拆解与迭代。选择独立完整处理。

## What Changes

- client capability 声明 elicitation。
- form 模式：受限 flat primitive JSON Schema；默认值预填；用户可 review / modify；提交前校验。
- url 模式：展示完整 URL 与目标 domain/host；显式同意后才导航；不得自动 fetch；client 只得知是否同意。
- MRTR：server 返回 `InputRequiredResult` + `elicitation/create`；client 收集 `inputResponses` 并回传 `requestState`，重试原请求。
- 独立 `ElicitationSlot`；独立 host↔client RPC 方法；不复用 `EditorSlot::Choice` 语义。
- 按 session / workspace 路由；显示 server 身份与请求原因；支持 decline / cancel。
- 权限模型：每次同意 / session 内记住 / 按 server 记住；敏感操作默认每次同意；审计。
- 日志脱敏；form 不得收集 secrets。

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
- 规范：form 只允许 flat primitive schema；secrets 必须走 URL mode；client 必须显示 server 身份、支持 decline/cancel；URL mode 必须显示 domain/host 并显式同意。
- 例子：旅行预订（座位/房型/联系方式）；API key/OAuth/payment 走 URL mode。

## Open Questions

- form 渲染范围（flat primitive schema）与默认值。
- url 同意的 UI 与域名展示。
- 权限模型：每次同意 / session 内记住 / 按 server 记住。
- MRTR 的 `InputRequiredResult` / `requestState` 是否被 rmcp 3.8.0 完整暴露：未验证，design 前查一次。
