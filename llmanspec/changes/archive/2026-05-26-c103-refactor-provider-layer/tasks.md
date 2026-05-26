# Tasks: c103-refactor-provider-layer

## 准备

- [x] 阅读 `patches/adk-model/src/openai/` 理解当前 OpenAI 客户端的请求/响应映射逻辑
- [x] 阅读 `patches/adk-model/src/anthropic/` 理解当前 Anthropic 客户端的映射逻辑
- [x] 确认 `adk_core::Llm` trait 签名及 `LlmRequest`/`LlmResponse`/`Part` 类型定义

## 实现

- [x] 创建 `src/agent/provider/openai.rs`：基于 `reqwest` 实现 `adk_core::Llm`，支持 streaming + tool_calls + base_url
- [x] 创建 `src/agent/provider/anthropic.rs`：基于 `reqwest` 实现 `adk_core::Llm`，支持 SSE streaming + tool_use + thinking + base_url
- [x] 创建 `src/agent/provider/mod.rs`：统一导出 + provider 工厂
- [x] 修改 `src/agent/model.rs`：`ModelConfig::build()` 使用新 provider 替代 `adk_model::OpenAIClient` / `AnthropicClient`

## 清理

- [x] 从 `Cargo.toml` 移除 `adk-model` 依赖和 `[patch.crates-io]` 段
- [x] 在 `Cargo.toml` 中添加 `reqwest`、`eventsource-stream`、`async-stream` 直接依赖
- [x] 删除 `patches/adk-model/` 目录（67 个文件）

## 验证

- [x] `cargo build` 通过，无 `adk_model` import 残留
- [x] `cargo test` 通过（306 passed, 0 failed）
- [x] 手动测试 OpenAI streaming + tool calling（`just run`）(cancelled — 需要 API key，已在 c104/c105 合并中通过集成测试覆盖)
- [x] 手动测试 Anthropic streaming + thinking（若有 key）(cancelled — 同上)
- [x] `cargo clippy` 零警告
