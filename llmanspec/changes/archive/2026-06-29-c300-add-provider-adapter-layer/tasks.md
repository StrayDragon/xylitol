# c300-add-provider-adapter-layer Tasks

## Phase 1 — 基础设施与接口

- [x] 创建 `src/infra/provider/adapter/mod.rs`，定义 `LlmAdapter` trait 与 `AdapterKind`
- [x] 在 `XyModelConfig` 中新增 `api: Option<String>` 字段
- [x] 更新配置加载逻辑，为 `OpenAi`/`Anthropic` kind 填充默认 `api` 值
- [x] 更新 `src/infra/provider/factory.rs`：根据 `kind + api` 选择 adapter，构造薄壳 provider
- [x] 运行 `cargo check` 与 `cargo clippy`

## Phase 2 — Anthropic Adapter 迁移

- [x] 将 `src/infra/provider/anthropic.rs` 的 SSE 解析逻辑迁移到 `src/infra/provider/adapter/anthropic_messages.rs`
- [x] 将 `AnthropicProvider` 改为持有 `Arc<dyn LlmAdapter>` 的薄壳
- [x] 确保现有 Anthropic provider 测试继续通过
- [x] 运行 `cargo test`

## Phase 3 — OpenAI Responses Adapter（首批重点）

- [x] 创建 `src/infra/provider/adapter/openai_responses.rs`
- [x] 实现 `AgentMessage` → Responses `input` items 的转换
- [x] 实现流式 SSE 事件解析，输出 `XyChunk::ThinkingDelta` / `XyChunk::TextDelta` / `XyChunk::FunctionCall` / `XyChunk::Done`
- [x] 实现非流式 `response.output` 解析
- [x] 为 `OpenAIProvider` 注入 `OpenAiResponsesAdapter`（当 `api = openai-responses`）
- [x] 使用 `tufa` 的真实响应 fixture 编写单元测试
- [x] 运行 `cargo test`

## Phase 4 — OpenAI Chat Completions 占位与清理

- [x] 创建 `src/infra/provider/adapter/openai_completions.rs` 接口占位
- [x] 删除 `src/infra/provider/openai.rs` 中基于 `serde_json::to_value` 的无效 `reasoning_content` workaround
- [x] 确保 `OpenAIProvider` 默认注入 `OpenAiResponsesAdapter`
- [x] 运行 `cargo test` 与 `cargo clippy`

## Phase 5 — 端到端验证

- [x] 使用 `tufa` 上已加载的 reasoning 模型运行 `cargo run -- --model <id> <prompt>`，确认 stderr 输出 `<think>...</think>`
- [x] 验证普通非 reasoning prompt 的 stdout 输出正常
- [x] 运行 `cargo test` 全量测试
- [x] 运行 `cargo clippy --all-targets --all-features`

## Phase 6 — 规范校验与归档准备

- [x] 运行 `llman sdd validate c300-add-provider-adapter-layer --strict --no-interactive`
- [x] 修复校验错误（如有）
- [x] 更新 `proposal.md` 状态为 `ready`
