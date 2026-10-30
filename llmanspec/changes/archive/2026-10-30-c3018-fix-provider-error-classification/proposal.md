---
depends_on: []
title: Provider 错误结构化与重试分类
needs_specs_change: true
branch: sdd/c3018-fix-provider-error-classification
base_branch: main
base_sha: 369684d41fa054c27824f06779d0a3d5987f95c6
---

# Provider 错误结构化与重试分类

## Why

两个方言适配器都把 `OpenAIError` 与 reqwest 错误字符串化进 `XyError::Provider(anyhow)`，HTTP status、`code`、`type` 与 `retry-after` 全部丢失。`is_retryable_error` 只能对消息做正则匹配：`invalid_request_error` 与上下文溢出可能被误判为可重试，`overloaded_error` / 429 的退避也拿不到服务端建议。

## What Changes

- 新增 `XyProviderError`（message / status / code / error_type / retry_after），经 `XyError::provider()` 包装进既有 `XyError::Provider(anyhow)`；`XyError::provider_detail()` 提供结构读取，不改枚举形状。
- 方言适配器 必须 把 HTTP 状态、错误 code/type 与 `retry-after` 填入结构化错误，不得 只保留字符串消息。
- 重试判定 必须 优先使用结构字段：429 与 5xx 可重试，4xx（含上下文溢出）不可重试，`insufficient_quota` / billing 不可重试；字符串正则仅作兜底。
- `retry-after` 必须 参与退避：取服务端建议与指数退避的较大值。

## Capabilities

- `agent-runtime`：新增 r3021（结构化重试判定）。
- `infra-provider`：新增 r3022（provider 错误结构）。

## Impact

- `crates/xylitol-plugin-sdk`：`XyProviderError` 与 `XyError` 辅助方法。
- `crates/xylitol-runtime`：`is_retryable_xy_error` 与 `RetryState` 退避。
- 两个方言插件：错误映射改为结构化。

## 非目标

- 不改 `XyError` 枚举变体形状（避免全仓构造点重写）。
- 不改 TUI / 线协议错误呈现。
- OpenAI 族经 async-openai 拿不到响应头，`retry-after` 仅 Anthropic reqwest 路径可用。

## 不承诺

- `code` / `type` 的取值集合以厂商为准；未知值回落到消息正则。
