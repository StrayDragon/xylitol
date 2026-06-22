---
id: c210-enhance-tool-system
title: "Enhance Tool System — allow/deny lists, argument validation, per-tool execution modes, definition wrapper"
depends_on: [c170-refactor-agent-message-types, c185-upgrade-agent-loop]
---

## Why

当前工具系统（`tool-system` spec）定义了基础 `XyTool` trait 和注册表，但缺少 pi 中的以下关键功能：

1. **无工具许可/拒绝名单**：不能在会话级别限制哪些工具可用
2. **无参数验证**：`validateToolArguments()` 缺失——工具调用参数可能在传给执行器前就已损坏
3. **无每个工具的执行模式**：`executionMode: "sequential" | "parallel"` 不应是全局的，每个工具应可独立声明
4. **无工具定义包装器**：`tool-definition-wrapper.ts` 缺失——从 `AgentTool` 到 `ToolDefinition` 的转换
5. **无 `prepareArguments`**：每个工具可选的参数预处理（如 XML→JSON 转换）
6. **无 `prompt_snippet`/`prompt_guidelines` 标准化**：当前 trait 有这些方法但未统一处理

## What Changes

1. **工具许可/拒绝名单**：`allowedToolNames` + `excludedToolNames` → 在构建工具列表时过滤
2. **参数验证**：`validate_tool_arguments(tool, call)` → 根据 JSON Schema 校验参数
3. **每个工具的执行模式**：在 `XyTool` trait 上加 `execution_mode()` 方法
4. **工具定义包装器**：`ToolDefinition` 统一结构体（name, description, parameters, promptGuidelines, sourceInfo）
5. **`prepareArguments`**：可选的参数预处理方法
6. **提示信息标准化**：`prompt_snippet()` 和 `prompt_guidelines()` 标准化为必选（而非可选）

## Capabilities

- tool-system

## Impact

- `src/agent/traits.rs`：更新 XyTool trait
- `src/agent/tools/mod.rs`：更新 ToolRegistry 添加过滤逻辑
- `src/agent/tools/`：每个工具检查 execution_mode 和 prepare_arguments
- `src/agent/session.rs`：传递 allowedToolNames/excludedToolNames
- `src/agent/loop.rs`：执行时检查每个工具的执行模式

## Definition of Done

- [ ] `allowedToolNames` / `excludedToolNames` 过滤可用
- [ ] `validate_tool_arguments()` 实现
- [ ] 每个工具可声明 `execution_mode()`
- [ ] `ToolDefinition` 包装器统一化
- [ ] `prepare_arguments()` 在每个工具调用前调用
- [ ] `cargo test` 通过
