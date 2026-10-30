---
depends_on: []
title: OpenAI Responses 终止事件语义
needs_specs_change: true
branch: sdd/c3016-fix-responses-terminal-semantics
base_branch: main
base_sha: 8396d108bf7e01fd0a697d24117eb748f71097e2
---

# OpenAI Responses 终止事件语义

## Why

Responses 流式方言只处理 `response.completed` 与 `response.incomplete`：`response.failed` 与流内 `error` 被丢弃，流静默结束且没有 `Done`；`response.incomplete` 一律映射为长度截断，未看 `incomplete_details.reason`，`content_filter` 被误当成 `MaxTokens`。失败与内容过滤因此无法被上层识别为重试或错误。

## What Changes

- `response.failed` 必须 转为错误，消息保留 `response.error.code` 与 `response.error.message`。
- 流内 `error` 事件 必须 转为错误，消息保留 `code` 与 `message`。
- `response.incomplete` 必须 按 `incomplete_details.reason` 分流：`max_output_tokens` 映射为长度截断；`content_filter` 映射为错误。
- 异常 EOF（未见终止事件）必须 返回错误，不得 静默当作完成。
- 已发终止 `Done` 后 不得 重复发出。

## Capabilities

`infra-provider`：新增 r3019（Responses 终止事件语义）。

## Impact

- `plugins/xylitol-official-plugin-provider-openai`：`provider/native/openai_responses.rs` 的事件映射与流包装。
- 无 SDK 形状变化。

## 非目标

- 不改 Completions 终止语义。
- 不做结构化错误分类（S7）。
- 不做缓存亲和（S5）。

## 不承诺

- `incomplete_details.reason` 的其他取值（`max_messages` / `steered`）按长度截断处理；官方语义差异未逐一区分。
