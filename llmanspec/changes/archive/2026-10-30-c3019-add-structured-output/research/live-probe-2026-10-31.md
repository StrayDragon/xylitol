# Live probe：结构化输出（2026-10-31）

探针入口：`plugins/xylitol-official-plugin-provider-openai/tests/lab_live_responses.rs`、
`plugins/xylitol-official-plugin-provider-anthropic/tests/lab_live_messages.rs`。

## OpenAI Responses

- 端点 / 模型：`http://tufa:8731/v1` / `halogen-qwen3.8-flash-next`。
- `text.format`（json_schema、strict）被端点接受并执行：返回 `{"answer":"OK"}`。
- usage：input 59 / output 36 / cache_read 59。

## Anthropic Messages

- 端点 / 模型：`https://api.deepseek.com/anthropic` / `deepseek-v4-flash`。
- `output_config.format`（json_schema）被端点接受，200；但端点忽略该字段，返回纯文本 `OK`。
- 结论：请求形状保真；端点能力不一，调用方不得假设 schema 一定被执行。

## 未覆盖

- strict schema 子集、嵌套 schema、`tool_choice` / `parallel_tool_calls` 交互未验证。
- 响应侧 `parsed_output` 解析未做（仍按文本返回）。
