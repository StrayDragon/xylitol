# Tasks

## 1. SDK 选项
- [ ] 新增 `XyStructuredOutput` 与 `XyGenerateOptions.structured_output`
- [ ] 更新全部 `XyGenerateOptions` 字面量
- [ ] 单测：默认 None

## 2. 请求映射
- [ ] Responses 写 `text.format`
- [ ] Completions 写 `response_format`
- [ ] Anthropic 写 `output_config.format`

## 3. 测试
- [ ] Responses body 断言 `text.format` 字段
- [ ] Completions body 断言 `response_format`
- [ ] Anthropic body 断言 `output_config.format`
- [ ] 未设置选项时三族请求体不变

## 4. 门禁
- [ ] `cargo test -p xylitol-plugin-sdk -p xylitol-official-plugin-provider-openai -p xylitol-official-plugin-provider-anthropic` 绿
- [ ] `just qa` 绿
