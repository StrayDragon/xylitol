---
depends_on: []
title: Anthropic Messages 终止语义与 usage 保真
needs_specs_change: true
branch: sdd/c3013-fix-anthropic-terminal-semantics
base_branch: main
base_sha: 11a392f3842c71dec815e1c5a673bfcc5baba6f4
---

# Anthropic Messages 终止语义与 usage 保真

## Why

Anthropic 方言的流式终止语义不完整：`event: error`（如 `overloaded_error`）被忽略，流静默结束且没有 `Done`；`stop_reason` 只区分 `max_tokens`，`model_context_window_exceeded` 被当成正常 Stop（压缩永不触发），`refusal` / `sensitive` 被当成成功 Stop（策略拒绝被隐藏）；usage 恒报 `PromptCacheRead::Tokens(0)`，忽略 `cache_read_input_tokens` / `cache_creation_input_tokens`。仓库里已有 `from_anthropic_usage`，却放在 OpenAI 插件里且从未被 Anthropic 适配器调用。

## What Changes

- 流式 `event: error` 必须转为 `Err(XyError::Provider)`，消息携带 `error.type` 与 `error.message`，不得静默结束。
- `message_stop` 与异常 EOF 必须兜底发出 `Done`；已发 `Done` 后不得重复。
- `stop_reason` 必须全映射：`max_tokens` / `model_context_window_exceeded` → `MaxTokens`；`tool_use` → `ToolUse`；`refusal` / `sensitive` → `Error`；`pause_turn` → `Stop`（自动重发语义未实现，留注释）。
- usage 必须经 `from_anthropic_usage` 上报：`cache_read_input_tokens` → cache read，`cache_creation_input_tokens` → cache write，`cache_creation.ephemeral_1h_input_tokens` → 1h 分量；总输入口径含缓存（`input + cache_read + cache_write + output`）。
- 把 `from_anthropic_usage` 迁入 Anthropic 插件；删除 OpenAI 插件中的跨族实现与其测试。

## Capabilities

`infra-provider`：新增三条 wire 规则（r3012 / r3013 / r3014）。

## Impact

- `plugins/xylitol-official-plugin-provider-anthropic`：流式终止、stop_reason、usage 映射、新增 `usage` 模块。
- `plugins/xylitol-official-plugin-provider-openai`：删除 `from_anthropic_usage` 与对应单测。
- 无 SDK 形状变化。

## 非目标

- 不做 prompt caching 打点（S3）。
- 不做结构化错误分类（S7）。
- 不实现 `pause_turn` 自动重发（需要 runtime 配合）。
- `sensitive` 的存在性仅有第三方报告，映射为 `Error` 属保守处理。

## 不承诺

- 流式 `error` 之后不再等待 `message_stop`：按官方 SDK 行为立即终止。

## Further Notes

- 协议取证见 `llmanspec/changes/archive/2026-10-30-c3012-fix-anthropic-thinking-signature/research/anthropic-protocol.md` 第 3–5 节。
