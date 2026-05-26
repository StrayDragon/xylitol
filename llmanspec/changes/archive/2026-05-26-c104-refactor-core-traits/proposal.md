---
depends_on:
  - c103-refactor-provider-layer
---

# c104-refactor-core-traits

## Why

c103 替换了 `adk-model`，但 xylitol 仍依赖 `adk-core` 的 `Llm`、`Tool`、`Content`、
`Part`、`AdkError` 等核心类型。这些类型分散在 21 个源文件中，
是 adk-rust 框架锁定的根源。

定义 xylitol 自有的 trait（`XyModel`、`XyTool`）并用 adapter 桥接，
可以将 adk 依赖收敛到一个薄适配层，为阶段三（彻底去除 adk）铺路。

## What Changes

1. 定义 `src/agent/traits.rs`：`XyModel` trait（generate + stream）和 `XyTool` trait（name + schema + execute）
2. 定义 `src/agent/types.rs`：`XyContent`、`XyPart`、`XyError` 等自有类型
3. 7 个 built-in tool 从 `impl adk_core::Tool` 迁移到 `impl XyTool`
4. `SecurityToolWrapper` 和 `McpToolAdapter` 迁移到 `XyTool`
5. c103 的 provider 从 `impl adk_core::Llm` 迁移到 `impl XyModel`
6. 创建 `src/agent/compat.rs`：`XyModelToLlm` adapter（实现 `adk_core::Llm`），
   让 `adk-runner::Runner` 仍能调用新 provider
7. 从 `Cargo.toml` 移除 `adk-core`（改为仅在 compat 模块通过 `adk-runner` 传递依赖使用）

## Capabilities

- tool-system（Tool trait 定义与 7 个工具实现）
- agent-runtime（Llm trait 与 provider）
- security-policy（SecurityToolWrapper）
- skill-extension（McpToolAdapter）

## Impact

- **中等工作量**：21 个文件的 import 迁移，但大部分是机械替换
- **低风险**：adapter pattern 保证 adk-runner 仍能工作
- **高收益**：xylitol 的核心抽象不再绑定任何外部框架
