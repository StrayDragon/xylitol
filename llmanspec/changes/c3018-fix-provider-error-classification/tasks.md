# Tasks

## 1. SDK 形状
- [ ] 新增 `XyProviderError` 与 `XyError::provider` / `provider_detail`
- [ ] 单测：包装后可 downcast 取回结构字段

## 2. 适配器映射
- [ ] Anthropic 非流式：status、retry-after、error.type
- [ ] Anthropic 流式 `error`：error_type
- [ ] OpenAI `ApiErrorResponse`：status、code、type
- [ ] Responses `failed` / `error` 事件：code
- [ ] 单测：wiremock 529 + retry-after 断言结构字段

## 3. 运行时判定与退避
- [ ] `is_retryable_xy_error` 结构优先
- [ ] `RetryState::next_delay_with_hint`
- [ ] react 调用点改用结构化判定与 hint
- [ ] 单测：429/529/5xx 可重试；400/上下文溢出/配额不可重试；hint 取较大值

## 4. 门禁
- [ ] `cargo test -p xylitol-plugin-sdk -p xylitol-runtime -p xylitol-official-plugin-provider-anthropic -p xylitol-official-plugin-provider-openai` 绿
- [ ] `just qa` 绿
