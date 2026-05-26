# Design: c103-refactor-provider-layer

## 架构决策

### OpenAI Provider

使用 `async-openai` crate 直接调用，原因：
- 成熟度高（7k+ stars、活跃维护）
- xylitol 已间接依赖它（通过 adk-model）
- 支持 Responses API / Chat Completions API / streaming

关键映射：
- `async_openai::types::CreateChatCompletionRequest` → `adk_core::LlmRequest`
- `async_openai::types::ChatCompletionStream` → `Stream<Item = LlmResponse>`
- `tool_calls` chunk → `Part::FunctionCall`

### Anthropic Provider

使用 `reqwest` 手写 HTTP 客户端，原因：
- Anthropic Messages API 结构简洁（单一 endpoint）
- 避免引入新的第三方 agent SDK 封装
- SSE streaming 只需解析 `event: content_block_delta` / `message_stop`

关键映射：
- `{"role": "user", "content": [...]}` → `adk_core::Content`
- `content_block_delta.type == "tool_use"` → `Part::FunctionCall`
- `content_block_delta.type == "text_delta"` → `Part::Text`
- `content_block_delta.type == "thinking"` → `Part::Thinking`

### 兼容层

两个 provider 均实现 `adk_core::Llm` trait，保持与 `adk-runner::Runner` 的兼容。
这是有意的——在阶段二（c104）定义自有 trait 之前，先保持运行时稳定。

### Tool Call Streaming 解析

这是 adk-model patch 修复的核心问题。自建 provider 中：
- OpenAI: `async-openai` 的 stream 已正确处理 `\n` token，无需特殊处理
- Anthropic: 手写 SSE parser 中保留所有 whitespace token，不做 trim

### base_url 支持

两个 provider 都必须支持 `base_url` 配置覆盖（用于代理/自托管场景）：
- OpenAI: `async_openai::config::OpenAIConfig::with_api_base()`
- Anthropic: 替换 `https://api.anthropic.com` 前缀

## 迁移路径

```
Before:  ModelConfig::build() → adk_model::OpenAIClient / AnthropicClient
After:   ModelConfig::build() → provider::openai::XyOpenAIProvider / provider::anthropic::XyAnthropicProvider
         (both impl adk_core::Llm)
```

对 `agent/loop.rs`、`agent/planner.rs`、tool 层完全透明。
