---
change_id: c300-add-provider-adapter-layer
title: 引入 LLM Provider Adapter 层以隔离供应商 API 差异
status: proposed
priority: 300
depends_on:
  - c295-consolidate-domain-entities-and-unify-xy-prefix
author: agent
created: 2026-06-29
---

# 引入 LLM Provider Adapter 层以隔离供应商 API 差异

## 背景与动机

当前 `infra/provider/` 下只有 `openai.rs` 和 `anthropic.rs` 两个硬编码实现，它们各自承担 HTTP/SSE 传输、供应商特定响应解析、转换为 `XyChunk` 这三件事。这在早期只支持 OpenAI Chat Completions 与 Anthropic Messages 时够用，但在修复 thinking/reasoning 展示问题的过程中暴露出三个瓶颈：

1. **非标准 reasoning 字段无法解析**：llama.cpp 等 OpenAI 兼容端点在 Chat Completions 的 `delta.reasoning_content` 中返回思考内容，而我们使用的 `async-openai 0.41.1` 在反序列化时会丢弃未知字段，导致拿不到 thinking。
2. **OpenAI Responses API 无法接入**：本地 `tufa` 服务器已经暴露 `/v1/responses`，其请求/响应结构（`input` items、`output` items、reasoning items、function-call item 配对）与 Chat Completions 完全不同，无法塞进现有实现。
3. **后续扩展困难**：1.0.0 之后若需支持 Google、DeepSeek 等供应商，每个都要重复写一套 HTTP+解析+转换。

参考 pi 的 `packages/ai` 组织方式：它把每个供应商 API 封装成独立的 provider 文件（`openai-completions.ts`、`openai-responses.ts`、`anthropic.ts` 等），通过 `Api` 类型注册到统一注册表，对外输出统一的事件流。我们也需要类似的 adapter 层：由它负责供应商特定的请求构造、流式/非流式调用、响应解析，并转换为项目内部统一的 `XyChunk` 流。`XyModel` trait 继续作为 agent 循环看到的窄接口。

## 变更内容

在 `infra/provider/` 下新增 `adapter/` 子层：

- `adapter/mod.rs`：定义 `LlmAdapter` trait，暴露 `generate_stream` / `generate` 两个方法，返回 `XyStream`。
- `adapter/openai_responses.rs`：**首批重点实现**，基于 `reqwest` + `eventsource-stream` 调用 `/v1/responses`，解析 `response.output_item.added`、`response.reasoning_text.delta`、`response.output_text.delta` 等事件，输出 `XyChunk`。
- `adapter/anthropic_messages.rs`：把当前 `anthropic.rs` 的 SSE 解析逻辑迁移进来，保持现有行为。
- `adapter/openai_completions.rs`（可选/第二批）：用原始 SSE 解析替代 `async-openai`，以支持 `reasoning_content` 等扩展字段；本次可先做接口预留，后续再完整替换。
- `adapter/factory.rs`：根据 `XyModelConfig` 的 `api` 字段选择对应 adapter。

调整上层：

- `OpenAIProvider` / `AnthropicProvider` 变为持有 `Arc<dyn LlmAdapter>` 的薄壳，继续实现 `XyModel`。
- `XyModelConfig` 增加可选 `api` 字段，配置加载时根据 `kind` 设置默认值：`openai` → `openai-responses`（或保留 `openai-completions` 作为默认），`anthropic` → `anthropic-messages`。
- 删除 `openai.rs` 中基于 `serde_json::to_value` 的无效 workaround。

## 首批支持范围

- **OpenAI Responses API**：重点支持，因为 `tufa` 已暴露该端点，且它是 OpenAI 官方新方向（gpt-5 等模型优先）。
- **Anthropic Messages API**：保持现有能力，仅做代码迁移。

明确不支持的（1.0.0 后考虑）：

- Google Generative AI、DeepSeek、NVIDIA 等供应商。
- OAuth / 订阅制认证流程。
- 图片生成模型接口。

## 受影响能力

- `provider-integration`（更新）：provider 构造与模型注册表 wiring。
- `provider-adapter`（新增）：供应商特定 API 的请求/响应处理。

## 影响面

- **agent 循环**：无变化，仍消费 `Arc<dyn XyModel>` 输出的 `XyChunk`。
- **协议/事件**：无变化，`XyEvent` 已有 `ThinkingDelta` / `MessageUpdate`。
- **CLI/print 模式**：直接受益，`openai-responses` adapter 可正确输出 reasoning items，`anthropic` adapter 保持 thinking 输出。
- **测试**：现有 provider 测试下沉到 adapter 级单元测试；`FakeProvider` / `MockProvider` 不变。
- **供应商范围规则**：不变；1.0.0 前仍只支持 OpenAI 兼容 API 与 Anthropic API。adapter 层只是让这两大家族内部更易于扩展。

## 验收标准

- `cargo test` 通过。
- `tufa` 上的 reasoning 模型通过 `--model <model>` 运行时，`print` 模式能在 stderr 展示 `<think>...</think>` 包裹的 reasoning 内容。
- `cargo clippy --all-targets --all-features` 无新增 warning。
- `llman sdd validate c300-add-provider-adapter-layer --strict --no-interactive` 通过。
