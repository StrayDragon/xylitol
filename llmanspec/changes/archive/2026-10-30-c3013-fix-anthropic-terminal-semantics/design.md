# 设计：Anthropic 终止语义与 usage 保真

## stop_reason 映射

- 单一映射函数供流式与非流式共用：`max_tokens` / `model_context_window_exceeded` → `MaxTokens`；`tool_use` → `ToolUse`；`refusal` / `sensitive` → `Error`；`pause_turn` → `Stop`；其余 → `Stop`。
- `model_context_window_exceeded` 映射为 `MaxTokens`，让既有 overflow Case1 判定（length/max_tokens 且近零输出）能触发压缩。
- `tool_use` 映射为 `ToolUse` 不影响 usage 锚点：`is_valid_api_anchor` 只排除 Error / Aborted。

## 流式终止

- `error` 事件：解析 `error.type` 与 `error.message`，yield `Err(XyError::Provider(...))` 并终止流；不再等待 `message_stop`。
- `message_stop`：若尚未发 `Done`，兜底发 `Done{Stop}`；已发则跳过。
- 循环正常结束（EOF）：若尚未发 `Done`，兜底发 `Done{Stop}`；不得静默结束。
- 错误消息内嵌 `error.type`，让 `is_retryable_error` 的正则能识别 `overloaded` / `rate limit`；结构化分类留待 S7。

## usage

- 流式合并 `message_start.message.usage` 与 `message_delta.usage` 的 JSON 字段（`cache_creation` 整体覆盖），结束时经 `from_anthropic_usage` 映射。
- `total_tokens = input + cache_read + cache_write + output`，与 `total_context_tokens` 的兜底口径一致；否则 `estimate_context` 会漏算缓存命中部分。
- `from_anthropic_usage` 迁入 Anthropic 插件 `usage` 模块；OpenAI 插件不再持有 Anthropic 语义。

## 风险

- `refusal` / `sensitive` 映射为 Error 会改变既有「成功 Stop」表现：策略拒绝现在进入错误路径（符合协议意图）。
- `pause_turn` 映射为 Stop 只是占位；服务端工具循环未完成时，runtime 不会自动重发。
