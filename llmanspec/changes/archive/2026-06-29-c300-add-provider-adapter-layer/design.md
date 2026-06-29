---
change_id: c300-add-provider-adapter-layer
title: Provider Adapter 层设计
---

# Provider Adapter 层设计

## 目标

1. 将供应商特定的 HTTP、SSE、JSON Schema 处理与 agent runtime 隔离。
2. 把每个支持的供应商 API 转换为项目内部统一的 `XyChunk` 流。
3. 在首批支持 OpenAI Responses API 与 Anthropic Messages API 的前提下，为后续 Chat Completions 适配留出扩展点。
4. 保持 `runtime_protocol::XyModel` 边界不变，agent 循环无需感知 adapter 存在。

## 非目标

- 1.0.0 前不新增 Google、DeepSeek 等供应商家族。
- 不改动 `AgentMessage`、session 历史、wire protocol 词汇。
- 不实现 OAuth / 订阅制认证流程。

## 当前状态

```text
src/infra/provider/
├── openai.rs      # 基于 async-openai 的 Chat Completions，无法读取 reasoning_content
├── anthropic.rs   # reqwest + eventsource-stream，直接解析 SSE
├── fake.rs
├── mock.rs
└── factory.rs     # 根据 XyModelConfig 构造 OpenAIProvider / AnthropicProvider
```

`openai.rs` 与 `anthropic.rs` 都同时承担：请求构造、HTTP 传输、供应商特定解析、转换到 `XyChunk`。

## 目标状态

参考 pi 的 `packages/ai/src/providers/` 组织，把供应商实现下沉为可插拔 adapter：

```text
src/infra/provider/
├── adapter/
│   ├── mod.rs                 # LlmAdapter trait + AdapterKind
│   ├── factory.rs             # 根据配置选择 adapter
│   ├── openai_responses.rs    # OpenAI Responses API（首批重点）
│   ├── anthropic_messages.rs  # Anthropic Messages API（迁移现有逻辑）
│   └── openai_completions.rs  # Chat Completions（第二批，预留接口）
├── openai.rs                  # 薄壳 XyModel，持有 OpenAI 系 adapter
├── anthropic.rs               # 薄壳 XyModel，持有 Anthropic adapter
├── fake.rs
├── mock.rs
└── factory.rs                 # 更新为构造 adapter + 薄壳 provider
```

## `LlmAdapter` Trait

```rust
#[async_trait]
pub trait LlmAdapter: Send + Sync {
    /// 供应商/adapter 名称，用于日志与诊断。
    fn name(&self) -> &str;

    /// 流式调用，返回内部统一的 XyChunk 流。
    async fn generate_stream(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[XyToolSchema],
    ) -> Result<XyStream, XyError>;

    /// 非流式调用，同样返回 XyChunk 流（一次性产出）。
    async fn generate(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[XyToolSchema],
    ) -> Result<XyStream, XyError>;
}
```

两个方法都返回 `XyStream`，因此上层的 `XyModel` 实现可以是通用的薄壳。

## Adapter 选择

`XyModelConfig` 已包含 `kind`、`api_key`、`model`、`base_url`。新增可选 `api` 字段：

```rust
pub struct XyModelConfig {
    pub kind: XyModelKind,
    pub api_key: String,
    pub model: String,
    pub base_url: Option<String>,
    pub api: Option<String>, // 新增
}
```

`AdapterKind` 由 `(kind, api)` 推导：

| `kind`      | `api` 默认值           | 选用的 adapter             |
|-------------|------------------------|---------------------------|
| `OpenAi`    | `openai-responses`     | `OpenAiResponsesAdapter`  |
| `OpenAi`    | `openai-completions`   | `OpenAiCompletionsAdapter` |
| `Anthropic` | `anthropic-messages`   | `AnthropicMessagesAdapter` |

配置加载时若未指定 `api`，则按上表填充默认值。后续可通过 `--model <name>:<api>` 或 YAML `api` 字段覆盖。

## 请求/响应流程

1. `agent::runtime::react` 调用 `model.generate_stream(...)`（`Arc<dyn XyModel>`）。
2. `OpenAIProvider` / `AnthropicProvider` 将调用转发给内部持有的 `Arc<dyn LlmAdapter>`。
3. Adapter 完成：
   - 将 `AgentMessage` 转换为供应商请求 JSON；
   - 通过 `reqwest::Client` 发送 HTTP 请求；
   - 解析 SSE / JSON 事件；
   - 输出 `XyChunk::TextDelta`、`XyChunk::ThinkingDelta`、`XyChunk::FunctionCall`、`XyChunk::Done`。
4. Agent 循环与今日一样消费 `XyChunk`。

## OpenAI Responses API 适配细节

### 请求体

使用 `/v1/responses`，请求体示例：

```json
{
  "model": "Qwen3.6-35B-A3B/UD-Q5_K_XL-think-coding",
  "input": [
    { "role": "user", "content": "1+1=?" }
  ],
  "tools": [...],
  "stream": true
}
```

`input` items 由 `AgentMessage` 转换而来。系统 prompt 作为 `system` item；tool result 作为 `function_call_output` item。

### 流式事件映射

| SSE 事件 | 处理 |
|----------|------|
| `response.created` / `response.in_progress` | 忽略或用于生命周期事件 |
| `response.output_item.added` (type=reasoning) | 开始为 item_id 收集 reasoning 片段 |
| `response.reasoning_text.delta` | `XyChunk::ThinkingDelta(delta)` |
| `response.output_item.added` (type=message) | 开始为 item_id 收集文本片段 |
| `response.content_part.added` / `response.output_text.delta` | `XyChunk::TextDelta(delta)` |
| `response.function_call_arguments.delta` | 累积到对应 function_call item |
| `response.output_item.done` (type=function_call) | `XyChunk::FunctionCall { name, args, id }` |
| `response.completed` | `XyChunk::Done` |

### 非流式映射

解析 `response.output` 数组：

- `type: reasoning` → 遍历 `content[].text`，输出 `XyChunk::ThinkingDelta`。
- `type: message` → 遍历 `content[].text`，输出 `XyChunk::TextDelta`。
- `type: function_call` → 输出 `XyChunk::FunctionCall`。
- 最后输出 `XyChunk::Done`。

## Anthropic Messages API 适配细节

把当前 `anthropic.rs` 的 SSE 解析逻辑整体迁移到 `adapter/anthropic_messages.rs`，行为保持不变：

- `content_block_delta` with `type: text_delta` → `XyChunk::TextDelta`
- `content_block_delta` with `type: thinking` → `XyChunk::ThinkingDelta`
- `content_block_delta` with `input_json_delta` → 累积 tool args
- `message_delta` / `message_stop` → 输出 tool calls 与 `XyChunk::Done`

## OpenAI Chat Completions 预留

`adapter/openai_completions.rs` 先创建文件与空实现/接口占位，具体实现放到后续变更。其职责是：

- 基于 `reqwest` + 原始 SSE 解析；
- 读取 `delta.reasoning_content` 并输出 `XyChunk::ThinkingDelta`；
- 读取 `delta.content` 并输出 `XyChunk::TextDelta`；
- 累积 `delta.tool_calls` 并输出 `XyChunk::FunctionCall`。

## 上层 Provider 薄壳

`OpenAIProvider` 与 `AnthropicProvider` 不再关心具体协议，仅：

```rust
pub struct OpenAIProvider {
    adapter: Arc<dyn LlmAdapter>,
}
```

`generate_stream` / `generate` 直接转发给 adapter。工厂根据 `XyModelConfig.api` 决定为 `OpenAIProvider` 注入 `OpenAiResponsesAdapter` 还是 `OpenAiCompletionsAdapter`。

## 迁移步骤

1. 创建 `infra/provider/adapter/mod.rs` 与 `LlmAdapter` trait。
2. 将 `anthropic.rs` 的 SSE 解析迁移到 `adapter/anthropic_messages.rs`；`anthropic.rs` 改为薄壳。
3. 创建 `adapter/openai_responses.rs`，实现 Responses API 请求构造、SSE 解析、非流式解析。
4. 创建 `adapter/openai_completions.rs` 占位。
5. 更新 `XyModelConfig` 增加 `api` 字段；更新 config loader 填充默认值。
6. 更新 `infra/provider/factory.rs` 使用 adapter factory。
7. 删除 `openai.rs` 中基于 `serde_json::to_value` 的无效 reasoning_content 提取代码。
8. 新增 adapter 级单元测试，使用 `tufa` 的 SSE fixture。

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| 重写 SSE 解析引入回归 | Anthropic adapter 尽量平移代码；Responses adapter 用真实 `tufa` fixture 做单元测试 |
| OpenAI Chat Completions 暂时仍依赖 async-openai | 先预留 adapter 接口，后续变更替换；当前不阻塞 reasoning 展示 |
| 配置 `api` 字段用户误用 | 加载时校验只允许 `openai-responses` / `openai-completions` / `anthropic-messages` |
| Responses API 与 Chat Completions 的消息格式差异 | 集中处理在各自 adapter 的 `AgentMessage` 转换函数中 |

## 验证

- `cargo test` 通过。
- `cargo clippy --all-targets --all-features` 通过。
- `llman sdd validate c300-add-provider-adapter-layer --strict --no-interactive` 通过。
