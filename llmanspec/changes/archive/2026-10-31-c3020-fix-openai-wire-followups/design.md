# 设计：OpenAI 线协议跟进

## 结构化输出并行开关

- `apply_text_format(body, options, has_tools)`：设置 `text.format` 后，若 `has_tools` 为真则写 `parallel_tool_calls: false`。
- `apply_structured_output(body, options, has_tools)`：写 `response_format` 后同上。
- 仅在 `structured_output` 存在且工具非空时写入；其余情况不触碰该字段。

## 缓存键默认关

- `wire_policy::defaults::PROMPT_CACHE_KEY` 改为 `false`。
- `resolve_wire_policy` 既有逻辑不变：`prompt_cache.enabled` 显式覆盖。
- 默认策略下 `prompt_cache_key` 不发送；显式开启时仍取会话 id。

## 测试

- Responses：结构化 + tools → `parallel_tool_calls=false`；结构化无 tools → 无该字段；默认策略 → 无 key。
- Completions：同上，wiremock `body_partial_json` 断言。
- wire policy：默认 key=false，显式 true 仍为 true。
