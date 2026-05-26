---
depends_on: []
---

# c103-refactor-provider-layer

## Why

`adk-model` 是当前唯一需要 `[patch.crates-io]` 本地 vendor 的 crate（67 个文件），
只为修复 `ToolCallBuffer::flush_as_emit()` 中一行 `trim` → `is_empty` 的 bug。
每次上游发版都需要 rebase 整个 vendor 目录，严重拖累迭代效率。

`adk-model` 本质上只是 `async-openai` 和 `adk-anthropic` 的封装层。
xylitol 仅使用 `OpenAIClient`/`AnthropicClient` 两个具体类型，
完全可以直接调用底层 SDK 或手写 HTTP 客户端来替代。

## What Changes

1. 新增 `src/agent/provider/` 模块，直接基于 `async-openai` 实现 OpenAI provider
2. 新增基于 `reqwest` 的 Anthropic provider（Anthropic API 简洁，不需要重量级 SDK）
3. 这两个 provider 均实现 `adk_core::Llm` trait（保持与现有 agent loop 兼容）
4. 从 `Cargo.toml` 移除 `adk-model` 依赖和 `[patch.crates-io]` 段
5. 删除 `patches/adk-model/` 整个目录
6. 更新 `agent/model.rs` 中的 `ModelConfig::build()` 工厂方法

## Capabilities

- agent-runtime（provider 工厂与 LLM 客户端）

## Impact

- **高收益**：立即消除 patch 维护债务，删除 67 个 vendor 文件
- **低风险**：provider 层是叶子依赖，不影响上层 agent loop / tool / UI
- **兼容性**：继续实现 `adk_core::Llm`，其余代码零改动
