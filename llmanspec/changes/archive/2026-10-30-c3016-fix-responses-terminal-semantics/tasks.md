# Tasks

## 1. 终止事件映射
- [x] `response.failed` 与顶层 `error` 写入 `terminal_error`
- [x] `response.incomplete` 按 `incomplete_details.reason` 分流
- [x] 终止事件置 `done_emitted`

## 2. 流包装
- [x] 映射后检查 `terminal_error` 并 `yield Err`
- [x] EOF 未见终止事件返回错误

## 3. 测试
- [x] `response.failed` 断言错误与 code
- [x] 流内 `error` 断言错误
- [x] `content_filter` 断言 `Done{Error}`；`max_output_tokens` 断言 `MaxTokens`
- [x] 无终止事件的流断言错误

## 4. 门禁
- [x] `cargo test -p xylitol-official-plugin-provider-openai` 绿
- [x] `just qa` 绿
