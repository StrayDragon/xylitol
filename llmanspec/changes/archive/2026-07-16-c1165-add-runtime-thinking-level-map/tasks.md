# Tasks — c1165-add-runtime-thinking-level-map

## 1. 配置与 domain

- [x] 1.1 `ThinkingLevelMap` + `ModelEntry.thinking_level_map` 解析/校验（未知键失败）
- [x] 1.2 `XyModelMeta` 携带 map；`resolve_model_meta` 填入
- [x] 1.3 `resolve_thinking_for_request` 单测（洞 / null / 覆盖默认）

## 2. 调用缝

- [x] 2.1 `XyGenerateOptions` + 扩展 `XyModel` / `LlmAdapter` / bridge `generate_stream`
- [x] 2.2 `react::call_with_retry` 传入当前 level + meta map + budgets
- [x] 2.3 Fake/Mock/测试 stub 实现更新

## 3. Bridge 请求体

- [x] 3.1 OpenAI Completions：`reasoning_effort`
- [x] 3.2 OpenAI Responses：`reasoning.effort`
- [x] 3.3 Anthropic：budget `thinking` 块；Off 省略
- [x] 3.4 请求体断言测（off / medium / 自定义 map）
- [x] 3.5 回归锁：cycle/set 后 resolve → OpenAI `reasoning_effort`；Completions 双请求 off→high 捕获 body

## 4. 校验

- [x] 4.1 `llman sdd validate c1165-add-runtime-thinking-level-map --strict --no-interactive`
- [x] 4.2 `just lint` + 相关 lib/bridge 测
