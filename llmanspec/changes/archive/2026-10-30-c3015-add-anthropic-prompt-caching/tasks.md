# Tasks

## 1. 配置类型与透传
- [x] SDK 新增 `XyPromptCacheConfig` / `XyPromptCacheTtl`，`XyModelConfig` 与模型条目增加 `prompt_cache`
- [x] 更新全部 `XyModelConfig` / `XyModelEntryConfig` 字面量
- [x] 内核适配器工厂把配置传给 Anthropic 适配器

## 2. 请求打点
- [x] 启用时 system 末块 / 最后一个 tool / 最后一条消息文本块打 `cache_control`
- [x] `ttl=1h` 时打点携带 `ttl` 并加 beta 头
- [x] `deepseek` 默认关闭；配置可覆盖

## 3. 测试
- [x] body 断言三个断点位置与数量
- [x] `deepseek` 断言无 `cache_control`
- [x] `1h` 断言 `ttl` 与 beta 头
- [x] 配置开关关闭时无打点

## 4. 配置产物与门禁
- [x] 再生成 schema 与示例配置
- [x] `cargo test -p xylitol-official-plugin-provider-anthropic` 绿
- [x] `just qa` 绿
