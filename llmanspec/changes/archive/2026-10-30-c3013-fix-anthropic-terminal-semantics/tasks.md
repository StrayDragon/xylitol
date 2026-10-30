# Tasks

## 1. 流式终止语义
- [x] `event: error` 转 `Err(XyError::Provider)`，携带 error.type 与 message
- [x] `message_stop` 与异常 EOF 兜底 `Done`，避免重复
- [x] 单测：wiremock 回放 `error` 事件断言 Err；无 `message_stop` 的 EOF 断言 Done

## 2. stop_reason 映射
- [x] 流式与非流式共用映射函数（max_tokens / model_context_window_exceeded / tool_use / refusal / sensitive / pause_turn）
- [x] 单测：`model_context_window_exceeded` → MaxTokens；`refusal` → Error；`tool_use` → ToolUse

## 3. usage 迁移与上报
- [x] `from_anthropic_usage` 迁入 Anthropic 插件并删除 OpenAI 插件实现与测试
- [x] 流式合并 message_start / message_delta usage；非流式直接映射
- [x] 单测：带 cache 字段的 usage 断言 cache_read / cache_write / 1h 与 total

## 4. 门禁
- [x] `cargo test -p xylitol-official-plugin-provider-anthropic` 绿
- [x] `cargo test -p xylitol-official-plugin-provider-openai` 绿
- [x] `just qa` 绿
