# Live probe：prompt caching（2026-10-31）

探针入口：`plugins/xylitol-official-plugin-provider-anthropic/tests/lab_live_messages.rs`、
`plugins/xylitol-official-plugin-provider-openai/tests/lab_live_responses.rs`。

## OpenAI Responses

- 端点 / 模型：`http://tufa:8731/v1` / `halogen-qwen3.8-flash-next`。
- 带 `obs_session.session_id` 的请求写入 `prompt_cache_key`，端点接受，200。
- 同前缀重复请求：`usage.cache_read=59`、`prompt_cache_read=Tokens(59)`，缓存命中如实透出。

## Anthropic Messages

- 端点 / 模型：`https://api.deepseek.com/anthropic` / `deepseek-v4-flash`。
- 显式开启 `prompt_cache.enabled=true` 后，`cache_control` 断点被端点接受，200，无 400。
- `usage.cache_read=0`、`cache_write=0`：该兼容端未上报 cache 字段（或未命中）。
- 结论：请求形状保真；usage 透出依赖端点，不得假设 Anthropic 官方语义。

## 未覆盖

- 长前缀 / 4 断点上限 / `ttl=1h` beta 头未对照验证。
- `prompt_cache_retention=24h` 未单独验证。
