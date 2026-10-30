---
depends_on: []
title: OpenAI 族缓存亲和（prompt_cache_key / retention）
needs_specs_change: true
branch: sdd/c3017-add-openai-cache-affinity
base_branch: main
base_sha: 94158bdd6d2a351151b8f570d7293a54fc80427a
---

# OpenAI 族缓存亲和（prompt_cache_key / retention）

## Why

OpenAI Responses 与 Completions 都不写 `prompt_cache_key`：wire policy 默认禁止，assembler 只做删除。会话内请求因此缺少路由亲和提示，前缀缓存命中率低于官方推荐做法；`prompt_cache_retention` 也从未发送。

## What Changes

- Responses 与 Completions 方言 必须 在会话 id 可用时发送 `prompt_cache_key`（取 `obs_session.session_id`）。
- `compat: generic` 必须 默认允许该字段，`compat: deepseek` 必须 默认禁止；配置 `prompt_cache.enabled` 必须 可覆盖。
- 配置 `prompt_cache.retention` 为 `24h` 时 必须 发送 `prompt_cache_retention: "24h"`；未配置 必须 省略，跟随官方默认。
- wire policy 由硬编码删除改为「默认允许 + compat/配置门控」。

## Capabilities

- `infra-provider`：新增 r3020（OpenAI 缓存亲和）。
- `runtime-config`：扩展 r3018（prompt_cache 字段增加 `retention`）。

## Impact

- `crates/xylitol-plugin-sdk`：`XyPromptCacheConfig.retention` 与 `XyPromptCacheRetention`。
- `plugins/xylitol-official-plugin-provider-openai`：wire policy 默认值、Responses assembler、Completions body。
- `crates/xylitol-kernel`：`resolve_wire_policy` 应用配置覆盖。
- 配置 schema 与示例模板再生成。

## 非目标

- 不改 Anthropic 缓存打点（S3 已交付）。
- 不做多 key 分发或按请求频次调优。
- 不发送 `prompt_cache_retention: in_memory`（官方默认即省略）。

## 不承诺

- `prompt_cache_key` 与 `store: false` 的组合未获官方书面确认；按字段语义发送。
- 第三方 OpenAI 兼容端若不识别该字段，可能忽略或报错；`compat: deepseek` 已默认关闭。
