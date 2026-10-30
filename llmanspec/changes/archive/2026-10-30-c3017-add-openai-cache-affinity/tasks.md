# Tasks

## 1. 配置与策略
- [x] SDK 增加 `XyPromptCacheRetention` 与 `XyPromptCacheConfig.retention`
- [x] OpenAI wire policy 增加 retention 字段，`PROMPT_CACHE_KEY` 默认改为 true
- [x] 内核 `resolve_wire_policy` 应用配置覆盖

## 2. 请求注入
- [x] Responses assembler 写 `prompt_cache_key` 与 `prompt_cache_retention`
- [x] Completions 流式与非流式写同一对字段

## 3. 测试
- [x] Responses body 断言 key 存在、无 retention
- [x] retention 配置断言字段值
- [x] deepseek 断言无 key；enabled=false 断言无 key
- [x] Completions body 断言 key
- [x] wire policy 默认值断言更新

## 4. 配置产物与门禁
- [x] 再生成 schema 与示例配置
- [x] `cargo test -p xylitol-official-plugin-provider-openai -p xylitol-kernel` 绿
- [x] `just qa` 绿
