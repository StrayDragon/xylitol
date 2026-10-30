# Tasks

## 1. 流式签名累积与 ThinkingEnd
- [x] 在 `anthropic_stream` 按 block 累积 thinking 文本与 signature（start 自带或 signature_delta）
- [x] `content_block_stop` 对 thinking block 发 `ThinkingEnd`，带完整文本与签名
- [x] 单测：mock SSE 含 signature_delta，断言 ThinkingEnd 携带签名

## 2. 非流式签名
- [x] `parse_anthropic_response` 的 thinking block 发 `ThinkingEnd` 并带签名
- [x] 单测：非流式 JSON 断言 ThinkingEnd 签名

## 3. 回放保真
- [x] `agent_parts_to_anthropic_blocks` 映射 thinking 与 redacted_thinking
- [x] 无签名 thinking 降级为 text 并记录诊断
- [x] 单测：回放 body 断言两类 block 与顺序；工具续轮顺序不变

## 4. 死代码与门禁
- [x] 删除 `convert_agent_messages_for_anthropic` 的 `system_parts` 死代码
- [x] `cargo test -p xylitol-official-plugin-provider-anthropic` 绿
- [x] `just qa` 与 `just test-provider-safe-tool-names` 绿
