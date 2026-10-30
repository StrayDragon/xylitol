# Tasks

## 1. SDK 选项
- [x] 新增 `XyStructuredOutput` 与 `XyGenerateOptions.structured_output`
- [x] 更新全部 `XyGenerateOptions` 字面量
- [x] 单测：默认 None

## 2. 请求映射
- [x] Responses 写 `text.format`
- [x] Completions 写 `response_format`
- [x] Anthropic 写 `output_config.format`

## 3. 测试
- [x] Responses body 断言 `text.format` 字段
- [x] Completions body 断言 `response_format`
- [x] Anthropic body 断言 `output_config.format`
- [x] 未设置选项时三族请求体不变

## 4. 门禁
- [x] `cargo test -p xylitol-plugin-sdk -p xylitol-official-plugin-provider-openai -p xylitol-official-plugin-provider-anthropic` 绿
- [x] `just qa` 绿
