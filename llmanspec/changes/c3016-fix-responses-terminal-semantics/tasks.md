# Tasks

## 1. 终止事件映射
- [ ] `response.failed` 与顶层 `error` 写入 `terminal_error`
- [ ] `response.incomplete` 按 `incomplete_details.reason` 分流
- [ ] 终止事件置 `done_emitted`

## 2. 流包装
- [ ] 映射后检查 `terminal_error` 并 `yield Err`
- [ ] EOF 未见终止事件返回错误

## 3. 测试
- [ ] `response.failed` 断言错误与 code
- [ ] 流内 `error` 断言错误
- [ ] `content_filter` 断言 `Done{Error}`；`max_output_tokens` 断言 `MaxTokens`
- [ ] 无终止事件的流断言错误

## 4. 门禁
- [ ] `cargo test -p xylitol-official-plugin-provider-openai` 绿
- [ ] `just qa` 绿
