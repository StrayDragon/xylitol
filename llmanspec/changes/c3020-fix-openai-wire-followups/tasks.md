# Tasks

## 1. 规则
- [ ] 新增 r3024（结构化输出并行开关）
- [ ] 改写 r3020（缓存键默认关）

## 2. 实现
- [ ] Responses `apply_text_format` 接收 has_tools 并写 `parallel_tool_calls: false`
- [ ] Completions `apply_structured_output` 接收 has_tools 并写 `parallel_tool_calls: false`
- [ ] `PROMPT_CACHE_KEY` 默认改为 false

## 3. 测试
- [ ] Responses 结构化+tools 断言；无 tools 断言；默认策略无 key
- [ ] Completions 结构化+tools 断言；默认策略无 key；显式开启仍发 key
- [ ] 更新 `completions_body_includes_prompt_cache_key` 为显式开启策略

## 4. 门禁
- [ ] `cargo test -p xylitol-official-plugin-provider-openai` 绿
- [ ] `just qa` 绿
