# Tasks

## 1. 规则
- [x] 新增 r3024（结构化输出并行开关）
- [x] 改写 r3020（缓存键默认关）

## 2. 实现
- [x] Responses `apply_text_format` 接收 has_tools 并写 `parallel_tool_calls: false`
- [x] Completions `apply_structured_output` 接收 has_tools 并写 `parallel_tool_calls: false`
- [x] `PROMPT_CACHE_KEY` 默认改为 false

## 3. 测试
- [x] Responses 结构化+tools 断言；无 tools 断言；默认策略无 key
- [x] Completions 结构化+tools 断言；默认策略无 key；显式开启仍发 key
- [x] 更新 `completions_body_includes_prompt_cache_key` 为显式开启策略

## 4. 门禁
- [x] `cargo test -p xylitol-official-plugin-provider-openai` 绿
- [x] `just qa` 绿
