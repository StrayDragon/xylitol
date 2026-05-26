# Tasks: c103-refactor-provider-layer

## 准备

- [ ] 阅读 `patches/adk-model/src/openai/` 理解当前 OpenAI 客户端的请求/响应映射逻辑
- [ ] 阅读 `patches/adk-model/src/anthropic/` 理解当前 Anthropic 客户端的映射逻辑
- [ ] 确认 `adk_core::Llm` trait 签名及 `LlmRequest`/`LlmResponse`/`Part` 类型定义

## 实现

- [ ] 创建 `src/agent/provider/openai.rs`：基于 `async-openai` 实现 `adk_core::Llm`，支持 streaming + tool_calls + base_url
- [ ] 创建 `src/agent/provider/anthropic.rs`：基于 `reqwest` 实现 `adk_core::Llm`，支持 SSE streaming + tool_use + thinking + base_url
- [ ] 创建 `src/agent/provider/mod.rs`：统一导出 + provider 工厂
- [ ] 修改 `src/agent/model.rs`：`ModelConfig::build()` 使用新 provider 替代 `adk_model::OpenAIClient` / `AnthropicClient`

## 清理

- [ ] 从 `Cargo.toml` 移除 `adk-model` 依赖和 `[patch.crates-io]` 段
- [ ] 在 `Cargo.toml` 中添加 `async-openai` 直接依赖（已是传递依赖，升为直接依赖）
- [ ] 删除 `patches/adk-model/` 目录（67 个文件）

## 验证

- [ ] `cargo build` 通过，无 `adk_model` import 残留
- [ ] `cargo test` 通过（含 `FakeProvider` 和 `MockLlm` 测试）
- [ ] 手动测试 OpenAI streaming + tool calling（`just run`）
- [ ] 手动测试 Anthropic streaming + thinking（若有 key）
- [ ] `just qa` 通过
