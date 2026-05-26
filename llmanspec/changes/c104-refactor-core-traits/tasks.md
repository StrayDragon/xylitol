# Tasks: c104-refactor-core-traits

## 类型定义

- [ ] 创建 `src/agent/types.rs`：定义 `XyContent`、`XyPart`（Text/Thinking/FunctionCall/FunctionResponse）、`XyChunk`（streaming chunk）
- [ ] 创建 `src/agent/error.rs`：定义 `XyError`（含 `XyToolError` 子类型），用 `thiserror` 派生
- [ ] 创建 `src/agent/traits.rs`：定义 `XyModel` trait（`generate_stream` 方法）和 `XyTool` trait（`name`/`description`/`schema`/`execute`）

## Tool 迁移

- [ ] 迁移 `src/agent/tools/read.rs`：`impl Tool` → `impl XyTool`
- [ ] 迁移 `src/agent/tools/write.rs`
- [ ] 迁移 `src/agent/tools/edit.rs`
- [ ] 迁移 `src/agent/tools/bash.rs`
- [ ] 迁移 `src/agent/tools/grep.rs`
- [ ] 迁移 `src/agent/tools/find.rs`
- [ ] 迁移 `src/agent/tools/ls.rs`
- [ ] 迁移 `src/agent/tools/mod.rs`：`ToolRegistry` 持有 `Arc<dyn XyTool>`
- [ ] 迁移 `src/agent/tools/patch.rs`：测试中的 mock ToolContext 替换

## Wrapper 迁移

- [ ] 迁移 `src/infra/security/mod.rs`：`SecurityToolWrapper` 包装 `XyTool`
- [ ] 迁移 `src/infra/skills/mcp.rs`：`McpToolAdapter` 实现 `XyTool`
- [ ] 迁移 `src/interface/tui/approval.rs`：`SecureApprovalToolWrapper` 包装 `XyTool`

## Provider 迁移

- [ ] 修改 c103 的 `provider/openai.rs`：`impl Llm` → `impl XyModel`
- [ ] 修改 c103 的 `provider/anthropic.rs`：`impl Llm` → `impl XyModel`
- [ ] 修改 `src/agent/provider/fake.rs`：`impl Llm` → `impl XyModel`

## 兼容层

- [ ] 创建 `src/agent/compat.rs`：`XyModelToLlm` adapter（`impl adk_core::Llm for XyModelToLlm`）供 `adk-runner` 使用
- [ ] 创建 `src/agent/compat.rs`：`XyToolToTool` adapter（`impl adk_core::Tool for XyToolToTool`）供 `adk-runner` 使用

## 验证

- [ ] `cargo build` 通过
- [ ] `cargo test` 通过
- [ ] 确认 `src/` 中直接 `use adk_core` 只出现在 `compat.rs` 和 `loop.rs`
- [ ] `just qa` 通过
