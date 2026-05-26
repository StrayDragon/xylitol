# Tasks: c104-refactor-core-traits

## 类型定义

- [x] 创建 `src/agent/types.rs`：定义 `XyContent`、`XyPart`（Text/Thinking/FunctionCall/FunctionResponse）、`XyChunk`（streaming chunk）
- [x] 创建 `src/agent/error.rs`：定义 `XyError`（含 `XyToolError` 子类型），用 `thiserror` 派生
- [x] 创建 `src/agent/traits.rs`：定义 `XyModel` trait（`generate_stream` 方法）和 `XyTool` trait（`name`/`description`/`schema`/`execute`）

## Tool 迁移

- [x] 迁移 `src/agent/tools/read.rs`：`impl Tool` → `impl XyTool`
- [x] 迁移 `src/agent/tools/write.rs`
- [x] 迁移 `src/agent/tools/edit.rs`
- [x] 迁移 `src/agent/tools/bash.rs`
- [x] 迁移 `src/agent/tools/grep.rs`
- [x] 迁移 `src/agent/tools/find.rs`
- [x] 迁移 `src/agent/tools/ls.rs`
- [x] 迁移 `src/agent/tools/mod.rs`：`ToolRegistry` 持有 `Arc<dyn XyTool>`
- [x] 迁移 `src/agent/tools/patch.rs`（cancelled — patch tool 已移除）

## Wrapper 迁移

- [x] 迁移 `src/infra/security/mod.rs`：`SecurityToolWrapper` 包装 `XyTool`
- [x] 迁移 `src/infra/skills/mcp.rs`：`McpToolAdapter` 实现 `XyTool`
- [x] 迁移 `src/interface/tui/approval.rs`：`SecureApprovalToolWrapper` 包装 `XyTool`

## Provider 迁移

- [x] 修改 c103 的 `provider/openai.rs`：`impl Llm` → `impl XyModel`
- [x] 修改 c103 的 `provider/anthropic.rs`：`impl Llm` → `impl XyModel`
- [x] 修改 `src/agent/provider/fake.rs`：`impl Llm` → `impl XyModel`

## 兼容层

- [x] 创建 `src/agent/compat.rs`：`XyModelToLlm` adapter (cancelled — 与 c105 合并，直接替换全部 adk 依赖，无需兼容层)
- [x] 创建 `src/agent/compat.rs`：`XyToolToTool` adapter (cancelled — 同上)

## 验证

- [x] `cargo build` 通过
- [x] `cargo test` 通过
- [x] 确认 `src/` 中无任何 `use adk_` 残留（与 c105 合并后全量清除）
- [x] `just qa` 通过
