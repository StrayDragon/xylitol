# 设计：Responses 终止事件语义

## 错误通道

- `map_responses_sse_event` 保持返回 `Vec<XyChunk>`（既有 13 个单测不重写），在 `ResponsesStreamState` 增加 `terminal_error: Option<XyError>` 与 `done_emitted: bool`。
- 流包装在每次映射后检查 `terminal_error`：有则 `yield Err` 并终止，不再 yield 后续 chunk。

## 事件映射

- `response.failed`：取 `response.error.code` / `message`，生成 `XyError::Provider`；code 内嵌消息以便 `is_retryable_error` 识别 `server_error` / `rate_limit_exceeded`。
- `error`（顶层）：取 `code` / `message`，同上。
- `response.incomplete`：`content_filter` → `Done{Error}`；其余（含 `max_output_tokens`）→ `Done{MaxTokens}`。
- `finish_responses_terminal` 置 `done_emitted = true`。

## EOF

- 循环结束且 `done_emitted=false` 且无 `terminal_error` → `Err("... ended without terminal event")`，避免截断流被当成成功回合。

## 风险

- 第三方 Responses 兼容端若省略终止事件会被判为错误；官方 Responses 恒发终止事件。
