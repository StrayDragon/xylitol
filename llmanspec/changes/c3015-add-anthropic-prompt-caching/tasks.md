# Tasks

## 1. 配置类型与透传
- [ ] SDK 新增 `XyPromptCacheConfig` / `XyPromptCacheTtl`，`XyModelConfig` 与模型条目增加 `prompt_cache`
- [ ] 更新全部 `XyModelConfig` / `XyModelEntryConfig` 字面量
- [ ] 内核适配器工厂把配置传给 Anthropic 适配器

## 2. 请求打点
- [ ] 启用时 system 末块 / 最后一个 tool / 最后一条消息文本块打 `cache_control`
- [ ] `ttl=1h` 时打点携带 `ttl` 并加 beta 头
- [ ] `deepseek` 默认关闭；配置可覆盖

## 3. 测试
- [ ] body 断言三个断点位置与数量
- [ ] `deepseek` 断言无 `cache_control`
- [ ] `1h` 断言 `ttl` 与 beta 头
- [ ] 配置开关关闭时无打点

## 4. 配置产物与门禁
- [ ] 再生成 schema 与示例配置
- [ ] `cargo test -p xylitol-official-plugin-provider-anthropic` 绿
- [ ] `just qa` 绿
