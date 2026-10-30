---
depends_on: []
title: Anthropic Messages thinking 签名保真
needs_specs_change: true
branch: sdd/c3012-fix-anthropic-thinking-signature
base_branch: main
base_sha: 77e17c0fb069035d9cb29d8df1d651a30b748609
---

# Anthropic Messages thinking 签名保真

## Why

Anthropic Messages 方言当前把 `AgentPart::Thinking` 回放成 `{"type":"text"}`，流式侧不累积 `signature_delta`，也不发 `XyChunk::ThinkingEnd`。Anthropic 要求 thinking block 原样回传（含 `signature`）；签名丢失会让工具续轮丢失推理，并在 thinking 开启时被 API 拒绝。长工具链的推理连续性因此被削弱。

## What Changes

- 流式：按 content block 累积 thinking 文本与 `signature`（`content_block_start.signature` 或 `content_block_delta.signature_delta`）；`content_block_stop` 发出 `XyChunk::ThinkingEnd { thinking, thinking_signature }`。
- 非流式：`parse_anthropic_response` 对 `thinking` block 先发 `ThinkingDelta`，再发带签名的 `ThinkingEnd`。
- 回放：`AgentPart::Thinking` 带签名 须映射为 `{"type":"thinking","thinking","signature"}`；`redacted` 须映射为 `{"type":"redacted_thinking","data": signature}`；带签名的 thinking 不得再降级为 text block。
- 无签名旧 thinking 不得伪造签名：维持文本降级并记录诊断。
- 清理 `convert_agent_messages_for_anthropic` 的空 `system_parts` 死代码。

## Capabilities

`infra-provider`：新增两条 Anthropic thinking wire 规则（r3010 / r3011）。

## Impact

- 插件：`plugins/xylitol-official-plugin-provider-anthropic` 的 `provider/native/anthropic_messages.rs`。
- 无 SDK 形状变化：`XyChunk::ThinkingEnd` 已携带 `thinking_signature`。
- 兼容：`AgentPart::Thinking.redacted` 目前恒为 false，响应侧 `redacted_thinking` 生产者缺失；本 change 只交付回放分支，生产者缺口记入未决问题。

## 非目标

- 不改 OpenAI 族。
- 不做 Anthropic 终止语义、usage、prompt caching（S2 / S3）。
- 不为旧历史重建或伪造 thinking 签名。

## 不承诺

- 交错 thinking（interleaved thinking）在 runtime 侧仍折叠为单一 Thinking part；本 change 不改变该形状。

## Further Notes

- 权威依据与 live 探测记录见 `research/anthropic-protocol.md`（thinking 回放、caching、stop_reason、错误形状；S2/S3 复用）。
