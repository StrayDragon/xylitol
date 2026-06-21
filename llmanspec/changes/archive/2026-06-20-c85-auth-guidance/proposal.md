---
depends_on: []
---

# c85-auth-guidance: 认证配置指引消息

## Why
pi 的 `auth-guidance.ts`（25 LOC）提供用户友好消息：
- `formatNoModelsAvailableMessage()` — "No models available. Use /login... + providers.md/models.md"
- `formatNoModelSelectedMessage()` — 同上 + model select 指引
- `formatNoApiKeyFoundMessage(provider)` — 带 provider 名称的 key 缺失提示
- `getProviderLoginHelp()` — 通用引导：`/login` + 文档路径

xylitol 在没有可用模型或 API key 时只抛原始错误，不提供下一步操作指引，用户体验差。

## What Changes
- 新增 `src/agent/auth_guidance.rs`：
  - `get_provider_login_help() -> String` — 引导 `/login`、`providers.md`、`models.md`
  - `format_no_models_available_message() -> String`
  - `format_no_model_selected_message() -> String`
  - `format_no_api_key_found_message(provider) -> String`
- 集成到 CLI startup：当无模型可用时打印引导消息
- 集成到 RPC mode：在 `get_state`/错误响应中包含引导消息

## Capabilities
- user-experience

## Impact
- 非破坏性：纯新增模块 + 消息集成。
- 无新依赖。
