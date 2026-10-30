# Anthropic Messages 协议要点（S1 取证；S2/S3 复用）

取证日期：2026-10-30。官方站点 docs.anthropic.com / platform.claude.com 在本环境被区域封锁（web_fetch 返回 "App unavailable"），结论取自官方 SDK 生成类型与 AWS Bedrock 官方镜像文档；仅第三方来源的结论标「未确认」。

## 1. thinking 回放

- block 形状：`{"type":"thinking","thinking","signature"}`；signature 为 opaque，不得解析或改写。
- 回传：按原样、原顺序回传；工具续轮必须回传 thinking block（SDK 注释 "required when using tools with extended thinking"）。
- 前缀校验：block 仅在 system、tools、之前 messages 均未变时有效；任一变更使其后全部 thinking block 失效。
- 流式签名：`content_block_delta` 的 `delta.type="signature_delta"`，紧邻 `content_block_stop` 之前。
- `content_block_start` 是否直接带 signature：未确认；已知签名只走 signature_delta。
- `redacted_thinking`：`{"type":"redacted_thinking","data":<opaque>}`，整块在 `content_block_start` 到达、无 delta；回放要求同 thinking。
- 缺/改签名：400 invalid_request_error，message 含 "Invalid `signature` in `thinking` block"。

## 2. prompt caching（S3 用）

- 断点：`{"type":"ephemeral"}`，可选 `ttl: "5m" | "1h"`，默认 5m。
- 语义：标记块及其之前全部前缀构成一个缓存前缀；每请求最多 4 个断点。
- 可打位置：system blocks、tools、messages content blocks（含 tool_result）。
- 1h：需 `anthropic-beta: extended-cache-ttl-2025-04-11`。
- usage：`input_tokens`、`cache_creation_input_tokens`、`cache_read_input_tokens`、`cache_creation{ephemeral_5m_input_tokens,ephemeral_1h_input_tokens}`；总输入 = 三者之和（input_tokens 不含缓存部分）。
- 最小可缓存前缀：512–4096 tokens，按模型。

## 3. stop_reason（S2 用）

- 7 值：end_turn、max_tokens、stop_sequence、tool_use、pause_turn、refusal、model_context_window_exceeded。
- `pause_turn`：服务端工具采样循环达上限，须把 assistant 响应原样回传继续。
- `model_context_window_exceeded`：超上下文窗口，须走上下文溢出路径。
- `refusal`：分类器介入处理策略违规。
- `sensitive`：官方类型与文档均无；pi issue #978 称实返。存在性未确认。

## 4. 流式错误（S2 用）

- 帧：`event: error`，data 为 `{"type":"error","error":{"type","message"}}`。
- error.type：invalid_request_error(400)、authentication_error(401)、permission_error(403)、not_found_error(404)、rate_limit_error(429)、api_error(500)、overloaded_error(529)、billing_error。
- SSE 已提交后 HTTP 为 200，错误只在流内；SDK 收到 error 即抛错终止，不发 message_stop（官方未明写，实现一致）。

## 5. 非流式错误

- Body：顶层 error 对象（type + message），响应带 request_id。
- 状态映射同上；thinking 相关 400 与普通参数 400 同为 invalid_request_error，需读 message 区分。

## 来源

- https://docs.aws.amazon.com/bedrock/latest/userguide/claude-messages-extended-thinking.html
- https://docs.aws.amazon.com/bedrock/latest/userguide/claude-messages-thinking-encryption.html
- https://docs.aws.amazon.com/bedrock/latest/userguide/prompt-caching.html
- https://platform.claude.com/docs/en/build-with-claude/handling-stop-reasons
- https://platform.claude.com/docs/en/api/errors
- https://unpkg.com/@anthropic-ai/sdk/resources/messages/messages.d.ts
- https://github.com/anthropics/anthropic-sdk-python/blob/main/src/anthropic/_streaming.py
- https://github.com/earendil-works/pi/issues/978
