---
depends_on: []
title: Anthropic Messages prompt caching 打点
needs_specs_change: true
branch: sdd/c3015-add-anthropic-prompt-caching
base_branch: main
base_sha: 1e0cffc88468dded2d5ad50dcd1f7d0b3cd1c08e
---

# Anthropic Messages prompt caching 打点

## Why

Anthropic 请求当前零缓存：`system` 是纯字符串，`tools[]` 与消息内容块都没有 `cache_control`。S2 已让 usage 如实上报 `cache_read_input_tokens`，但请求从不产生缓存命中，长系统提示与工具定义每轮都按全价计费并抬高延迟。

## What Changes

- 配置面：`models.*.prompt_cache` 接受可选 `enabled`（布尔）与 `ttl`（`5m` / `1h`），进入模型配置并生成配置 schema 产物。
- 生效默认：`compat: generic` 默认启用，`compat: deepseek` 默认关闭；配置显式 `enabled` 覆盖默认。
- 打点：启用时在 `system` 末块、最后一个 tool 定义、最后一条消息的文本块（user / assistant / tool_result）各打 `{"type":"ephemeral"}`；每请求 必须 不超过 4 个断点。
- `ttl: 1h` 时 `cache_control` 携带 `ttl` 且请求头 必须 带 `anthropic-beta: extended-cache-ttl-2025-04-11`。
- 缓存命中后 usage 的 `cache_read` 由 S2 的映射自然非零。

## Capabilities

- `infra-provider`：新增 r3017（Anthropic 缓存断点与开关）。
- `runtime-config`：新增 r3018（模型条目 prompt_cache 字段）。

## Impact

- `crates/xylitol-plugin-sdk`：`XyPromptCacheConfig` / `XyPromptCacheTtl`、`XyModelConfig.prompt_cache`、模型条目字段。
- `crates/xylitol-kernel`：适配器工厂把缓存配置传给 Anthropic 适配器。
- `plugins/xylitol-official-plugin-provider-anthropic`：请求体打点与 beta 头。
- 配置 schema 与示例模板再生成。

## 非目标

- OpenAI 族 `prompt_cache_key`（S5）。
- 不做自动 TTL 选择或按消息长度裁剪断点。
- 不给图片块打点。

## 不承诺

- 第三方 Anthropic 兼容端可能拒绝 `cache_control`；本 change 提供关闭开关，但不探测对端能力。
