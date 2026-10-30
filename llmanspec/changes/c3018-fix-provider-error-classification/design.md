# 设计：Provider 错误结构化与重试分类

## 形状

- `XyProviderError { message, status: Option<u16>, code: Option<String>, error_type: Option<String>, retry_after: Option<Duration> }`，实现 Display / Error。
- 包装路径：`XyError::Provider(anyhow::Error::new(XyProviderError))`；`provider_detail()` 用 `downcast_ref` 取回。枚举形状不变，存量构造点无需改写。

## 映射

- Anthropic 非流式：HTTP status + `retry-after` 头 + body `error.type` / `message`。
- Anthropic 流式 `error` 事件：`error_type` + message，status 未知。
- OpenAI `OpenAIError::ApiError(ApiErrorResponse)`：`status_code`、`api_error.code`、`api_error.type`、message；其余变体回落字符串。
- Responses `response.failed` / 流内 `error`：code + message。

## 重试判定

- `is_retryable_xy_error(&XyError)`：结构字段优先——429 与 5xx 可重试；4xx 不可重试；`insufficient_quota` / `billing` 不可重试；未命中回落 `is_retryable_error(message)`（保留上下文溢出排除）。
- `RetryState::next_delay_with_hint`：`max(指数退避, retry_after)`。

## 风险

- 4xx 一律不可重试可能把某些 409 冲突挡掉；当前无此类路径。
- `retry-after` 单位按秒解析；HTTP-date 形式未支持（厂商实际多用秒）。
