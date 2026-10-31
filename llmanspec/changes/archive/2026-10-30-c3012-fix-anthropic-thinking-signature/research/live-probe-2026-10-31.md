# Live probe：thinking 签名（2026-10-31）

探针入口：`plugins/xylitol-official-plugin-provider-openai/tests/lab_live_responses.rs`、
`plugins/xylitol-official-plugin-provider-anthropic/tests/lab_live_messages.rs`（均 `#[ignore]`）。

## OpenAI Responses

- 端点 / 模型：`http://tufa:8731/v1` / `halogen-qwen3.8-flash-next`（generic compat）。
- `thinking_level=high`：收到 `ThinkingEnd`，`thinking_len=85`，`thinking_signature=Some(<reasoning item JSON>)`。
- 签名承载完整 reasoning item（含 `id`、`summary`、`encrypted_content`），回放路径可用。

## Anthropic Messages

- 端点 / 模型：`https://api.deepseek.com/anthropic` / `deepseek-v4-flash`（deepseek compat）。
- `thinking_level=high`：收到 `ThinkingEnd`，`thinking_len=84`，`thinking_signature=Some("39787800-…")`。
- 端点返回 thinking block 与签名串；回放形状与官方一致。

## 未覆盖

- 多 thinking block 交错、`redacted_thinking`（本次端点未返回）。
- 签名回放后的二次请求未做（只验证了首次响应解析）。
