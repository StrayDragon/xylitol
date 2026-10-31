---
depends_on: []
title: OpenAI 线协议跟进：结构化输出并行开关与缓存键默认关
needs_specs_change: true
---

# OpenAI 线协议跟进：结构化输出并行开关与缓存键默认关

## Why

权威惯例核对与 live 探测暴露两处偏差：

- OpenAI 结构化输出不支持并行函数调用；设置 `structured_output` 且带 tools 时未发 `parallel_tool_calls: false`，可能被端点拒绝（Azure/OpenAI 文档明示）。
- `prompt_cache_key` 官方默认不发送；当前 generic 默认发送，与官方默认及 pi-ai 惯例不一致（live 探测虽被接受，但默认行为应跟随官方）。

## What Changes

- 结构化输出 + 非空工具列表：Responses 与 Completions 必须 写 `parallel_tool_calls: false`；未设置结构化输出 不得 改变该字段。
- `prompt_cache_key` 默认改为不发送（generic 与 deepseek 一致）；配置 `prompt_cache.enabled: true` 且会话 id 可用时 必须 发送。
- 更新既有 r3020 表述与对应单测。

## Capabilities

`infra-provider`：新增 r3024（结构化输出并行开关）；改写 r3020（缓存键默认关）。

## Impact

- `plugins/xylitol-official-plugin-provider-openai`：wire policy 默认、请求组装与单测。
- 无配置面新增；`prompt_cache.enabled` 语义不变（只是默认值变化）。

## 非目标

- 不做 `tool_choice` 参数。
- 不实现 Anthropic `structured-outputs-2025-11-13` beta 头。
- 不改 Anthropic 缓存断点策略。

## 不承诺

- 端点是否真正执行 `parallel_tool_calls: false` 未做 live 验证（字段形状按官方文档）。
